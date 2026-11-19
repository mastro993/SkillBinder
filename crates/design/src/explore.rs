//! Explore metrics in points, on the shared scales.

use crate::library::GRID_MIN;
use crate::tokens::{icon, space};

/// Width of a creator chip in Popular creators' row, a grid column's.
pub const CREATOR_CHIP_WIDTH: f32 = GRID_MIN;
/// A creator chip's avatar: the prototype's 28-point box, snapped to the small control.
pub const CREATOR_AVATAR: f32 = space::CONTROL_SM;
/// The avatar in a creator's scope header.
pub const SCOPE_AVATAR: f32 = space::CONTROL_MD;
/// The avatar in a result row's source tag.
pub const SOURCE_AVATAR: f32 = icon::SM;
/// Space between creator chips, and between a chip's avatar and its text.
pub const CHIP_GAP: f32 = space::MD;
/// Padding inside a creator chip.
pub const CHIP_PADDING: f32 = space::SM;
/// Space between a section's blocks: its header, then its cards or chips.
pub const SECTION_GAP: f32 = space::MD;
/// Height of a skeleton line standing in for a card's name or description.
pub const SKELETON_LINE: f32 = space::LG;
