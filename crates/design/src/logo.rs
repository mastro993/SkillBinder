//! Atom: what an avatar or a pill draws for an agent or a skill directory.

use crate::IconSize;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::{AnyElement, IntoElement, Styled, StyledImage, img, px};

/// A retained glyph, an agent's monochrome logo, or its full-color logo, by asset path. The app
/// picks an agent's logo; this draws it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Logo {
    /// A retained Hugeicons glyph by its asset path, drawn in the text color.
    Glyph(&'static str),
    /// A monochrome logo by its asset path, drawn in the text color.
    Mono(&'static str),
    /// A full-color logo by its asset path, drawn in its own colors.
    Color(&'static str),
}

impl Logo {
    /// Whether the logo keeps its own colors.
    pub fn is_color(self) -> bool {
        matches!(self, Self::Color(_))
    }

    /// The logo at `size`: a glyph or monochrome logo is Kit's icon in the text color it
    /// inherits; a color logo keeps its colors, or turns grayscale when `gray`.
    pub fn element(self, size: IconSize, gray: bool) -> AnyElement {
        match self {
            Self::Glyph(path) | Self::Mono(path) => {
                Icon::empty().path(path).with_size(size).into_any_element()
            }
            Self::Color(path) => img(path)
                .flex_none()
                .size(px(size.points()))
                .grayscale(gray)
                .into_any_element(),
        }
    }
}

#[cfg(test)]
mod tests;
