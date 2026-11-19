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
/// Width of the skill detail dialog's About and Files sidebar, from the skill-detail prototype.
/// No scale token is near it.
pub const DETAIL_SIDEBAR: f32 = 260.0;
/// Space between the skill detail dialog's band and its columns.
pub const DETAIL_GAP: f32 = space::XL;
/// Whitespace between the skill detail dialog's page and its sidebar, which no rule divides.
pub const DETAIL_COLUMN_GAP: f32 = space::XXL;
/// Padding inside the skill detail sidebar's Files card.
pub const DETAIL_CARD_PADDING: f32 = space::LG;
/// Height of a row of the skill detail dialog's file tree.
pub const TREE_ROW: f32 = space::CONTROL_SM;
/// How far each folder level indents its rows in the file tree.
pub const TREE_INDENT: f32 = space::LG;
/// A repository row's glyph tile, square: the creator chip's avatar box.
pub const REPOSITORY_TILE: f32 = space::CONTROL_SM;
/// How far a repository's skills indent under its row: past its glyph tile, so their names line
/// up with the repository's.
pub const REPOSITORY_INDENT: f32 = space::LG + REPOSITORY_TILE + space::LG;
/// Height of a slug line in the Add repository dialog's list of skills found.
pub const FOUND_ROW: f32 = space::CONTROL_SM;
