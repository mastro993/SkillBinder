//! A menu's metrics in points, on the shared scales, and the rows, dividers, and surface they
//! draw. Rows reuse the [`crate::sidebar`] row height, padding, and icon metrics, rounded to
//! `sm`.

use crate::sidebar::{ICON_GAP, ROW_HEIGHT, ROW_PADDING};
use crate::tokens::{icon, opacity, radius, space, text};
use crate::{Level, Separator, Surface};
use gpui_kit::component::Theme;
use gpui_kit::{Hsla, Styled, px};

/// Corner radius of a menu, submenu, or panel.
pub const RADIUS: f32 = radius::XL;
/// Space between the menu's edge and its rows.
pub const PADDING: f32 = space::MD;
/// Narrowest a menu or submenu is drawn.
pub const MIN_WIDTH: f32 = 180.0;
/// Size of the check and of a submenu row's chevron.
pub const TRAILING_SIZE: f32 = icon::SM;
/// Space above and below a separator.
pub const SEPARATOR_MARGIN: f32 = space::XS;
/// Foreground alpha of the highlighted row's fill.
pub const HIGHLIGHT_INK: f32 = 0.08;
/// Opacity of a disabled row.
pub const DISABLED_OPACITY: f32 = opacity::DISABLED;
/// How far a submenu overlaps the edge of its parent.
pub const SUBMENU_OVERLAP: f32 = space::XS;
/// Closest a menu comes to the window edges.
pub const WINDOW_MARGIN: f32 = space::MD;

/// The fill of a highlighted row.
pub fn highlight(theme: &Theme) -> Hsla {
    theme.foreground.alpha(HIGHLIGHT_INK)
}

/// Lays `row` out as a menu row: its icon, label, and trailing mark in a row the sidebar row's
/// height, in `body-sm`.
pub fn row<E: Styled>(row: E) -> E {
    text::BODY_SM
        .apply(row)
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap(px(ICON_GAP))
        .h(px(ROW_HEIGHT))
        .px(px(ROW_PADDING))
        .rounded(px(radius::SM))
}

/// A divider between groups of rows.
pub fn separator() -> Separator {
    Separator::horizontal()
        .flex_shrink_0()
        .my(px(SEPARATOR_MARGIN))
}

/// The overlay surface shared by a menu, its submenu, and a panel. Its ring is a shadow, which a
/// submenu overlapping the menu covers, where a border would be drawn over it.
pub(super) fn panel<E: Styled>(panel: E, theme: &Theme) -> E {
    let panel = Surface::new(Level::Overlay)
        .radius(RADIUS)
        .apply(panel, theme);
    text::BODY_SM
        .apply(panel)
        .flex()
        .flex_col()
        .min_w(px(MIN_WIDTH))
        .p(px(PADDING))
        .text_color(theme.popover_foreground)
}
