mod controls;
mod dialogs;
mod discovery;
mod library;
#[cfg(feature = "native-test")]
mod native_test;
mod onboarding;
mod project_dialogs;
mod route;
mod settings;
mod shell;
mod sidebar;
mod sidebar_folders;
mod sidebar_projects;
mod sidebar_reorder;
mod sidebar_row;
mod status_bar;
mod sync;
mod target_dialogs;
mod target_drift;
mod target_page;

pub use shell::{GoBack, GoForward, ToggleSidebar};

use std::{collections::BTreeSet, future::Future, path::PathBuf, sync::Arc};

use controls::{Inactive, button};
use gpui_kit::component::{
    ActiveTheme,
    input::{InputEvent, InputState},
};
use gpui_kit::{
    AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Subscription, Task, Window, div, prelude::FluentBuilder,
};
use route::{History, LibraryFilter, Route};
use sidebar_folders::FolderEdit;
use sidebar_reorder::RowReorder;
use skillbinder_client::Client;
use skillbinder_proto::*;

enum DialogState {
    ImportPlan(ImportPlan),
    DeleteFolder(DeletePreview),
    Conflict {
        slug: String,
        ids: Vec<SkillId>,
        revision: String,
        selected: Option<SkillId>,
    },
    /// Scan progress and outcome of the project being added.
    AddProject,
    RemoveProject(ProjectId),
    /// Library skills, and agents when the target has none yet, chosen to add to a target.
    AddSkills {
        target: DeploymentTarget,
        skills: BTreeSet<SkillId>,
        agents: Option<BTreeSet<String>>,
    },
    /// Confirms discarding copies that changed since they were deployed.
    RemoveChanged {
        target: DeploymentTarget,
        skill: SkillId,
        paths: Vec<String>,
    },
    /// Confirms discarding the changed copies of the deployment as `shown` by writing the
    /// library version over them.
    DiscardChanges {
        shown: Deployment,
        drift: Drift,
    },
    None,
}

enum Effect {
    None,
    CloseDialog,
    Plan(ImportPlan),
    Preview(SkillPreview),
    DeletePreview(DeletePreview),
    Dialog(DialogState),
}

/// GPUI owner of the native product journeys and client projections.
pub struct NativeView {
    client: Client,
    snapshot: Arc<AppSnapshot>,
    focus: FocusHandle,
    route: Route,
    history: History,
    busy: bool,
    /// A Git synchronization is in flight; implies `busy`.
    syncing: bool,
    error: Option<AppError>,
    dialog: DialogState,
    /// The addition whose modal the user closed, until the engine drops it.
    dismissed_addition: Option<ScanId>,
    selected_candidates: BTreeSet<CandidateId>,
    candidate_page: usize,
    acknowledge_invalid: bool,
    selected_skills: BTreeSet<SkillId>,
    /// Inline folder name editing in the sidebar.
    folder_edit: Option<FolderEdit>,
    folder_reorder: RowReorder<FolderId>,
    /// Target agents being edited on the current target page.
    agent_edit: Option<BTreeSet<String>>,
    /// Inline project renaming in the sidebar.
    project_edit: Option<ProjectId>,
    project_reorder: RowReorder<ProjectId>,
    sidebar_width: f32,
    sidebar_collapsed: bool,
    appearance: Appearance,
    /// Sidebar toggle count; zero renders without motion, so launch does not animate.
    sidebar_motion: u64,
    /// Collapsed sidebar still mounted for its closing motion.
    sidebar_closing: bool,
    resizing_sidebar: bool,
    preference_writes: shell::PreferenceWrites,
    selected_skill: Option<SkillId>,
    preview: Option<SkillPreview>,
    search: Entity<InputState>,
    folder_name: Entity<InputState>,
    project_name: Entity<InputState>,
    remote_url: Entity<InputState>,
    remote_branch: Entity<InputState>,
    _search_subscription: Subscription,
    _folder_name_subscription: Subscription,
    _project_name_subscription: Subscription,
    _activation_subscription: Subscription,
    _subscription: Task<()>,
}

