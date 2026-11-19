//! Sidebar metrics in points, on the shared scales.

use crate::tokens::{icon, opacity, radius, space, text};

/// Height of a navigation or preset row.
pub const ROW_HEIGHT: f32 = space::SIDEBAR_ROW;
/// Vertical space between rows: a hairline, below the spacing scale.
pub const ROW_GAP: f32 = 1.0;
/// Space between the sidebar edges and a row's selection fill.
pub const INSET: f32 = space::MD;
/// Horizontal padding inside a row.
pub const ROW_PADDING: f32 = space::MD;
/// Row icon size.
pub const ICON_SIZE: f32 = icon::MD;
/// Space between a row icon and its label.
pub const ICON_GAP: f32 = space::MD;
/// Section title and trailing count size.
pub const DETAIL_SIZE: f32 = text::CAPTION.size;
/// Height of a section title row, measured from the reference design.
pub const SECTION_HEIGHT: f32 = 28.0;
/// Space above a section title.
pub const SECTION_GAP: f32 = space::MD;
/// Corner radius of the selection fill.
pub const RADIUS: f32 = radius::MD;
/// Diameter of a row's attention dot.
pub const DOT_SIZE: f32 = space::MD;
/// Opacity of the selection fill under the pointer, or behind a row whose menu is open.
pub const HOVER_OPACITY: f32 = opacity::INACTIVE;
/// Opacity of an active preset's color fill: fainter than the selection fill in both modes, and
/// lighter still in light mode, whose selection fill is dark against its surface.
pub const PRESET_TINT_OPACITY: f32 = 0.07;
/// [`PRESET_TINT_OPACITY`] in dark mode.
pub const PRESET_TINT_OPACITY_DARK: f32 = 0.15;
/// Opacity of the selection fill for an active preset with no color, kept fainter than a selected
/// navigation row like the tinted fill.
pub const PRESET_NEUTRAL_OPACITY: f32 = 0.6;
