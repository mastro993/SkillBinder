//! Context menu metrics in points, on the shared scales. Rows reuse the [`crate::sidebar`] row
//! height, padding, and icon metrics, and a highlighted row is rounded to the theme radius.

use crate::tokens::{icon, opacity, radius, space, text};
use gpui_kit::component::Theme;
use gpui_kit::{BoxShadow, Hsla, hsla, px};

/// Corner radius of a menu, submenu, or panel.
pub const RADIUS: f32 = radius::XL;
/// Space between the menu's edge and its rows; separators are inset by the same amount.
pub const PADDING: f32 = space::MD;
/// Row label size, the macOS menu size, a step below the sidebar's.
pub const LABEL_SIZE: f32 = text::BODY_SM.size;
/// Narrowest a menu or submenu is drawn.
pub const MIN_WIDTH: f32 = 180.0;
/// Size of the check and of a submenu row's chevron.
pub const TRAILING_SIZE: f32 = icon::SM;
/// Space above and below a separator.
pub const SEPARATOR_MARGIN: f32 = space::XS;
/// Foreground alpha of the menu's edge and its separators.
pub const LINE_INK: f32 = 0.1;
/// Foreground alpha of the highlighted row's fill.
pub const HIGHLIGHT_INK: f32 = 0.08;
/// Opacity of a disabled row.
pub const DISABLED_OPACITY: f32 = opacity::DISABLED;
/// How far a submenu overlaps the edge of its parent.
pub const SUBMENU_OVERLAP: f32 = space::XS;
/// Closest a menu comes to the window edges.
pub const WINDOW_MARGIN: f32 = space::MD;

/// The menu surface: the popover color, a step above the sidebar it opens over in both modes.
pub fn surface(theme: &Theme) -> Hsla {
    theme.popover
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
