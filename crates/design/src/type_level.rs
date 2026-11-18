//! A level of the type scale.

use gpui_kit::FontWeight;

/// A `DESIGN.md` typography level: its size in points, weight, line height as a multiple of the
/// size, and whether it is set in the monospace font.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TypeLevel {
    /// Font size in points.
    pub size: f32,
    /// Font weight; the medium weight is 520, which GPUI draws with SF Pro Medium.
    pub weight: FontWeight,
    /// Line height as a multiple of [`TypeLevel::size`].
    pub line_height: f32,
    /// Set in the monospace font rather than the system font.
    pub mono: bool,
}