impl NativeView {
    /// Creates the native view and subscribes to client projections.
    pub fn new(client: Client, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let snapshot = client.snapshot();
        let sidebar_width = sidebar::clamp_sidebar_width(snapshot.preferences.sidebar_width);
        let sidebar_collapsed = snapshot.preferences.sidebar_collapsed;
        let appearance = snapshot.preferences.appearance;
        status_bar::apply_appearance(appearance, window, cx);
        let activation_subscription = cx.observe_window_activation(window, |view, window, cx| {
            if window.is_window_active() && view.snapshot.sync.remote.is_some() {
                view.synchronize(SyncAction::Refresh, cx);
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
        let folder_name_subscription =
            cx.subscribe_in(&folder_name, window, Self::folder_name_event);
        let project_name = cx.new(|cx| InputState::new(window, cx).placeholder("Project name"));
        let project_name_subscription =
            cx.subscribe_in(&project_name, window, Self::project_name_event);
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
        // Window-level shortcuts such as ToggleSidebar dispatch through the focused view.
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self {
            client,
            snapshot,
            focus,
            route: Route::Discovery,
            history: History::default(),
            busy: false,
            syncing: false,
            error: None,
            dialog: DialogState::None,
            dismissed_addition: None,
            selected_candidates: BTreeSet::new(),
            candidate_page: 0,
            acknowledge_invalid: false,
            selected_skills: BTreeSet::new(),
            folder_edit: None,
            folder_reorder: RowReorder::default(),
            agent_edit: None,
            project_edit: None,
            project_reorder: RowReorder::default(),
            sidebar_width,
            sidebar_collapsed,
            appearance,
            sidebar_motion: 0,
            sidebar_closing: false,
            resizing_sidebar: false,
            preference_writes: shell::PreferenceWrites::default(),
            selected_skill: None,
            preview: None,
            search,
            folder_name,
            project_name,
            remote_url,
            remote_branch,
            _search_subscription: search_subscription,
            _folder_name_subscription: folder_name_subscription,
            _project_name_subscription: project_name_subscription,
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
            let exists = |id: &FolderId| library.library.folders.iter().any(|f| &f.id == id);
            if matches!(&self.route, Route::Library(LibraryFilter::Folder(id)) if !exists(id)) {
                self.route = Route::Library(LibraryFilter::All);
            }
            if matches!(&self.folder_edit, Some(FolderEdit::Rename(id)) if !exists(id)) {
                self.folder_edit = None;
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
        let project_exists = |id: &ProjectId| latest.projects.iter().any(|p| &p.id == id);
        if matches!(&self.route, Route::Target(DeploymentTarget::Project(id)) if !project_exists(id))
        {
            self.route = Route::Discovery;
            self.agent_edit = None;
        }
        if self
            .project_edit
            .as_ref()
            .is_some_and(|id| !project_exists(id))
        {
            self.project_edit = None;
        }
        self.snapshot = latest;
        self.sync_addition_dialog();
        if self.snapshot.preferences.sidebar_width != self.sidebar_width
            && !self.resizing_sidebar
            && !self.preference_writes.is_busy()
        {
            self.sidebar_width =
                sidebar::clamp_sidebar_width(self.snapshot.preferences.sidebar_width);
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
                view.syncing = false;
                view.folder_reorder.settle();
                view.project_reorder.settle();
                match result {
                    Ok(Ok(effect)) => {
                        view.receive(view.client.snapshot(), cx);
                        match effect {
                            Effect::None => {}
                            Effect::CloseDialog => view.dialog = DialogState::None,
                            Effect::Plan(plan) => view.dialog = DialogState::ImportPlan(plan),
                            Effect::Preview(preview) => view.preview = Some(preview),
                            Effect::DeletePreview(preview) => {
                                view.dialog = DialogState::DeleteFolder(preview)
                            }
                            Effect::Dialog(dialog) => view.dialog = dialog,
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

    /// Opens `route` and records the current one for Back.
    fn navigate(&mut self, route: Route, window: &mut Window, cx: &mut Context<Self>) {
        if self.route != route {
            self.history.push(self.route.clone());
        }
        self.show(route, window, cx);
    }

    fn go_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(route) = self.history.back(self.route.clone()) {
            self.show(route, window, cx);
        }
    }

    fn go_forward(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(route) = self.history.forward(self.route.clone()) {
            self.show(route, window, cx);
        }
    }

    /// Shows `route` without touching history; a deleted folder falls back to all skills, and
    /// a removed project to Discovery.
    fn show(&mut self, route: Route, window: &mut Window, cx: &mut Context<Self>) {
        let folder_exists = |id: &FolderId| {
            self.snapshot
                .library
                .as_ref()
                .is_some_and(|library| library.library.folders.iter().any(|f| &f.id == id))
        };
        let project_exists = |id: &ProjectId| self.snapshot.projects.iter().any(|p| &p.id == id);
        let route = match route {
            Route::Library(LibraryFilter::Folder(id)) if !folder_exists(&id) => {
                Route::Library(LibraryFilter::All)
            }
            Route::Target(DeploymentTarget::Project(id)) if !project_exists(&id) => {
                Route::Discovery
            }
            route => route,
        };
        if self.route != route {
            self.candidate_page = 0;
            self.selected_candidates.clear();
            self.acknowledge_invalid = false;
            self.selected_skills.clear();
            self.selected_skill = None;
            self.preview = None;
            self.agent_edit = None;
            self.search
                .update(cx, |input, cx| input.set_value("", window, cx));
        }
        let sync = route == Route::Sync;
        self.route = route;
        self.error = None;
        if sync {
            self.synchronize(SyncAction::Refresh, cx);
        }
        cx.notify();
    }

    /// Library filter of the current route; other routes behave as all skills.
    fn library_filter(&self) -> &LibraryFilter {
        match &self.route {
            Route::Library(filter) => filter,
            _ => &LibraryFilter::All,
        }
    }

    fn input_text(input: &Entity<InputState>, cx: &Context<Self>) -> String {
        input.read(cx).value().trim().to_owned()
    }

    fn reveal_logs(&self, cx: &mut Context<Self>) {
        cx.reveal_path(&PathBuf::from(&self.snapshot.bootstrap.logs_path));
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
                    "Import finished: {} copied, {} already in library, {} name conflicts",
                    outcome.imported.len(),
                    outcome.existing.len(),
                    outcome.conflicts.len()
                ))
                .children(
                    outcome
                        .conflicts
                        .iter()
                        .map(|slug| div().child(format!("Resolve shared name: {slug} in Library"))),
                )
                .child(
                    button("dismiss-import")
                        .label("Dismiss result")
                        .inactive(self.busy)
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
        let onboarding = self.snapshot.bootstrap.step != OnboardingStep::Complete;
        let body = if onboarding {
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(self.render_header(false, window, cx))
                .child(div().flex_1().min_h_0().child(self.render_onboarding(cx)))
                .into_any_element()
        } else {
            let page = match &self.route {
                Route::Discovery => self.render_discovery(cx).into_any_element(),
                Route::Library(_) => self.render_library(cx).into_any_element(),
                Route::Target(target) => self.render_target(&target.clone(), cx),
                Route::Sync => self.render_sync(cx).into_any_element(),
                Route::Settings => self.render_settings(cx).into_any_element(),
            };
            self.render_shell(page, window, cx)
        };
        self.sync_dialog(window, cx);
        let colors = cx.theme().colors;
        let global_error = self
            .error
            .as_ref()
            .or(self.snapshot.background_error.as_ref());
        div()
            .track_focus(&self.focus)
            .on_action(cx.listener(|view, _: &ToggleSidebar, _, cx| view.toggle_sidebar(cx)))
            .when(!onboarding, |root| {
                root.on_action(cx.listener(|view, _: &GoBack, window, cx| view.go_back(window, cx)))
                    .on_action(
                        cx.listener(|view, _: &GoForward, window, cx| view.go_forward(window, cx)),
                    )
            })
            .size_full()
            .font_family(skillbinder_theme::system_font())
            .bg(colors.background)
            .text_color(colors.foreground)
            .child(body)
            .when_some(global_error, |root, error| {
                root.child(
                    div()
                        .absolute()
                        // Clears the status bar, which only the product shell shows.
                        .bottom(gpui_kit::px(if onboarding {
                            16.0
                        } else {
                            skillbinder_theme::STATUS_BAR_HEIGHT + 16.0
                        }))
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
