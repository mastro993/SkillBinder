//! Molecule: a dialog's footer, an optional note or checkbox at its start, then its secondary
//! and primary buttons at its end, 8 apart (`DESIGN.md` § Components). The primary may be a
//! danger button, for an action that removes something.

use crate::Button;
use crate::tokens::space;
use gpui_kit::{
    AnyElement, App, Div, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window,
    div, prelude::FluentBuilder, px,
};

/// A dialog's footer.
#[derive(IntoElement)]
pub struct DialogFooter {
    base: Div,
    leading: Option<AnyElement>,
    secondary: Option<Button>,
    primary: Option<Button>,
}

impl DialogFooter {
    /// An empty footer.
    pub fn new() -> Self {
        Self {
            base: div(),
            leading: None,
            secondary: None,
            primary: None,
        }
    }

    /// Starts the footer with `leading`, such as a note or a checkbox.
    pub fn leading(self, leading: impl IntoElement) -> Self {
        Self {
            leading: Some(leading.into_any_element()),
            ..self
        }
    }

    /// The secondary button, such as Cancel, before the primary.
    pub fn secondary(self, button: Button) -> Self {
        Self {
            secondary: Some(button),
            ..self
        }
    }

    /// The primary button, last: the primary variant, or danger for a removal.
    pub fn primary(self, button: Button) -> Self {
        Self {
            primary: Some(button),
            ..self
        }
    }
}

impl Default for DialogFooter {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for DialogFooter {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for DialogFooter {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        // Without a note the buttons take the width, so one can grow to fill it.
        let buttons = div()
            .flex()
            .items_center()
            .justify_end()
            .gap(px(space::MD))
            .map(|buttons| match self.leading {
                Some(_) => buttons.flex_none(),
                None => buttons.flex_1(),
            });
        self.base
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(space::LG))
            .children(
                self.leading
                    .map(|leading| div().flex_1().min_w_0().child(leading)),
            )
            .child(buttons.children(self.secondary).children(self.primary))
    }
}

#[cfg(test)]
mod tests;
