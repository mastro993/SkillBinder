//! Sync page metrics in points, on the shared scales.

use crate::tokens::space;

/// Width of the page's one column, from the sync prototype. No scale token is near it.
pub const COLUMN: f32 = 720.0;
/// Width of Not connected's setup column, from the sync prototype.
pub const SETUP_COLUMN: f32 = 460.0;
/// The status card's tinted glyph tile: the prototype's 36 points.
pub const STATUS_TILE: f32 = space::CONTROL_MD + space::XS;
/// The setup column's glyph tile: the prototype's 44 points.
pub const SETUP_TILE: f32 = space::CONTROL_MD + space::LG;
/// Space above the setup column, the prototype's 48 points.
pub const SETUP_TOP: f32 = space::XXXL + space::XL;
