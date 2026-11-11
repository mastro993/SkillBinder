//! The design's type: a font size on a line box 1.4 times as high, and tabular figures for counts.
//! A line of text that an enclosing element's accessible name does not read is GPUI text with an
//! id, which makes it a label holding its text and lets the native-test bridge name it; a line
//! such a name reads is a plain string, so assistive technology reads it once.

use gpui_kit::{FontFeatures, Styled, px};
use std::sync::Arc;

/// The design's line height, as a multiple of the font size.
const LINE: f32 = 1.4;

/// Sets `size` px text on a line box 1.4 times as high, the design's type, where GPUI's own line
/// height (about 1.618) would make rows taller than the design's.
pub(crate) fn sized<E: Styled>(element: E, size: f32) -> E {
    element.text_size(px(size)).line_height(px(size * LINE))
}

/// Tabular figures, so counts keep their width as they change.
pub(crate) fn tabular() -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".into(), 1)]))
}

#[cfg(test)]
mod tests;
