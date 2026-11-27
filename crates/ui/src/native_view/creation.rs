//! Creating the view: its state from the first snapshot, its inputs, and its subscriptions.

#[cfg(feature = "native-test")]
use super::native_test;
use super::route::{History, Route};
use super::sidebar_reorder::RowReorder;
use super::{
    DialogState, NativeView, app_update_notice, explore, folder_reorder, git_setup, hover_scroll,
    new_agent_notice, onboarding, settings, shell, sidebar, snapshot_query, sync, toasts,
};
use crate::gate::{self, Gate};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::{AppContext, Context, Entity, Window};
use skillbinder_client::Client;
use std::collections::{BTreeMap, BTreeSet};

impl NativeView {
    /// Creates the native view and subscribes to client projections.
    pub fn new(client: Client, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let snapshot = client.snapshot();
        let sidebar_width = sidebar::clamp_sidebar_width(snapshot.preferences.sidebar_width);
        let sidebar_collapsed = snapshot.preferences.sidebar_collapsed;
        let folded_sections = snapshot.preferences.folded_sections.clone();
        let appearance = snapshot.preferences.appearance;
        let library_view = snapshot.preferences.library_view;
        let show_in_tray = snapshot.preferences.show_in_tray;
        // Launch offers and the "Updated to" toast show in the launch's first window only.
        let earlier = cx.try_global::<app_update_notice::LaunchNotice>().copied();
        let dialog = match earlier {
            Some(_) => DialogState::None,
            None => Self::first_dialog(&snapshot.app_update),
        };
        let notice = earlier.map_or_else(
            app_update_notice::Notice::default,
            app_update_notice::Notice::resumed,
        );
        cx.set_global(notice.launch());
        sidebar::apply_appearance(appearance, window, cx);
        // Launch refreshes through the client; afterwards only regaining focus refreshes.
        let activation_subscription = cx.observe_window_activation(window, |view, window, cx| {
            if !window.is_window_active() {
                view.left_focus = true;
            } else if std::mem::take(&mut view.left_focus) {
                view.client.refresh_app();
                // An agent such as Cursor may have been connected outside SkillBinder.
                if view.route == Route::Settings {
                    view.read_mcp_clients(cx);
                }
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
        let remote_url =
            cx.new(|cx| InputState::new(window, cx).placeholder("git@github.com:you/skills.git"));
        let remote_branch = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Branch")
                .default_value("main")
        });
        let device_name = cx.new(|cx| InputState::new(window, cx).placeholder("Device name"));
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
        let gate_subscription = cx.observe_global::<Gate>(|view, cx| {
            let commanded = Gate::of(cx).commands > 0;
            if view.commanded != commanded {
                view.commanded = commanded;
                cx.notify();
            }
            view.follow_gate(cx);
        });
        gate::opened(cx.entity().downgrade(), window.window_handle(), cx);
        let mut view = Self {
            client,
            snapshot,
            focus,
            route: Route::default(),
            history: History::default(),
            busy: false,
            commanded: Gate::of(cx).commands > 0,
            error: None,
            dialog,
            dialogs_opened: 0,
            importing: None,
            first_import: None,
            onboarding_back_to: None,
            unticked_agents: None,
            agents_turned_off: BTreeSet::new(),
            agent_notice: new_agent_notice::Notice::default(),
            app_update_notice: notice,
            held_step: None,
            offers: onboarding::Offers::default(),
            git_setup: git_setup::GitSetup::default(),
            git_checking: false,
            menu: None,
            preset_edit: None,
            preset_reorder: RowReorder::default().pinned(1),
            explore: explore::Explore::new(window, cx),
            settings: settings::Settings::new(window, cx),
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
            show_in_tray,
            active_preset: None,
            asked_copies: Vec::new(),
            optimistic_preset_name: None,
            optimistic_presets: Vec::new(),
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
            found_flash: None,
            skill_load: None,
            sync_history: sync::HistoryPages::default(),
            sync_choices: sync::Choices::default(),
            search,
            preset_name,
            folder_name,
            workspace_name,
            remote_url,
            remote_branch,
            device_name,
            _search_subscription: search_subscription,
            _preset_name_subscription: preset_name_subscription,
            _folder_name_subscription: folder_name_subscription,
            _workspace_name_subscription: workspace_name_subscription,
            left_focus: false,
            _activation_subscription: activation_subscription,
            _focus_lost_subscription: focus_lost_subscription,
            _gate_subscription: gate_subscription,
            _subscription: subscription,
        };
        view.adopt_strip(cx);
        view
    }

    pub(super) fn input_text(input: &Entity<InputState>, cx: &Context<Self>) -> String {
        input.read(cx).value().trim().to_owned()
    }
}
