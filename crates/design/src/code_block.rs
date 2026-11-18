//! Molecule: a code block, a command in `mono-sm` on the `muted` fill with Kit's clipboard
//! button, Copy, which turns to a check for a moment once it has copied.

use crate::Text;
use crate::tokens::{radius, space, text};
use gpui_kit::component::{ActiveTheme, Sizable, Size, clipboard::Clipboard};
use gpui_kit::{
    App, Div, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    Stateful, StyleRefinement, Styled, Window, div, px,
};
use std::rc::Rc;

/// Runs after Copy put the text on the clipboard.
type Copied = Rc<dyn Fn(&mut Window, &mut App)>;

/// A command to copy, such as a package manager's install line.
#[derive(IntoElement)]
pub struct CodeBlock {
    base: Stateful<Div>,
    text: SharedString,
    on_copy: Option<Copied>,
}

impl CodeBlock {
    /// The block `id` showing `text`. Its Copy button is `id` too, inside the block.
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            base: div().id(id),
            text: text.into(),
            on_copy: None,
        }
    }

    /// Runs `copied` after Copy put the text on the clipboard.
    pub fn on_copy(self, copied: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        Self {
            on_copy: Some(Rc::new(copied)),
            ..self
        }
    }
}

impl Styled for CodeBlock {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for CodeBlock {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let mut copy = Clipboard::new("copy")
            .label("Copy")
            .value(self.text.clone())
            .with_size(Size::Small);
        if let Some(copied) = self.on_copy {
            copy = copy.on_copied(move |_, window, cx| copied(window, cx));
        }
        self.base
            .flex()
            .items_center()
            .gap(px(space::MD))
            .pl(px(space::LG))
            .pr(px(space::XS))
            .py(px(space::XS))
            .rounded(px(radius::MD))
            .bg(theme.muted)
            .text_color(theme.foreground)
            .child(
                Text::labeled("command", self.text)
                    .level(text::MONO_SM)
                    .flex_1()
                    .min_w_0(),
            )
            .child(copy)
    }
}

#[cfg(test)]
mod tests;
