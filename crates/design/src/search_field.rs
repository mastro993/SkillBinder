//! Molecule: a search field, Kit's input on the `muted` fill after a `Sm` search glyph.
//!
//! What it shows while empty is its input state's placeholder, which the state owns.

use crate::{IconSize, icon};
use gpui_kit::component::{
    ActiveTheme, Sizable,
    input::{Input, InputState},
};
use gpui_kit::{
    App, Div, ElementId, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Stateful, StyleRefinement, Styled, Window, div, prelude::FluentBuilder, px,
};

/// A search field over `state`, named `Search` unless [`SearchField::accessibility_label`]
/// names it.
#[derive(IntoElement)]
pub struct SearchField {
    base: Stateful<Div>,
    state: Entity<InputState>,
    width: Option<f32>,
    label: SharedString,
}

impl SearchField {
    /// The field `id` editing `state`. It takes its container's width until
    /// [`SearchField::width`] sets one.
    pub fn new(id: impl Into<ElementId>, state: &Entity<InputState>) -> Self {
        Self {
            base: div().id(id),
            state: state.clone(),
            width: None,
            label: "Search".into(),
        }
    }

    /// A fixed width in points, never wider than its container.
    pub fn width(self, width: f32) -> Self {
        Self {
            width: Some(width),
            ..self
        }
    }

    /// Names the field for assistive technology.
    pub fn accessibility_label(self, label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            ..self
        }
    }
}

impl Styled for SearchField {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for SearchField {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        self.base
            .when_some(self.width, |field, width| {
                field.flex_none().w(px(width)).max_w_full()
            })
            .child(
                Input::new(&self.state)
                    .aria_label(self.label)
                    .prefix(
                        icon("search")
                            .with_size(IconSize::Sm)
                            .text_color(theme.muted_foreground),
                    )
                    .bg(theme.muted),
            )
    }
}

#[cfg(test)]
mod tests;
