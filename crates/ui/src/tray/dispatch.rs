//! What a menu click does: the same as in the window while one is open, otherwise the same work
//! run without it (spec rule 12, A7).

use super::{Tray, TrayAction, open_window};
use crate::gate::{self, Gate, main_view};
use crate::native_view::{activate, resolved_active, switching};
use gpui_kit::App;
use skillbinder_proto::PresetId;

pub(super) fn dispatch(action: TrayAction, cx: &mut App) {
    match action {
        TrayAction::ReviewCopies => {
            open_window(cx);
            if let Some(view) = main_view(cx) {
                view.update(cx, |view, cx| view.review_copies(cx));
            }
        }
        TrayAction::ActivatePreset(id) => activate_preset(id, cx),
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
    if Gate::of(cx).blocked() || active.as_ref() == Some(&id) {
        return;
    }
    if let Some(activity) = switching(&snapshot, &id) {
        let change = activate(client.clone(), id);
        gate::start_change(&client, activity, change, cx);
    }
}
