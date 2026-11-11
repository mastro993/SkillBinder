//! Application appearance and typography.

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{App, FontWeight, Global, Window};

/// Height of the window header row.
pub const HEADER_HEIGHT: f32 = 44.0;
/// Label size of the Library header's folder tabs.
pub const FOLDER_TAB_SIZE: f32 = 13.0;

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

/// Context menu metrics in points. Rows reuse the [`sidebar`] row height, padding, and icon
/// metrics, and a highlighted row is rounded to the theme radius.
pub mod menu {
    use gpui_kit::component::Theme;
    use gpui_kit::{BoxShadow, Hsla, hsla, px};

    /// Corner radius of a menu, submenu, or panel.
    pub const RADIUS: f32 = 12.0;
    /// Space between the menu's edge and its rows; separators are inset by the same amount.
    pub const PADDING: f32 = 8.0;
    /// Row label size, the macOS menu size, a step below the sidebar's.
    pub const LABEL_SIZE: f32 = 13.0;
    /// Narrowest a menu or submenu is drawn.
    pub const MIN_WIDTH: f32 = 180.0;
    /// Size of the check and of a submenu row's chevron.
    pub const TRAILING_SIZE: f32 = 14.0;
    /// Space above and below a separator.
    pub const SEPARATOR_MARGIN: f32 = 4.0;
    /// Foreground alpha of the menu's edge and its separators.
    pub const LINE_INK: f32 = 0.1;
    /// Foreground alpha of the highlighted row's fill.
    pub const HIGHLIGHT_INK: f32 = 0.08;
    /// Opacity of a disabled row.
    pub const DISABLED_OPACITY: f32 = 0.4;
    /// How far a submenu overlaps the edge of its parent.
    pub const SUBMENU_OVERLAP: f32 = 4.0;
    /// Closest a menu comes to the window edges.
    pub const WINDOW_MARGIN: f32 = 8.0;

    /// The menu surface, a step above the sidebar it opens over. The dark popover is darker
    /// than the dark sidebar, so dark mode lifts it to the muted color.
    pub fn surface(theme: &Theme) -> Hsla {
        if theme.is_dark() {
            theme.muted
        } else {
            theme.popover
        }
    }

    /// The menu's edge, a hairline `ring` just outside it, then a soft drop shadow, all at
    /// `strength` so an opening menu can fade them in. A border would be drawn over a submenu
    /// that overlaps the menu; a shadow is drawn under it.
    pub fn shadow(ring: Hsla, strength: f32) -> Vec<BoxShadow> {
        let strength = strength.clamp(0., 1.);
        let ink = hsla(0., 0., 0., 0.1 * strength);
        vec![
            BoxShadow::new(px(0.), px(0.), ring.opacity(strength))
                .blur_radius(px(0.))
                .spread_radius(px(1.)),
            BoxShadow::new(px(0.), px(10.), ink)
                .blur_radius(px(15.))
                .spread_radius(px(-3.)),
            BoxShadow::new(px(0.), px(4.), ink)
                .blur_radius(px(6.))
                .spread_radius(px(-4.)),
        ]
    }
}

/// Library metrics in points, measured from the reference design.
pub mod library {
    use gpui_kit::Hsla;
    use gpui_kit::component::Theme;

    /// Space between the page's edges and its toolbar and list.
    pub const PAGE_GUTTER: f32 = 16.0;
    /// Space above the toolbar.
    pub const PAGE_TOP: f32 = 14.0;
    /// Space between the toolbar and the list.
    pub const PAGE_GAP: f32 = 12.0;
    /// Space below the list's end, inside its scroll view.
    pub const PAGE_BOTTOM: f32 = 16.0;
    /// Height of the toolbar row: GPUI Kit's standard control height, which its search field,
    /// segmented filters, and Updates chip take.
    pub const TOOLBAR_HEIGHT: f32 = 32.0;
    /// Corner radius of the card that holds the list.
    pub const CARD_RADIUS: f32 = 10.0;
    /// Shortest a list row is drawn.
    pub const ROW_MIN_HEIGHT: f32 = 56.0;
    /// Horizontal padding inside a list row.
    pub const ROW_PADDING_X: f32 = 14.0;
    /// Vertical padding inside a list row.
    pub const ROW_PADDING_Y: f32 = 10.0;
    /// Space between a row's text and its end.
    pub const ROW_GAP: f32 = 12.0;
    /// How far a row's divider is inset from the card's edges, so it starts at the name.
    pub const DIVIDER_INSET: f32 = 14.0;
    /// Corner radius of a grid tile.
    pub const TILE_RADIUS: f32 = 10.0;
    /// Padding inside a grid tile.
    pub const TILE_PADDING: f32 = 12.0;
    /// Space between a tile's lines.
    pub const TILE_GAP: f32 = 4.0;
    /// Shortest a grid tile is drawn.
    pub const TILE_MIN_HEIGHT: f32 = 116.0;
    /// Narrowest a grid column is drawn.
    pub const GRID_MIN: f32 = 180.0;
    /// Space between grid tiles.
    pub const GRID_GAP: f32 = 10.0;
    /// Below this width a list hides folder tags and counts a skill's workspaces in one chip.
    pub const NARROW_LIST: f32 = 440.0;
    /// Below this width a grid tile counts a skill's workspaces in one chip.
    pub const NARROW_TILE: f32 = 200.0;

    /// The surface of the list card: the page's own color, lifted a step in the dark theme so the
    /// card and its hairline read as one object against the page.
    pub fn surface(theme: &Theme) -> Hsla {
        if theme.is_dark() {
            Hsla {
                l: theme.background.l + 0.02,
                ..theme.background
            }
        } else {
            theme.background
        }
    }
}

/// Applies the system font used by the production baseline.
pub fn init(cx: &mut App) {
    Theme::update(cx, |theme| theme.font_family = system_font().into());
}

/// Native system font matching the baseline's system-ui fallback.
pub fn system_font() -> &'static str {
    ".SystemUIFont"
}

/// The system font's Medium face, the design's `font-weight: 500`. GPUI matches weights through
/// font-kit, which places SF Pro Medium's Core Text weight (0.23) at 530, so a request for 500
/// (`FontWeight::MEDIUM`) falls back to the Regular face and medium text would draw regular.
/// font-kit gives Medium every request above 500 up to 530 (Semibold reads 600): 520 sits inside.
pub const MEDIUM: FontWeight = FontWeight(520.0);

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
/// The light sidebar stays slightly darker than the main surface, with a soft `#ebebeb`
/// selection fill.
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
            theme.sidebar_accent = gpui_kit::rgb(0xebebeb).into();
        }
    });
}
