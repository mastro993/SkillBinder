//! Application appearance and typography.

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{App, Global, Window};

/// Height of the window header row.
pub const HEADER_HEIGHT: f32 = 50.0;

/// Height of the window-wide status bar.
pub const STATUS_BAR_HEIGHT: f32 = 30.0;

/// Top-left of the 14 pt macOS traffic lights, vertically centred in the header row.
pub const TRAFFIC_LIGHTS_ORIGIN: (f32, f32) = (16.0, (HEADER_HEIGHT - 14.0) / 2.0);

/// Applies the system font used by the production baseline.
pub fn init(cx: &mut App) {
    Theme::update(cx, |theme| theme.font_family = system_font().into());
}

/// Native system font matching the baseline's system-ui fallback.
pub fn system_font() -> &'static str {
    ".SystemUIFont"
}

/// Explicit mode chosen in the app; absent while the app follows the window appearance.
struct ModeOverride(Option<ThemeMode>);

impl Global for ModeOverride {}

/// Tracks the native window appearance unless the app has chosen a mode.
pub fn follow_window(window: &mut Window, cx: &mut App) {
    apply_mode(effective_mode(window, cx), cx);
    window
        .observe_window_appearance(|window, cx| apply_mode(effective_mode(window, cx), cx))
        .detach();
}

/// Forces `mode`, or follows the window appearance again when it is `None`.
pub fn set_mode(mode: Option<ThemeMode>, window: &mut Window, cx: &mut App) {
    cx.set_global(ModeOverride(mode));
    apply_mode(effective_mode(window, cx), cx);
    window.refresh();
}

fn effective_mode(window: &Window, cx: &App) -> ThemeMode {
    cx.try_global::<ModeOverride>()
        .and_then(|mode| mode.0)
        .unwrap_or_else(|| ThemeMode::from(window.appearance()))
}

/// Loads the mode's colors, then keeps the sidebar slightly darker than the main surface.
fn apply_mode(mode: ThemeMode, cx: &mut App) {
    Theme::change(mode, None, cx);
    Theme::update(cx, |theme| {
        let background = theme.background;
        theme.sidebar = gpui_kit::Hsla {
            l: (background.l - 0.02).max(0.0),
            ..background
        };
    });
}
