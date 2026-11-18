//! Atom: a line of text at a type level, in a tone, optionally with tabular figures.
//!
//! A line that an enclosing element's accessible name does not read is [`Text::labeled`], GPUI
//! text with an id, which makes it a label holding its text and lets the native-test bridge name
//! it; a line such a name reads is [`Text::new`], a plain string, so assistive technology reads
//! it once.

use crate::{Tone, TypeLevel};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    App, Div, ElementId, FontFeatures, IntoElement, ParentElement, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder,
};
use std::sync::Arc;

/// The color a line of text takes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextTone {
    /// The enclosing element's color, `foreground` unless a row or a control sets its own.
    #[default]
    Default,
    /// `muted-foreground`: descriptions, details, counts, section headers.
    Muted,
    /// A status `-text` tone, such as [`crate::tokens::color::WARNING_TEXT`].
    Status(Tone),
}

/// What a line holds.
enum Content {
    Plain(SharedString),
    Labeled(ElementId, SharedString),
}

/// A line of text: a level's size, weight, and line height, a tone, and tabular figures for
/// numbers that change.
#[derive(IntoElement)]
pub struct Text {
    base: Div,
    content: Content,
    level: Option<TypeLevel>,
    tone: TextTone,
    tabular: bool,
}

impl Text {
    /// `text` as a plain string, which an enclosing accessible name reads. It takes the
    /// enclosing element's type, such as a chip's or a button's, until [`Text::level`] sets one.
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self::with(Content::Plain(text.into()))
    }

    /// `text` as a label of its own, `id`, for a line no enclosing name reads.
    pub fn labeled(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self::with(Content::Labeled(id.into(), text.into()))
    }

    fn with(content: Content) -> Self {
        Self {
            base: div(),
            content,
            level: None,
            tone: TextTone::Default,
            tabular: false,
        }
    }

    /// Sets the type level.
    pub fn level(self, level: TypeLevel) -> Self {
        Self {
            level: Some(level),
            ..self
        }
    }

    /// Sets the tone.
    pub fn tone(self, tone: TextTone) -> Self {
        Self { tone, ..self }
    }

    /// `muted-foreground`.
    pub fn muted(self) -> Self {
        self.tone(TextTone::Muted)
    }

    /// Tabular figures, so a number keeps its width as it changes.
    pub fn tabular(self) -> Self {
        Self {
            tabular: true,
            ..self
        }
    }
}

/// Tabular figures, so counts keep their width as they change.
pub fn tabular() -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".into(), 1)]))
}

impl Styled for Text {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Text {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let color = match self.tone {
            TextTone::Default => None,
            TextTone::Muted => Some(theme.muted_foreground),
            TextTone::Status(tone) => Some(tone.of(theme)),
        };
        let line = self
            .base
            .when_some(self.level, |line, level| {
                level.apply(line).when(level.mono, |line| {
                    line.font_family(theme.mono_font_family.clone())
                })
            })
            .when_some(color, |line, color| line.text_color(color))
            .when(self.tabular, |line| line.font_features(tabular()));
        match self.content {
            Content::Plain(text) => line.child(text),
            Content::Labeled(id, text) => line.child(gpui_kit::Text::new(id, text)),
        }
    }
}

#[cfg(test)]
mod tests;
