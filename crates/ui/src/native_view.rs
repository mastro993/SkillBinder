mod agent_page;
mod agent_skills;
mod breadcrumb;
mod controls;
mod custom_color;
mod dialogs;
mod discovery;
mod discovery_versions;
mod folder_reorder;
mod hover_scroll;
mod library;
mod library_dates;
mod library_drift;
mod library_folders;
mod library_items;
mod library_presence;
mod library_updates;
mod menu;
#[cfg(feature = "native-test")]
mod native_test;
mod navigation;
mod onboarding;
mod preset_activation;
mod preset_colors;
mod preset_icons;
mod route;
mod settings;
mod shell;
mod sidebar;
mod sidebar_agents;
mod sidebar_fold;
mod sidebar_presets;
mod sidebar_reorder;
mod sidebar_row;
mod sidebar_scroll;
mod sidebar_workspaces;
mod skill_page;
mod sync;
mod updates;
mod workspace_dialogs;
mod workspace_page;

pub use shell::{GoBack, GoForward, ToggleSidebar};

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
};

use controls::{Inactive, button};
use gpui_kit::component::{
    ActiveTheme,
    input::{InputEvent, InputState},
};
use gpui_kit::{
    AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Subscription, Task, Window, div, prelude::FluentBuilder,
};
use library_folders::FolderEdit;
use route::{History, LibraryFilter, Route};
use sidebar_presets::PresetEdit;
use sidebar_reorder::RowReorder;
use skillbinder_client::Client;
use skillbinder_proto::*;

enum DialogState {
    ImportPlan(ImportPlan),
    DeletePreset(DeletePreview),
    DeleteFolder { id: FolderId, revision: String },
    RemoveWorkspace(WorkspaceId),
    None,
}

enum Effect {
    None,
    CloseDialog,
    Plan(ImportPlan),
    DeletePreview(DeletePreview),
    Updated(UpdateOutcome),
    /// A skill was reinstalled, so its page shows its restored content.
    Reinstalled(SkillId),
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
    /// The keys of the found skills chosen for import.
    selected_skills: BTreeSet<String>,
    /// The version the user chose for a found skill, by key, instead of its preferred one.
    version_choices: BTreeMap<String, CandidateId>,
    /// The keys of the found skills whose versions Discovery lists.
    expanded_skills: BTreeSet<String>,
    acknowledge_invalid: bool,
    /// The open context menu.
    menu: Option<menu::OpenMenu>,
    /// Inline preset name editing in the sidebar.
    preset_edit: Option<PresetEdit>,
    preset_reorder: RowReorder<PresetId>,
    /// The library folder chosen in the header; none is All skills. Not kept across restarts.
    library_folder: Option<FolderId>,
    /// Inline folder naming in the header.
    folder_edit: Option<FolderEdit>,
    folder_reorder: RowReorder<FolderId>,
    /// The name of a created folder being saved, shown as its tab until then.
    creating_folder: Option<String>,
    /// Inline workspace renaming in the sidebar.
    workspace_edit: Option<WorkspaceId>,
    workspace_reorder: RowReorder<WorkspaceId>,
    sidebar_width: f32,
    sidebar_collapsed: bool,
    folded_sections: BTreeSet<SidebarSection>,
    /// Fold toggle count per section, which keys its slide.
    section_motion: BTreeMap<SidebarSection, u64>,
    /// Sections whose rows are sliding open or shut.
    sections_moving: BTreeSet<SidebarSection>,
    /// Scroll position of the sidebar's preset, workspace, and agent sections.
    sidebar_scroll: hover_scroll::HoverScroll,
    /// Scroll position of the page, whose scrollbar shows while the pointer is over it.
    content_scroll: hover_scroll::HoverScroll,
    appearance: Appearance,
    library_view: LibraryView,
    /// The preset whose skills activation toggles edit; absent starts with Default.
    active_preset: Option<PresetId>,
    /// Whether Library lists only skills with an update; reset with search when the route changes.
    needs_update_only: bool,
    /// Sidebar toggle count; zero renders without motion, so launch does not animate.
    sidebar_motion: u64,
    /// Collapsed sidebar still mounted for its closing motion.
    sidebar_closing: bool,
    resizing_sidebar: bool,
    preference_writes: shell::PreferenceWrites,
    preview: Option<SkillPreview>,
    /// The last Update or Update all result, shown until the page changes.
    update_outcome: Option<UpdateOutcome>,
    /// The skill page's file read in flight; dropping it discards the read's result.
    skill_load: Option<Task<()>>,
    search: Entity<InputState>,
    preset_name: Entity<InputState>,
    folder_name: Entity<InputState>,
    workspace_name: Entity<InputState>,
    remote_url: Entity<InputState>,
    remote_branch: Entity<InputState>,
    _search_subscription: Subscription,
    _preset_name_subscription: Subscription,
    _folder_name_subscription: Subscription,
    _workspace_name_subscription: Subscription,
    /// The window lost focus since the app last refreshed.
    left_focus: bool,
    _activation_subscription: Subscription,
    _subscription: Task<()>,
}

