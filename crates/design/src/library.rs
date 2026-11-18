//! Library metrics in points, on the shared scales.

use crate::tokens::{radius, space};
use gpui_kit::Hsla;
use gpui_kit::component::Theme;

/// Space between the page's edges and its toolbar and list.
pub const PAGE_GUTTER: f32 = space::XL;
/// Space above the toolbar, the page gutter.
pub const PAGE_TOP: f32 = space::XL;
/// Space between the toolbar and the list.
pub const PAGE_GAP: f32 = space::LG;
/// Space below the list's end, inside its scroll view.
pub const PAGE_BOTTOM: f32 = space::XL;
/// Height of the toolbar row: GPUI Kit's standard control height, which its search field,
/// segmented filters, and Updates chip take.
pub const TOOLBAR_HEIGHT: f32 = space::CONTROL_MD;
/// Corner radius of the card that holds the list.
pub const CARD_RADIUS: f32 = radius::LG;
/// Shortest a list row is drawn.
pub const ROW_MIN_HEIGHT: f32 = space::LIST_ROW;
/// Horizontal padding inside a list row.
pub const ROW_PADDING_X: f32 = space::LG;
/// Vertical padding inside a list row.
pub const ROW_PADDING_Y: f32 = space::MD;
/// Space between a row's text and its end.
pub const ROW_GAP: f32 = space::LG;
/// How far a row's divider is inset from the card's edges, so it starts at the name.
pub const DIVIDER_INSET: f32 = ROW_PADDING_X;
/// Corner radius of a grid tile.
pub const TILE_RADIUS: f32 = radius::LG;
/// Padding inside a grid tile.
pub const TILE_PADDING: f32 = space::LG;
/// Space between a tile's lines.
pub const TILE_GAP: f32 = space::XS;
/// Shortest a grid tile is drawn.
pub const TILE_MIN_HEIGHT: f32 = 116.0;
/// Narrowest a grid column is drawn.
pub const GRID_MIN: f32 = 180.0;
/// Space between grid tiles.
pub const GRID_GAP: f32 = space::LG;
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
