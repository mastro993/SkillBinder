//! Molecule: an inline name field, Kit's borderless input in a filled box the shape of what it
//! names, so the edited name stays where the name was. Escape cancels it.
//!
//! An input's own focus ring would draw outside the box and be clipped by a scrolling strip, so
//! the box is the field's only outline.

use crate::tokens::{radius, space, text};
use gpui_kit::component::{
    ActiveTheme, Sizable,
    input::{Input, InputState},
};
use gpui_kit::{
    AnyElement, App, Div, ElementId, Entity, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, RenderOnce, Stateful, StyleRefinement, Styled, Window, div, px,
};
use std::rc::Rc;

/// What a name field sits in place of.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NameFieldSize {
    /// A small tab: 24 high, `body-sm`, on the `muted` fill.
    #[default]
    Sm,
    /// A sidebar row: 32 high, `body-md`, on the selected row's `sidebar-accent` fill.
    Md,
}

impl NameFieldSize {
    /// Both sizes.
    pub const ALL: [Self; 2] = [Self::Sm, Self::Md];
}

/// Runs when Escape cancels the edit.
type Cancel = Rc<dyn Fn(&mut Window, &mut App)>;

/// A field editing a name in place.
#[derive(IntoElement)]
pub struct InlineNameField {
    base: Stateful<Div>,
    state: Entity<InputState>,
    size: NameFieldSize,
    leading: Option<AnyElement>,
    on_cancel: Option<Cancel>,
}

impl InlineNameField {
    /// The field `id` editing `state`.
    pub fn new(id: impl Into<ElementId>, state: &Entity<InputState>) -> Self {
        Self {
            base: div().id(id),
            state: state.clone(),
            size: NameFieldSize::default(),
            leading: None,
            on_cancel: None,
        }
    }

    /// Sets the size.
    pub fn size(self, size: NameFieldSize) -> Self {
        Self { size, ..self }
    }

    /// Shows `leading` before the name, such as the row's icon.
    pub fn leading(self, leading: impl IntoElement) -> Self {
        Self {
            leading: Some(leading.into_any_element()),
            ..self
        }
    }

    /// Runs `cancel` when Escape is pressed in the field.
    pub fn on_cancel(self, cancel: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        Self {
            on_cancel: Some(Rc::new(cancel)),
            ..self
        }
    }
}

impl Styled for InlineNameField {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for InlineNameField {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (height, corner, fill, level) = match self.size {
            NameFieldSize::Sm => (space::CONTROL_SM, radius::SM, theme.muted, text::BODY_SM),
            NameFieldSize::Md => (
                space::SIDEBAR_ROW,
                radius::MD,
                theme.sidebar_accent,
                text::BODY_MD,
            ),
        };
        let cancel = self.on_cancel;
        self.base
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap(px(space::MD))
            .h(px(height))
            .px(px(space::MD))
            .rounded(px(corner))
            .bg(fill)
            .children(self.leading)
            .child(
                div().flex_1().min_w_0().child(
                    // Unpadded, so the edited name starts where the name did.
                    Input::new(&self.state)
                        .appearance(false)
                        .small()
                        .px_0()
                        .text_size(px(level.size)),
                ),
            )
            .capture_key_down(move |event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape"
                    && let Some(cancel) = &cancel
                {
                    cancel(window, cx);
                    cx.stop_propagation();
                }
            })
    }
}

#[cfg(test)]
mod tests;
