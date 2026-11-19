mod agent_page;
mod agent_skills;
mod breadcrumb;
mod controls;
mod copies_review;
mod copy_changed;
mod copy_dialogs;
mod custom_color;
mod dialogs;
mod explore;
mod folder_reorder;
mod git_setup;
mod hover_scroll;
mod import_dialog;
mod import_review;
mod library;
mod library_copies;
mod library_dates;
mod library_drift;
mod library_filters;
mod library_folders;
mod library_items;
mod library_places;
mod library_reveal;
mod library_targets;
mod library_updates;
mod library_validation;
#[cfg(feature = "native-test")]
mod native_test;
mod navigation;
mod new_agent_notice;
mod onboarding;
mod page_head;
mod place_popover;
mod preset_activation;
mod preset_colors;
mod preset_dialogs;
mod preset_icons;
mod progress_strip;
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
mod skill_picker;
mod snapshot_query;
mod sync;
mod toasts;
mod undo;
mod updates;
mod without_git;
mod workspace_dialogs;
mod workspace_icons;
mod workspace_page;
mod workspace_targets;

#[cfg(feature = "native-test")]
pub use native_test::StripHold;
pub use shell::{GoBack, GoForward, ToggleSidebar};

/// Binds the keys the view's own controls take before the window does: Tab and Shift-Tab in the
/// Library popover.
pub(crate) fn bind_keys(cx: &mut gpui_kit::App) {
    cx.bind_keys(place_popover::key_bindings());
}

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
};

use gpui_kit::component::{
    ActiveTheme,
    input::{InputEvent, InputState},
};
use gpui_kit::{
    AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Subscription, Task, Window, div, prelude::FluentBuilder, px,
};
use library_folders::FolderEdit;
use route::{History, Route};
use sidebar_presets::PresetEdit;
use sidebar_reorder::RowReorder;
use skillbinder_client::Client;
use skillbinder_design::tokens::space;
use skillbinder_design::{Notice, Status};
use skillbinder_proto::*;

enum DialogState {
    ImportSkills(Box<import_dialog::ImportDialog>),
    /// Onboarding's Review list: the Import skills dialog over the import step's own list, which
    /// outlives it (O7).
    FirstImport,
    DeletePreset(DeletePreview),
    /// The copies of one skill that are not applied, every copy that asks the user, or the
    /// review of the copies that need the user after a blocking change.
    Copies(copy_dialogs::CopiesListing),
    DeleteFolder {
        id: FolderId,
        revision: String,
    },
    RemoveWorkspace(WorkspaceId),
    AddSkill(Box<skill_picker::SkillPicker>),
    /// A catalog skill's detail, opened from Explore.
    SkillDetail(Box<explore::Detail>),
    AddRepository(Box<explore::AddRepository>),
    RemoveRepository(Box<Repository>),
    /// Unticking a skill's last folder on a workspace's page asks before removing it there.
    RemoveFromWorkspace {
        workspace: WorkspaceId,
        skill: SkillId,
    },
    None,
}

