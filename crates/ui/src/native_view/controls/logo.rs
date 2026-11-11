//! What an avatar or a pill draws for an agent or a skill directory: a retained glyph, an agent's
//! monochrome logo in the text color, or its full-color logo in its own colors.

use super::super::library_copies::Reader;
use gpui_kit::{
    AnyElement, Hsla, InteractiveElement, IntoElement, Styled, StyledImage, img,
    prelude::FluentBuilder, px, svg,
};

/// The text color a glyph or monochrome logo takes, and the one it takes while the named group is
/// hovered.
#[derive(Clone, Copy)]
pub(crate) struct Ink {
    pub rest: Hsla,
    pub hover: Option<(&'static str, Hsla)>,
}

impl From<Hsla> for Ink {
    fn from(rest: Hsla) -> Self {
        Self { rest, hover: None }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Logo {
    /// A retained Hugeicons glyph by its asset path, drawn in the text color.
    Glyph(&'static str),
    /// A monochrome logo by its asset path, drawn in the text color.
    Mono(&'static str),
    /// A full-color logo by its asset path, drawn in its own colors.
    Color(&'static str),
}

impl Logo {
    /// An agent's full-color logo, else its monochrome logo, else the square terminal glyph.
    pub(crate) fn agent(id: &str) -> Self {
        crate::assets::agent_color_logo(id)
            .map(Self::Color)
            .or_else(|| crate::assets::agent_logo(id).map(Self::Mono))
            .unwrap_or(Self::Glyph("icons/square-terminal.svg"))
    }

    /// The `layers` glyph for Universal, otherwise the reading agent's logo.
    pub(crate) fn reader(reader: &Reader) -> Self {
        match reader {
            Reader::Universal(_) => Self::Glyph("icons/layers.svg"),
            Reader::Agent(id) => Self::agent(id),
        }
    }

    pub(crate) fn is_color(self) -> bool {
        matches!(self, Self::Color(_))
    }

    /// The logo at `size` px. A glyph or monochrome logo takes `ink`; `gray` draws a color logo
    /// in grayscale.
    pub(crate) fn element(self, size: f32, ink: Ink, gray: bool) -> AnyElement {
        match self {
            Self::Glyph(path) | Self::Mono(path) => svg()
                .path(path)
                .flex_none()
                .size(px(size))
                .text_color(ink.rest)
                .when_some(ink.hover, |logo, (group, color)| {
                    logo.group_hover(group, move |style| style.text_color(color))
                })
                .into_any_element(),
            Self::Color(path) => img(path)
                .flex_none()
                .size(px(size))
                .grayscale(gray)
                .into_any_element(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agents_take_their_color_logo_then_their_monochrome_one_then_the_terminal_glyph() {
        assert_eq!(
            Logo::agent("claude-code"),
            Logo::Color("agents/claude-code-color.svg")
        );
        assert_eq!(Logo::agent("cursor"), Logo::Mono("agents/cursor.svg"));
        assert_eq!(
            Logo::agent("dexto"),
            Logo::Glyph("icons/square-terminal.svg")
        );
        assert_eq!(
            Logo::reader(&Reader::Universal(Vec::new())),
            Logo::Glyph("icons/layers.svg")
        );
    }
}
