//! Application appearance and typography.

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{App, Global, Window};

/// Height of the window header row.
pub const HEADER_HEIGHT: f32 = 50.0;

/// Height of the window-wide status bar.
pub const STATUS_BAR_HEIGHT: f32 = 30.0;

/// Top-left of the 14 pt macOS traffic lights, vertically centred in the header row.
pub const TRAFFIC_LIGHTS_ORIGIN: (f32, f32) = (16.0, (HEADER_HEIGHT - 14.0) / 2.0);

/// Sidebar metrics in points, measured from the reference design.
pub mod sidebar {
    /// Height of a navigation or preset row.
    pub const ROW_HEIGHT: f32 = 32.0;
    /// Vertical space between rows.
    pub const ROW_GAP: f32 = 1.0;
    /// Space between the sidebar edges and a row's selection fill.
    pub const INSET: f32 = 8.0;
    /// Horizontal padding inside a row.
    pub const ROW_PADDING: f32 = 8.0;
    /// Row icon size.
    pub const ICON_SIZE: f32 = 16.0;
    /// Space between a row icon and its label.
    pub const ICON_GAP: f32 = 8.0;
    /// Row label size.
    pub const LABEL_SIZE: f32 = 14.0;
    /// Section title and trailing count size.
    pub const DETAIL_SIZE: f32 = 12.0;
    /// Height of a section title row.
    pub const SECTION_HEIGHT: f32 = 28.0;
    /// Space above a section title.
    pub const SECTION_GAP: f32 = 8.0;
    /// Corner radius of the selection fill.
    pub const RADIUS: f32 = 8.0;
    /// Diameter of a row's attention dot.
    pub const DOT_SIZE: f32 = 8.0;
}

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

/// Loads the mode's colors, then applies the sidebar surfaces of the reference design.
///
/// The light sidebar stays slightly darker than the main surface.
fn apply_mode(mode: ThemeMode, cx: &mut App) {
    Theme::change(mode, None, cx);
    Theme::update(cx, |theme| {
        if mode.is_dark() {
            theme.sidebar = gpui_kit::rgb(0x161616).into();
            theme.sidebar_accent = gpui_kit::rgb(0x2f2f2f).into();
            theme.sidebar_accent_foreground = gpui_kit::rgb(0xebebeb).into();
        } else {
            let background = theme.background;
            theme.sidebar = gpui_kit::Hsla {
                l: (background.l - 0.02).max(0.0),
                ..background
            };
        }
    });
}
