mod agent_page;
mod agent_skills;
mod app_update_notice;
mod applications_menu;
mod breadcrumb;
mod controls;
mod copies_review;
mod copy_changed;
mod copy_dialogs;
mod creation;
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
mod move_to_applications;
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

pub(crate) use app_update_notice::RestartBlocker;
pub(crate) use library_copies::{AgentRow, found_rows, shown_dir};
pub(crate) use library_targets::held;
pub(crate) use library_updates::{available, summary};
#[cfg(feature = "native-test")]
pub use native_test::StripHold;
pub(crate) use place_popover::bind_keys;
pub(crate) use preset_activation::{activate, resolved_active, switching};
pub(crate) use preset_dialogs::switch_note;
pub(crate) use progress_strip::{Activity, Landing, waiting_copies};
pub use shell::{GoBack, GoForward, ToggleSidebar};
#[cfg(test)]
pub(crate) use snapshot_query::fixture;
pub(crate) use snapshot_query::{
    ActiveFilter, Entry as AgentEntry, LibraryFilter, Places, Query, has_update, library_counts,
    library_skills, workspace_skills,
};
pub(crate) use workspace_targets::offered_folders;

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use gpui_kit::component::{ActiveTheme, input::InputState};
use gpui_kit::{
    Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement,
    Render, Styled, Subscription, Task, Window, div, prelude::FluentBuilder, px,
};
use library_folders::FolderEdit;
use route::{History, Route};
use sidebar_presets::PresetEdit;
use sidebar_reorder::RowReorder;
use skillbinder_client::Client;
use skillbinder_design::tokens::space;
use skillbinder_design::{Notice, Status};
use skillbinder_proto::*;
use updates::Effect;

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
    /// macOS, outside Applications: asks to move there before anything else (spec rule 21), and
    /// says so when Move failed.
    MoveToApplications {
        failed: bool,
    },
    /// Linux, as an AppImage: asks once to add SkillBinder to the applications menu (spec rule
    /// 23), and says so when Add failed.
    ApplicationsMenu {
        failed: bool,
    },
    None,
}

/// GPUI owner of the native product journeys and client projections.
pub struct NativeView {
    client: Client,
    snapshot: Arc<AppSnapshot>,
    focus: FocusHandle,
    route: Route,
    history: History,
    /// The view's short work: a single Update or an optimistic save. Controls read
    /// [`Self::working`], which adds Update all.
    busy: bool,
    /// A command's short change runs, which the window waits for as for its own saves.
    commanded: bool,
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
    app_update_notice: app_update_notice::Notice,
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
    settings: settings::Settings,
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
    /// Whether the menu bar or system tray icon shows (Settings › General).
    show_in_tray: bool,
    /// A preset clicked while its activation is saved, shown active until the engine answers.
    active_preset: Option<PresetId>,
    /// The copies that asked the user in a dialog already, by directory, skill, and state.
    asked_copies: Vec<(String, SkillId, CopyState)>,
    /// A preset name shown while its rename is being saved; cleared on success or failure.
    optimistic_preset_name: Option<(PresetId, String)>,
    /// Created or cloned presets shown before the engine's snapshot arrives.
    optimistic_presets: Vec<sidebar_presets::OptimisticPreset>,
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
    /// Ends the strip once its change returns, whoever started it.
    _gate_subscription: Subscription,
    _subscription: Task<()>,
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
                Route::Settings => self.render_settings(cx),
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
        self.sync_app_update_notice(window, cx);
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
