mod dialogs;
mod discovery;
mod library;
#[cfg(feature = "native-test")]
mod native_test;
mod onboarding;
mod settings;
mod sync;

use std::{collections::BTreeSet, future::Future, path::PathBuf, sync::Arc};

use gpui_kit::component::{
    ActiveTheme, Disableable,
    button::Button,
    input::{InputEvent, InputState},
};
use gpui_kit::{
    AppContext, Context, DragMoveEvent, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Render, StatefulInteractiveElement, Styled,
    Subscription, Task, Window, div, prelude::FluentBuilder,
};
use skillbinder_client::Client;
use skillbinder_proto::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Discovery,
    Library,
    Sync,
    Settings,
}

#[derive(Clone, PartialEq, Eq)]
enum LibraryFilter {
    All,
    Unfiled,
    Folder(FolderId),
}

struct SidebarDrag(gpui_kit::EntityId);

enum DialogState {
    ImportPlan(ImportPlan),
    DeleteFolder(DeletePreview),
    Conflict {
        slug: String,
        ids: Vec<SkillId>,
        revision: String,
        selected: Option<SkillId>,
    },
    RemoveRoot(RootId),
    None,
}

enum Effect {
    None,
    CloseDialog,
    Grant(DirectoryGrant),
    RootRegistered,
    Plan(ImportPlan),
    Preview(SkillPreview),
    DeletePreview(DeletePreview),
}

/// GPUI owner of the native product journeys and client projections.
pub struct NativeView {
    client: Client,
    snapshot: Arc<AppSnapshot>,
    focus: FocusHandle,
    screen: Screen,
    busy: bool,
    error: Option<AppError>,
    dialog: DialogState,
    grant: Option<DirectoryGrant>,
    editing_root: Option<RootId>,
    selected_candidates: BTreeSet<CandidateId>,
    candidate_page: usize,
    acknowledge_invalid: bool,
    selected_skills: BTreeSet<SkillId>,
    library_filter: LibraryFilter,
    sidebar_width: f32,
    resizing_sidebar: bool,
    selected_skill: Option<SkillId>,
    preview: Option<SkillPreview>,
    search: Entity<InputState>,
    folder_name: Entity<InputState>,
    root_label: Entity<InputState>,
    remote_url: Entity<InputState>,
    remote_branch: Entity<InputState>,
    _search_subscription: Subscription,
    _activation_subscription: Subscription,
    _subscription: Task<()>,
}

