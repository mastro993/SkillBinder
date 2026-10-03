//! Application appearance and typography.

use ely_gpui_component::theme::{Mode, Theme};
use gpui::{App, Window};

/// Applies the system font used by the production baseline.
pub fn init(cx: &mut App) {
    let theme = cx.global_mut::<Theme>();
    theme.font_family = system_font().into();
}

/// Native system font matching the baseline's system-ui fallback.
pub fn system_font() -> &'static str {
    ".SystemUIFont"
}

/// Tracks the native window appearance without forcing a system preference.
pub fn follow_window(window: &mut Window, cx: &mut App) {
    Theme::set_mode_now(Mode::from(window.appearance()), cx);
    window
        .observe_window_appearance(|window, cx| {
            Theme::set_mode_now(Mode::from(window.appearance()), cx);
        })
        .detach();
}