impl NativeView {
    /// Creates the native view and subscribes to client projections.
    pub fn new(client: Client, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let snapshot = client.snapshot();
        let sidebar_width = sidebar::clamp_sidebar_width(snapshot.preferences.sidebar_width);
        let sidebar_collapsed = snapshot.preferences.sidebar_collapsed;
        let folded_sections = snapshot.preferences.folded_sections.clone();
        let appearance = snapshot.preferences.appearance;
        let library_view = snapshot.preferences.library_view;
        let active_preset = snapshot.preferences.active_preset.clone();
        sidebar::apply_appearance(appearance, window, cx);
        // Launch refreshes through the client; afterwards only regaining focus refreshes.
        let activation_subscription = cx.observe_window_activation(window, |view, window, _| {
            if !window.is_window_active() {
                view.left_focus = true;
            } else if std::mem::take(&mut view.left_focus) {
                view.client.refresh_app();
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
        let preset_name = cx.new(|cx| InputState::new(window, cx).placeholder("Preset name"));
        let preset_name_subscription =
            cx.subscribe_in(&preset_name, window, Self::preset_name_event);
        let folder_name = cx.new(|cx| InputState::new(window, cx).placeholder("Folder name"));
        let folder_name_subscription =
            cx.subscribe_in(&folder_name, window, Self::folder_name_event);
        let workspace_name = cx.new(|cx| InputState::new(window, cx).placeholder("Workspace name"));
        let workspace_name_subscription =
            cx.subscribe_in(&workspace_name, window, Self::workspace_name_event);
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
            selected_skills: BTreeSet::new(),
            version_choices: BTreeMap::new(),
            expanded_skills: BTreeSet::new(),
            acknowledge_invalid: false,
            menu: None,
            preset_edit: None,
            preset_reorder: RowReorder::default().pinned(1),
            library_folder: None,
            folder_edit: None,
            creating_folder: None,
            workspace_edit: None,
            workspace_reorder: RowReorder::default(),
            folder_reorder: RowReorder::measured(folder_reorder::TAB_GAP),
            sidebar_width,
            sidebar_collapsed,
            folded_sections,
            section_motion: BTreeMap::new(),
            sections_moving: BTreeSet::new(),
            sidebar_scroll: hover_scroll::HoverScroll::default(),
            content_scroll: hover_scroll::HoverScroll::default(),
            appearance,
            library_view,
            active_preset,
            needs_update_only: false,
            sidebar_motion: 0,
            sidebar_closing: false,
            resizing_sidebar: false,
            preference_writes: shell::PreferenceWrites::default(),
            preview: None,
            update_outcome: None,
            skill_load: None,
            search,
            preset_name,
            folder_name,
            workspace_name,
            remote_url,
            remote_branch,
            _search_subscription: search_subscription,
            _preset_name_subscription: preset_name_subscription,
            _folder_name_subscription: folder_name_subscription,
            _workspace_name_subscription: workspace_name_subscription,
            left_focus: false,
            _activation_subscription: activation_subscription,
            _subscription: subscription,
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
                    "Import finished: {} copied, {} already in library",
                    outcome.imported.len(),
                    outcome.existing.len(),
                ))
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
                Route::Library(_) => self.render_library(window, cx),
                Route::Skill(id) => self.render_skill(&id.clone(), cx),
                Route::Workspace(id) => self.render_workspace(&id.clone()),
                Route::Agent(id) => self.render_agent(&id.clone(), cx),
                Route::Universal => self.render_universal(cx),
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
            .children(self.menu.as_ref().map(menu::OpenMenu::render))
    }
}