enum Effect {
    None,
    CloseDialog,
    DeletePreview(DeletePreview),
    Updated(UpdateOutcome),
    /// A skill was reinstalled, so its page shows its restored content.
    Reinstalled(SkillId),
    /// A blocking change landed, so the progress strip ends.
    Activated(progress_strip::Landing),
    /// A skill was removed from one place, so the toast offers Undo.
    Removed(undo::Removal),
    /// A repository was removed, so the toast offers Undo.
    RepositoryRemoved(Box<Repository>),
    /// Choose your agents saved its choice, so onboarding moves on to an import that scans again.
    AgentsChosen,
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
    /// Counts dialogs opened; a closing one's layer builds nothing once another opens.
    dialogs_opened: u64,
    /// The Import skills dialog while its import runs, reopened if the list changed (D11).
    importing: Option<Box<import_dialog::ImportDialog>>,
    /// Onboarding's import step, once it shows.
    first_import: Option<import_dialog::ImportDialog>,
    /// The onboarding step Back returned to, shown while it comes before the first step not done.
    onboarding_back_to: Option<OnboardingStep>,
    /// The global skill directories Choose your agents shows unticked, from the saved choice once
    /// it shows, until its Continue saves them.
    unticked_agents: Option<BTreeSet<String>>,
    /// The global skill directories turned off in Settings this session, whose rows read "Its
    /// skills stay in place." while off.
    agents_turned_off: BTreeSet<String>,
    agent_notice: new_agent_notice::Notice,
    /// Where the Git step showed when Git was found, kept while "Git <version> is ready" shows.
    git_ready: Option<onboarding::Place>,
    /// What installing Git waits on, in the Git step or Settings.
    git_setup: git_setup::GitSetup,
    /// The app checks for Git every 3 seconds.
    git_checking: bool,
    /// The open context menu.
    menu: Option<skillbinder_design::OpenMenu>,
    /// Inline preset name editing in the sidebar.
    preset_edit: Option<PresetEdit>,
    preset_reorder: RowReorder<PresetId>,
    /// Explore's search and the creator it is narrowed to.
    explore: explore::Explore,
    /// The library folder chosen in the header; none is All skills. Not kept across restarts.
    library_folder: Option<FolderId>,
    /// Library's Active and Updates filters; like the folder, not kept across restarts.
    library_filter: snapshot_query::LibraryFilter,
    /// The skill Library scrolls to once it lays out, after Show in Library.
    library_reveal: Option<SkillId>,
    /// The Library skill whose popover, or the workspace page's skill whose agents menu, is open.
    place_popover: Option<place_popover::PlacePopover>,
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
    /// Scroll position of the page, or of onboarding's import review, whose scrollbar shows while
    /// the pointer is over it.
    content_scroll: hover_scroll::HoverScroll,
    appearance: Appearance,
    library_view: LibraryView,
    /// A preset clicked while its activation is saved, shown active until the engine answers.
    active_preset: Option<PresetId>,
    /// The copies that asked the user in a dialog already, by directory, skill, and state.
    asked_copies: Vec<(String, SkillId, CopyState)>,
    /// A preset name shown while its rename is being saved; cleared on success or failure.
    optimistic_preset_name: Option<(PresetId, String)>,
    /// Created or cloned presets shown before the engine's snapshot arrives.
    optimistic_presets: Vec<sidebar_presets::OptimisticPreset>,
    /// How many skills an update run has finished of its total, while it runs.
    updating: Option<(usize, usize)>,
    /// The progress strip of a blocking change, from the click until it ends.
    strip: Option<progress_strip::Strip>,
    /// When the wash of a strip that ended began fading out, while it fades.
    wash_fade: Option<std::time::Instant>,
    /// Focus for the strip while it is up, so keys reach nothing else.
    overlay_focus: FocusHandle,
    /// A toast waiting for the next frame, where the window it shows in is at hand.
    toast: toasts::PendingToast,
    #[cfg(feature = "native-test")]
    holds: native_test::Holds,
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
    _focus_lost_subscription: Subscription,
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
        sidebar::apply_appearance(appearance, window, cx);
        // Launch refreshes through the client; afterwards only regaining focus refreshes.
        let activation_subscription = cx.observe_window_activation(window, |view, window, _| {
            if !window.is_window_active() {
                view.left_focus = true;
            } else if std::mem::take(&mut view.left_focus) {
                view.client.refresh_app();
                // Before the library exists the app refresh does not look for Git.
                if view.checks_git() {
                    view.client.check_git();
                }
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
        // Window-level shortcuts such as ToggleSidebar dispatch through the focused view, so it
        // takes focus back whenever the focused element leaves the window, such as an open
        // popover on leaving the page.
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let focus_lost_subscription =
            cx.on_focus_lost(window, |view, window, cx| window.focus(&view.focus, cx));
        Self {
            client,
            snapshot,
            focus,
            route: Route::default(),
            history: History::default(),
            busy: false,
            syncing: false,
            error: None,
            dialog: DialogState::None,
            dialogs_opened: 0,
            importing: None,
            first_import: None,
            onboarding_back_to: None,
            unticked_agents: None,
            agents_turned_off: BTreeSet::new(),
            agent_notice: new_agent_notice::Notice::default(),
            git_ready: None,
            git_setup: git_setup::GitSetup::default(),
            git_checking: false,
            menu: None,
            preset_edit: None,
            preset_reorder: RowReorder::default().pinned(1),
            explore: explore::Explore::new(window, cx),
            library_folder: None,
            library_filter: snapshot_query::LibraryFilter::default(),
            library_reveal: None,
            place_popover: None,
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
            active_preset: None,
            asked_copies: Vec::new(),
            optimistic_preset_name: None,
            optimistic_presets: Vec::new(),
            updating: None,
            strip: None,
            wash_fade: None,
            overlay_focus: cx.focus_handle().tab_stop(true),
            toast: toasts::PendingToast::default(),
            #[cfg(feature = "native-test")]
            holds: native_test::Holds::default(),
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
            _focus_lost_subscription: focus_lost_subscription,
            _subscription: subscription,
        }
    }

    fn input_text(input: &Entity<InputState>, cx: &Context<Self>) -> String {
        input.read(cx).value().trim().to_owned()
    }

    fn reveal_logs(&self, cx: &mut Context<Self>) {
        cx.reveal_path(&PathBuf::from(&self.snapshot.bootstrap.logs_path));
    }
}

impl Focusable for NativeView {
    fn focus_handle(&self, _: &gpui_kit::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for NativeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let onboarding = !self.snapshot.bootstrap.onboarding.finished;
        let body = if onboarding {
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(self.render_header(false, window, cx))
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .child(self.render_onboarding(window, cx)),
                )
                .into_any_element()
        } else {
            let page = match &self.route {
                Route::Explore => self.render_explore(window, cx),
                Route::Library => self.render_library(window, cx),
                Route::Skill(id) => self.render_skill(&id.clone(), cx),
                Route::Workspace(id) => self.render_workspace(&id.clone(), cx),
                Route::Agent(id) => self.render_agent(&id.clone(), cx),
                Route::Universal => self.render_universal(cx),
                Route::Sync => self.render_sync(cx).into_any_element(),
                Route::Settings => self.render_settings(cx).into_any_element(),
            };
            self.render_shell(page, window, cx)
        };
        #[cfg(feature = "native-test")]
        let body = self.gallery().unwrap_or(body);
        self.sync_detail(cx);
        self.sync_dialog(window, cx);
        self.sync_overlay_focus(window, cx);
        self.push_toast(window, cx);
        self.sync_agent_notice(window, cx);
        let colors = cx.theme().colors;
        let global_error = self
            .error
            .as_ref()
            .or(self.snapshot.background_error.as_ref());
        div()
            .track_focus(&self.focus)
            .on_action(cx.listener(|view, _: &ToggleSidebar, _, cx| {
                if !view.blocked() {
                    view.toggle_sidebar(cx);
                }
            }))
            .when(!onboarding, |root| {
                root.on_action(cx.listener(|view, _: &GoBack, window, cx| {
                    if !view.blocked() {
                        view.go_back(window, cx);
                    }
                }))
                .on_action(cx.listener(|view, _: &GoForward, window, cx| {
                    if !view.blocked() {
                        view.go_forward(window, cx);
                    }
                }))
            })
            .size_full()
            .font_family(skillbinder_design::system_font())
            .bg(colors.background)
            .text_color(colors.foreground)
            .child(body)
            .when_some(global_error, |root, error| {
                root.child(
                    Notice::blocking(
                        "global-error",
                        Status::Danger,
                        error.message.clone(),
                        format!("{} (Diagnostic {})", error.recovery, error.diagnostic_id),
                    )
                    .absolute()
                    .bottom(px(space::XL))
                    .right(px(space::XL))
                    .max_w_96(),
                )
            })
            .children(self.render_place_popover(window, cx))
            .children(self.menu.as_ref().map(skillbinder_design::OpenMenu::render))
            .children(self.render_strip(window, cx))
    }
}
