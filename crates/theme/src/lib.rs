//! Application appearance and typography.

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{App, Window};

/// Applies the system font used by the production baseline.
pub fn init(cx: &mut App) {
    Theme::update(cx, |theme| theme.font_family = system_font().into());
}

/// Native system font matching the baseline's system-ui fallback.
pub fn system_font() -> &'static str {
    ".SystemUIFont"
}

/// Tracks the native window appearance without forcing a system preference.
pub fn follow_window(window: &mut Window, cx: &mut App) {
    Theme::change(ThemeMode::from(window.appearance()), Some(window), cx);
    window
        .observe_window_appearance(|window, cx| {
            Theme::change(ThemeMode::from(window.appearance()), Some(window), cx);
        })
        .detach();
}
