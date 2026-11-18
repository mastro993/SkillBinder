//! Molecule: a section header, a `caption-sm` muted label with an optional count and a trailing
//! action.
//!
//! Its container sets its padding, such as a sidebar section's or a dialog group's.

use crate::Text;
use crate::tokens::{space, text};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    AnyElement, App, Div, ElementId, IntoElement, ParentElement, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, div, px,
};

/// What a header reads.
enum Label {
    /// A label of its own.
    Text(ElementId, SharedString),
    /// An element drawn in the header's type, such as a title a click folds.
    Element(AnyElement),
}

/// A header above a group of rows: its label, then its count, then its action at the end.
#[derive(IntoElement)]
pub struct SectionHeader {
    base: Div,
    label: Label,
    count: Option<usize>,
    action: Option<AnyElement>,
}

impl SectionHeader {
    /// The header reading `label`, a label of its own, `id`.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self::with(Label::Text(id.into(), label.into()))
    }

    /// The header around `label`, an element that takes the header's type and tone.
    pub fn custom(label: impl IntoElement) -> Self {
        Self::with(Label::Element(label.into_any_element()))
    }

    fn with(label: Label) -> Self {
        Self {
            base: div(),
            label,
            count: None,
            action: None,
        }
    }

    /// Shows `count` after the label.
    pub fn count(self, count: usize) -> Self {
        Self {
            count: Some(count),
            ..self
        }
    }

    /// Ends the header with `action`, such as an add button.
    pub fn action(self, action: impl IntoElement) -> Self {
        Self {
            action: Some(action.into_any_element()),
            ..self
        }
    }
}

impl Styled for SectionHeader {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for SectionHeader {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let label = match self.label {
            Label::Text(id, label) => Text::labeled(id, label).into_any_element(),
            Label::Element(label) => label,
        };
        text::CAPTION_SM
            .apply(self.base)
            .flex()
            .items_center()
            .gap(px(space::XS))
            .text_color(cx.theme().muted_foreground)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(space::SM))
                    .flex_1()
                    .min_w_0()
                    .child(label)
                    .children(
                        self.count
                            .map(|count| Text::labeled_count("count", count).flex_none()),
                    ),
            )
            .children(self.action)
    }
}

#[cfg(test)]
mod tests;