impl NativeView {
    /// Creates the native view and subscribes to client projections.
    pub fn new(client: Client, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let snapshot = client.snapshot();
        let sidebar_width = snapshot.preferences.sidebar_width.clamp(192.0, 400.0);
        let activation_subscription = cx.observe_window_activation(window, |view, window, cx| {
            if window.is_window_active() && view.snapshot.sync.remote.is_some() && !view.busy {
                let client = view.client.clone();
                view.run(
                    async move {
                        client
                            .synchronize(SyncAction::Refresh)
                            .await
                            .map(|_| Effect::None)
                    },
                    cx,
                );
            }
        });
        let mut receiver = client.subscribe();
        let subscription = cx.spawn(async move |this, cx| {
            while receiver.changed().await.is_ok() {
                let latest = receiver.borrow_and_update().clone();
                if this
                    .update(cx, |view, cx| view.receive(latest, cx))
                    .is_err()
                {
                    break;
                }
            }
        });
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search skills"));
        let search_subscription = cx.subscribe(&search, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        let folder_name = cx.new(|cx| InputState::new(window, cx).placeholder("Folder name"));
        let root_label = cx.new(|cx| InputState::new(window, cx).placeholder("Project label"));
        let remote_url = cx.new(|cx| InputState::new(window, cx).placeholder("Remote URL"));
        let remote_branch = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Branch")
                .default_value("main")
        });
        if let Some(remote) = &snapshot.sync.remote {
            remote_url.update(cx, |input, cx| {
                input.set_value(remote.url.clone(), window, cx)
            });
            remote_branch.update(cx, |input, cx| {
                input.set_value(remote.branch.clone(), window, cx)
            });
        }
        Self {
            client,
            snapshot,
            focus: cx.focus_handle(),
            screen: Screen::Discovery,
            busy: false,
            error: None,
            dialog: DialogState::None,
            grant: None,
            editing_root: None,
            selected_candidates: BTreeSet::new(),
            candidate_page: 0,
            acknowledge_invalid: false,
            selected_skills: BTreeSet::new(),
            library_filter: LibraryFilter::All,
            sidebar_width,
            resizing_sidebar: false,
            selected_skill: None,
            preview: None,
            search,
            folder_name,
            root_label,
            remote_url,
            remote_branch,
            _search_subscription: search_subscription,
            _activation_subscription: activation_subscription,
            _subscription: subscription,
        }
    }

    fn receive(&mut self, latest: Arc<AppSnapshot>, cx: &mut Context<Self>) {
        let old_scan = self.snapshot.scan.as_ref().map(|scan| &scan.id);
        let new_scan = latest.scan.as_ref().map(|scan| &scan.id);
        if old_scan != new_scan {
            self.selected_candidates.clear();
            self.candidate_page = 0;
            self.acknowledge_invalid = false;
        }
        if let Some(library) = &latest.library {
            if matches!(&self.library_filter, LibraryFilter::Folder(id) if {
                !library
                    .library
                    .folders
                    .iter()
                    .any(|folder| &folder.id == id)
            }) {
                self.library_filter = LibraryFilter::All;
            }
            self.selected_skills
                .retain(|id| library.library.skills.iter().any(|s| &s.id == id));
            if self
                .selected_skill
                .as_ref()
                .is_some_and(|id| !library.library.skills.iter().any(|s| &s.id == id))
            {
                self.selected_skill = None;
                self.preview = None;
            }
        }
        self.snapshot = latest;
        if self.snapshot.preferences.sidebar_width != self.sidebar_width && !self.resizing_sidebar {
            self.sidebar_width = self.snapshot.preferences.sidebar_width.clamp(192.0, 400.0);
        }
        cx.notify();
    }

    fn run<F>(&mut self, operation: F, cx: &mut Context<Self>)
    where
        F: Future<Output = AppResult<Effect>> + Send + 'static,
    {
        if self.busy {
            return;
        }
        self.busy = true;
        self.error = None;
        let client = self.client.clone();
        let handle = client.runtime().spawn(operation);
        cx.spawn(async move |this, cx| {
            let result = handle.await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                match result {
                    Ok(Ok(effect)) => {
                        view.receive(view.client.snapshot(), cx);
                        match effect {
                            Effect::None => {}
                            Effect::CloseDialog => view.dialog = DialogState::None,
                            Effect::Grant(grant) => {
                                view.grant = Some(grant);
                                view.editing_root = None;
                            }
                            Effect::RootRegistered => view.grant = None,
                            Effect::Plan(plan) => view.dialog = DialogState::ImportPlan(plan),
                            Effect::Preview(preview) => view.preview = Some(preview),
                            Effect::DeletePreview(preview) => {
                                view.dialog = DialogState::DeleteFolder(preview)
                            }
                        }
                    }
                    Ok(Err(error)) => view.error = Some(error),
                    Err(_) => view.error = Some(AppError::storage()),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn navigate(&mut self, screen: Screen, window: &mut Window, cx: &mut Context<Self>) {
        if self.screen != screen {
            self.candidate_page = 0;
            self.selected_candidates.clear();
            self.acknowledge_invalid = false;
            self.selected_skills.clear();
            self.selected_skill = None;
            self.preview = None;
            self.search
                .update(cx, |input, cx| input.set_value("", window, cx));
        }
        self.screen = screen;
        self.error = None;
        if screen == Screen::Sync {
            let client = self.client.clone();
            self.run(
                async move {
                    client
                        .synchronize(SyncAction::Refresh)
                        .await
                        .map(|_| Effect::None)
                },
                cx,
            );
        }
        cx.notify();
    }

    fn input_text(input: &Entity<InputState>, cx: &Context<Self>) -> String {
        input.read(cx).value().trim().to_owned()
    }

    fn reveal_logs(&self, cx: &mut Context<Self>) {
        cx.reveal_path(&PathBuf::from(&self.snapshot.bootstrap.logs_path));
    }

    fn persist_sidebar_width(&mut self, cx: &mut Context<Self>) {
        self.resizing_sidebar = false;
        let width = self.sidebar_width;
        if (width - self.snapshot.preferences.sidebar_width).abs() < 0.5 {
            return;
        }
        let client = self.client.clone();
        let handle = client.runtime().spawn(async move {
            client
                .save_preferences(Preferences {
                    sidebar_width: width,
                })
                .await
        });
        cx.spawn(async move |this, cx| {
            let result = handle.await;
            let _ = this.update(cx, |view, cx| {
                match result {
                    Ok(Ok(())) => view.receive(view.client.snapshot(), cx),
                    Ok(Err(error)) => view.error = Some(error),
                    Err(_) => view.error = Some(AppError::storage()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn render_import_outcome(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let mut banner = div();
        if let Some(outcome) = &self.snapshot.import_outcome {
            banner = banner
                .p_4()
                .flex()
                .flex_col()
                .gap_2()
                .child(format!(
                    "Import finished: {} copied, {} existing observations, {} name conflicts",
                    outcome.imported.len(),
                    outcome.attached.len(),
                    outcome.conflicts.len()
                ))
                .children(
                    outcome
                        .conflicts
                        .iter()
                        .map(|slug| div().child(format!("Resolve shared name: {slug} in Library"))),
                )
                .child(
                    Button::new("dismiss-import")
                        .label("Dismiss result")
                        .disabled(self.busy)
                        .on_click(cx.listener(|view, _, _, cx| {
                            let client = view.client.clone();
                            view.run(
                                async move { client.dismiss_import().await.map(|_| Effect::None) },
                                cx,
                            );
                        })),
                );
        }
        banner
    }
}

impl Focusable for NativeView {
    fn focus_handle(&self, _: &gpui_kit::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for NativeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = if self.snapshot.bootstrap.step != OnboardingStep::Complete {
            self.render_onboarding(cx).into_any_element()
        } else {
            let page = match self.screen {
                Screen::Discovery => self.render_discovery(cx).into_any_element(),
                Screen::Library => self.render_library(cx).into_any_element(),
                Screen::Sync => self.render_sync(cx).into_any_element(),
                Screen::Settings => self.render_settings(cx).into_any_element(),
            };
            let outcome = self.render_import_outcome(cx);
            div()
                .id("product-shell")
                .size_full()
                .flex()
                .on_drag_move(
                    cx.listener(|view, event: &DragMoveEvent<SidebarDrag>, _, cx| {
                        if event.drag(cx).0 != cx.entity_id() {
                            return;
                        }
                        view.resizing_sidebar = true;
                        view.sidebar_width =
                            f32::from(event.event.position.x - event.bounds.left())
                                .clamp(192.0, 400.0);
                        cx.notify();
                    }),
                )
                .on_drop(cx.listener(|view, drag: &SidebarDrag, _, cx| {
                    if drag.0 == cx.entity_id() {
                        view.persist_sidebar_width(cx);
                    }
                }))
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|view, _, _, cx| {
                        if view.resizing_sidebar {
                            view.persist_sidebar_width(cx);
                        }
                    }),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(|view, _, _, cx| {
                        if view.resizing_sidebar {
                            view.persist_sidebar_width(cx);
                        }
                    }),
                )
                .child(self.render_sidebar(cx))
                .child(
                    div()
                        .id("main-content")
                        .flex_1()
                        .min_w_0()
                        .overflow_y_scroll()
                        .p_6()
                        .child(outcome)
                        .child(page),
                )
                .into_any_element()
        };
        self.sync_dialog(window, cx);
        let colors = cx.theme().colors;
        let global_error = self
            .error
            .as_ref()
            .or(self.snapshot.background_error.as_ref());
        div()
            .track_focus(&self.focus)
            .size_full()
            .font_family(skillbinder_theme::system_font())
            .bg(colors.background)
            .text_color(colors.foreground)
            .child(body)
            .when_some(global_error, |root, error| {
                root.child(
                    div()
                        .absolute()
                        .bottom_4()
                        .right_4()
                        .max_w_96()
                        .p_4()
                        .bg(colors.secondary)
                        .border_1()
                        .border_color(colors.border)
                        .child(format!(
                            "{} {} (Diagnostic {})",
                            error.message, error.recovery, error.diagnostic_id
                        )),
                )
            })
    }
}
