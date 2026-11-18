//! Atom: a status glyph, an icon in a status `-text` tone with a tooltip saying what it marks.

use crate::tokens::color;
use crate::{IconSize, Tone, icon, tooltip};
use gpui_kit::component::{ActiveTheme, Sizable};
use gpui_kit::{
    App, Div, ElementId, InteractiveElement, Interactivity, IntoElement, ParentElement, RenderOnce,
    Role, SharedString, Stateful, StatefulInteractiveElement, StyleRefinement, Styled, Window, div,
};

/// What a status glyph marks, which sets its tone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Something new, such as an update: `info-text`.
    Info,
    /// Something to look at: `warning-text`.
    Warning,
    /// Something broken: `danger-text`.
    Danger,
    /// Something done: `success-text`.
    Success,
}

impl Status {
    /// Every status.
    pub const ALL: [Self; 4] = [Self::Info, Self::Warning, Self::Danger, Self::Success];

    /// The tone of the status's text and glyphs.
    pub fn text(self) -> Tone {
        match self {
            Self::Info => color::INFO_TEXT,
            Self::Warning => color::WARNING_TEXT,
            Self::Danger => color::DANGER_TEXT,
            Self::Success => color::SUCCESS_TEXT,
        }
    }
}

/// A `Sm` glyph in its status's tone. The tooltip's first line is its accessible name.
#[derive(IntoElement)]
pub struct StatusGlyph {
    base: Stateful<Div>,
    status: Status,
    glyph: SharedString,
    tooltip: SharedString,
}

impl StatusGlyph {
    /// The glyph `name`, `id`, marking `status`, explained by `tooltip`.
    pub fn new(
        id: impl Into<ElementId>,
        status: Status,
        name: impl Into<SharedString>,
        tooltip: impl Into<SharedString>,
    ) -> Self {
        Self {
            base: div().id(id),
            status,
            glyph: name.into(),
            tooltip: tooltip.into(),
        }
    }
}

impl Styled for StatusGlyph {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for StatusGlyph {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for StatusGlyph {}

impl RenderOnce for StatusGlyph {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let name = self.tooltip.lines().next().unwrap_or_default().to_owned();
        self.base
            .flex_shrink_0()
            .role(Role::Image)
            .aria_label(name)
            .child(
                icon(&self.glyph)
                    .with_size(IconSize::Sm)
                    .text_color(self.status.text().of(cx.theme())),
            )
            .tooltip(tooltip(self.tooltip))
    }
}

#[cfg(test)]
mod tests;
