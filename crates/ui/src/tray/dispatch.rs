//! What a menu click does: the same as in the window while one is open, otherwise the same work
//! run without it (spec rules 12 and 13, A7).

use super::{Tray, TrayAction, open_window};
use crate::gate::{self, Gate, main_view, update_main_view};
use crate::native_view::{RestartBlocker, activate, resolved_active, switching};
use gpui_kit::App;
use skillbinder_proto::{AppUpdateState, PresetId};

pub(super) fn dispatch(action: TrayAction, cx: &mut App) {
    match action {
        TrayAction::ReviewCopies => {
            open_window(cx);
            if let Some(view) = main_view(cx) {
                view.update(cx, |view, cx| view.review_copies(cx));
            }
        }
        TrayAction::UpdateAll => update_all(cx),
        TrayAction::LibraryUpdates => {
            open_window(cx);
            update_main_view(cx, |view, window, cx| view.show_library_updates(window, cx));
        }
        TrayAction::ActivatePreset(id) => activate_preset(id, cx),
        TrayAction::CheckAppUpdate => cx.global::<Tray>().client.check_app_update(),
        TrayAction::Restart => restart(cx),
        TrayAction::DownloadAppUpdate => {
            if let AppUpdateState::DownloadOnly { notes_url, .. } =
                &cx.global::<Tray>().client.snapshot().app_update.state
            {
                cx.open_url(notes_url);
            }
        }
        TrayAction::OpenWindow => open_window(cx),
        TrayAction::Quit => cx.quit(),
    }
}

/// Switches as clicking the preset in the sidebar does, or with no window, runs the switch behind
/// the icon, refused while anything blocks it and for the active preset.
fn activate_preset(id: PresetId, cx: &mut App) {
    if let Some(view) = main_view(cx) {
        view.update(cx, |view, cx| view.set_active_preset(id, cx));
        return;
    }
    let client = cx.global::<Tray>().client.clone();
    let snapshot = client.snapshot();
    let active = snapshot
        .library
        .as_ref()
        .and_then(|library| {
            resolved_active(&library.library.presets, &library.library.active_preset)
        })
        .map(|preset| preset.id.clone());
    if Gate::of(cx).blocked(&snapshot) || active.as_ref() == Some(&id) {
        return;
    }
    if let Some(activity) = switching(&snapshot, &id) {
        let change = activate(client.clone(), id);
        gate::start_change(&client, activity, change, cx);
    }
}

/// Runs Update all as Library's header does, or with no window, in the client alone, refused
/// whenever the window would refuse it (spec rule 10).
fn update_all(cx: &mut App) {
    if let Some(view) = main_view(cx) {
        view.update(cx, |view, cx| view.update_all(cx));
        return;
    }
    let client = cx.global::<Tray>().client.clone();
    if !Gate::of(cx).refuses_update_all(&client.snapshot()) {
        // The run reports on the snapshot, which the menu shows; nothing else waits for it.
        client.runtime().spawn(client.update_all());
    }
}

/// Restarts as the window's Restart does, or with no window, asks the engine to install the ready
/// app update at quit and quits, refused while one of Restart's blockers runs. Should the engine
/// refuse, the window opens to show why.
fn restart(cx: &mut App) {
    if main_view(cx).is_none() {
        let client = cx.global::<Tray>().client.clone();
        let snapshot = client.snapshot();
        if RestartBlocker::of(&snapshot, Gate::of(cx).blocked(&snapshot)).is_some() {
            return;
        }
        if client.request_restart().is_ok() {
            cx.quit();
            return;
        }
        open_window(cx);
    }
    if let Some(view) = main_view(cx) {
        view.update(cx, |view, cx| view.restart(cx));
    }
}
