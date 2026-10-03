//! Window-owned controller: schedules application work and publishes presentation state.
use crate::state::*;
use gpui::{prelude::*, *};
use gpui_component::input::InputState;
use skillbinder_app::{AppState, actions, models::*};
use std::{collections::BTreeMap, sync::Arc, time::Duration};

pub struct Workspace {
    pub(crate) service: Arc<AppState>,
    pub(crate) state: WorkspaceState,
    pub(crate) remote: Entity<InputState>,
    pub(crate) branch: Entity<InputState>,
    pub(crate) root_labels: BTreeMap<String, Entity<InputState>>,
    scan_poll: Option<Task<()>>,
    _appearance: Subscription,
}

impl Workspace {
    pub fn new(service: Arc<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let remote = cx
            .new(|cx| InputState::new(window, cx).placeholder("https://github.com/you/skills.git"));
        let branch = cx.new(|cx| {
            let mut input = InputState::new(window, cx);
            input.set_value("main", window, cx);
            input
        });
        let appearance = cx.observe_window_appearance(window, |_, window, cx| {
            crate::theme::sync(window, cx);
            cx.notify();
        });
        let mut view = Self {
            service,
            state: WorkspaceState::default(),
            remote,
            branch,
            root_labels: BTreeMap::new(),
            scan_poll: None,
            _appearance: appearance,
        };
        view.load_bootstrap(window, cx);
        view
    }

    /// Every service call runs on the background executor. The retained weak entity means a
    /// completed job never updates a closed window. Keys prevent duplicate submissions.
    pub(crate) fn call<T: Send + 'static>(
        &mut self,
        key: Operation,
        work: impl FnOnce(Arc<AppState>) -> Result<T, AppError> + Send + 'static,
        done: impl FnOnce(&mut Self, T, &mut Window, &mut Context<Self>) + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.state.pending.insert(key) {
            if key.is_read() {
                self.state.refresh_again.insert(key);
            }
            return;
        }
        self.state.errors.remove(&key);
        let service = Arc::clone(&self.service);
        let task = cx.background_executor().spawn(async move { work(service) });
        cx.spawn_in(window, async move |view, cx| {
            let result = task.await;
            let _ = view.update_in(cx, |view, window, cx| {
                view.state.pending.remove(&key);
                match result {
                    Ok(value) => done(view, value, window, cx),
                    Err(error) => {
                        if error.diagnostic_id == "discovery-unknown" {
                            view.state.scan = None;
                            view.state.selection = Selection::default();
                        }
                        view.state.errors.insert(key, error);
                    }
                }
                if view.state.refresh_again.remove(&key) && !view.state.pending.contains(&key) {
                    view.refresh(key, window, cx);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(crate) fn busy(&self) -> bool {
        self.state.pending.iter().any(|key| !key.is_read())
    }

    pub(crate) fn refresh(&mut self, key: Operation, window: &mut Window, cx: &mut Context<Self>) {
        match key {
            Operation::Bootstrap | Operation::ScanCurrent => self.load_bootstrap(window, cx),
            Operation::Roots => self.load_roots(window, cx),
            Operation::Library => self.load_library(window, cx),
            Operation::GitStatus => self.load_git(window, cx),
            Operation::ScanResults => self.load_scan(window, cx),
            _ => {}
        }
    }

    pub(crate) fn load_bootstrap(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.call(
            Operation::Bootstrap,
            |s| actions::bootstrap::system_bootstrap(&s).into_result(),
            |view, data, w, cx| {
                let ready = data.onboarding.completed && data.library_state == LibraryState::Ready;
                view.state.bootstrap = Some(data);
                if ready {
                    view.load_roots(w, cx);
                    view.load_library(w, cx);
                    view.load_git(w, cx);
                    view.call(
                        Operation::ScanCurrent,
                        |s| actions::discovery::discovery_current(&s).into_result(),
                        |view, current, w, cx| {
                            if let Some(id) = current.scan_id {
                                view.state.selection.adopt(id);
                                view.load_scan(w, cx);
                            }
                        },
                        w,
                        cx,
                    );
                }
            },
            window,
            cx,
        );
    }

    pub(crate) fn load_roots(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.call(
            Operation::Roots,
            |s| actions::roots::roots_list(&s).into_result(),
            |view, data, _, _| {
                view.root_labels
                    .retain(|id, _| data.roots.iter().any(|root| &root.root_id == id));
                view.state.roots = Some(data);
            },
            window,
            cx,
        );
    }
    pub(crate) fn load_library(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.call(
            Operation::Library,
            |s| actions::library::library_list(&s).into_result(),
            |view, data, _, _| {
                view.state.library = Some(data);
            },
            window,
            cx,
        );
    }
    pub(crate) fn load_git(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.call(
            Operation::GitStatus,
            |s| actions::git_sync::git_sync_status(&s).into_result(),
            |view, data, _, _| {
                view.state.git = Some(data);
            },
            window,
            cx,
        );
    }
    pub(crate) fn load_scan(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(scan_id) = self.state.selection.scan_id.clone() else {
            return;
        };
        let request = DiscoveryResultsRequest {
            scan_id,
            offset: self.state.selection.offset,
            limit: PAGE_SIZE,
        };
        self.call(
            Operation::ScanResults,
            move |s| actions::discovery::discovery_results(&s, request).into_result(),
            |view, data, window, cx| {
                if !view.state.accept_scan(data) {
                    // Navigation can supersede an in-flight request even after the scan ends.
                    view.load_scan(window, cx);
                    return;
                }
                view.poll_running_scan(window, cx);
            },
            window,
            cx,
        );
    }

    pub(crate) fn navigate(&mut self, screen: Screen, window: &mut Window, cx: &mut Context<Self>) {
        self.state.screen = screen;
        match screen {
            Screen::Discovery => {
                self.load_roots(window, cx);
                self.load_scan(window, cx);
            }
            Screen::Library => self.load_library(window, cx),
            Screen::Git => self.load_git(window, cx),
            Screen::Settings => self.load_roots(window, cx),
        }
        cx.notify();
    }

    fn poll_running_scan(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.scan_poll = None;
        if !self
            .state
            .scan
            .as_ref()
            .is_some_and(|scan| scan.phase == ScanPhase::Running)
        {
            return;
        }
        // Poll only while discovery runs. Dropping the window cancels the retained timer.
        let timer = cx.background_executor().timer(Duration::from_millis(400));
        self.scan_poll = Some(cx.spawn_in(window, async move |view, cx| {
            timer.await;
            let _ = view.update_in(cx, |view, window, cx| view.load_scan(window, cx));
        }));
    }
}
