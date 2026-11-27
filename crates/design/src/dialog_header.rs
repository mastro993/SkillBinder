//! Molecule: a dialog's header, its `title-md` title, an optional `body-sm` muted description
//! or a detail line of any content under it, and the close button at its end.
//!
//! Kit's own close control keeps the arrow cursor, so a dialog turns it off and its header draws
//! [`Button::close`] in its place.

use crate::tokens::{space, text};
use crate::{Button, Text};
use gpui_kit::{
    AnyElement, App, ClickEvent, ElementId, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div,
};
use gpui_kit::{component::ActiveTheme, px};

/// What a header's title reads.
enum Title {
    /// A label of its own, `title`.
    Text(SharedString),
    /// An element in the title's type, such as text followed by a place's chip.
    Element(AnyElement),
}

/// A dialog's header.
#[derive(IntoElement)]
pub struct DialogHeader {
    title: Title,
    description: Option<AnyElement>,
    close: Option<Button>,
}

impl DialogHeader {
    /// The header titled `title`, a label of its own, `title`.
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self::with(Title::Text(title.into()))
    }

    /// The header around `title`, an element that takes the title's type.
    pub fn custom(title: impl IntoElement) -> Self {
        Self::with(Title::Element(title.into_any_element()))
    }

    fn with(title: Title) -> Self {
        Self {
            title,
            description: None,
            close: None,
        }
    }

    /// Shows `description` under the title.
    pub fn description(self, description: impl Into<SharedString>) -> Self {
        let description = Text::labeled("description", description.into())
            .level(text::BODY_SM)
            .muted();
        self.detail(description)
    }

    /// Shows `detail` under the title in place of a description, such as a line mixing glyphs,
    /// mono text, and status tones.
    pub fn detail(self, detail: impl IntoElement) -> Self {
        Self {
            description: Some(detail.into_any_element()),
            ..self
        }
    }

    /// Ends the header with the close button `id`, named `label`, which runs `on_click`.
    pub fn close(
        self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            close: Some(Button::close(id, label).on_click(on_click)),
            ..self
        }
    }
}

impl RenderOnce for DialogHeader {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let title = match self.title {
            Title::Text(title) => Text::labeled("title", title).into_any_element(),
            Title::Element(title) => title,
        };
        div()
            .w_full()
            .flex()
            .items_start()
            .justify_between()
            .gap(px(space::MD))
            .text_color(cx.theme().foreground)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(px(space::XS))
                    .child(text::TITLE_MD.apply(div()).child(title))
                    .children(self.description),
            )
            .children(self.close)
    }
}

#[cfg(test)]
mod tests;
