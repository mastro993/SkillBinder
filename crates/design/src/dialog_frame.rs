//! Organism: the frame every dialog shares, Kit's `Dialog` at one of `DESIGN.md`'s sizes, on
//! the `popover` fill with `xl` corners and `body-sm` body text, with a [`DialogHeader`] and a
//! [`DialogFooter`]. Kit's own close control is off; the header draws the close button.
//!
//! Kit hands a dialog's builder the `Dialog` it opens, so the frame is applied to it rather than
//! drawn on its own; the caller adds the body as the dialog's children, which scroll as one, or
//! as a [`DialogFrame::body`] that fills the dialog and scrolls its own parts.

use crate::tokens::{radius, text};
use crate::{DialogFooter, DialogHeader, Level};
use gpui_kit::component::{ActiveTheme, dialog::Dialog};
use gpui_kit::{AnyElement, App, IntoElement, ParentElement, Pixels, Size, Styled, px};
use std::cell::Cell;

/// How wide a dialog is (`DESIGN.md` § Layout).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DialogSize {
    /// 320 wide: a one-question prompt.
    Compact,
    /// 420 wide: a confirmation or a short list.
    Standard,
    /// 92% of the window's viewport wide and 90% tall, centered: a picker or a long list.
    Window(Size<Pixels>),
}

/// The window's share a `Window` dialog takes, across and down.
const WINDOW_WIDTH: f32 = 0.92;
const WINDOW_HEIGHT: f32 = 0.9;
/// A compact and a standard dialog's width.
const COMPACT: f32 = 320.;
const STANDARD: f32 = 420.;

/// A dialog's frame: its size, header, and footer.
pub struct DialogFrame {
    size: DialogSize,
    header: Option<DialogHeader>,
    footer: Option<DialogFooter>,
    body: Option<AnyElement>,
}

impl DialogFrame {
    /// A frame of `size`.
    pub fn new(size: DialogSize) -> Self {
        Self {
            size,
            header: None,
            footer: None,
            body: None,
        }
    }

    /// The header.
    pub fn header(self, header: DialogHeader) -> Self {
        Self {
            header: Some(header),
            ..self
        }
    }

    /// The footer.
    pub fn footer(self, footer: DialogFooter) -> Self {
        Self {
            footer: Some(footer),
            ..self
        }
    }

    /// A body that fills the dialog between its header and footer and never scrolls as a whole,
    /// so a list in it scrolls in its own area while what sits beside or below the list stays
    /// in view. Kit's children grow to their content and scroll together instead, and their
    /// scroll area clips the focus ring of a field at the top; this body keeps room for it.
    pub fn body(self, body: impl IntoElement) -> Self {
        Self {
            body: Some(body.into_any_element()),
            ..self
        }
    }

    /// Frames `dialog`.
    pub fn apply(self, dialog: Dialog, cx: &App) -> Dialog {
        let dialog = match self.size {
            DialogSize::Compact => dialog.w(px(COMPACT)),
            DialogSize::Standard => dialog.w(px(STANDARD)),
            DialogSize::Window(viewport) => dialog
                .w(viewport.width * WINDOW_WIDTH)
                .h(viewport.height * WINDOW_HEIGHT)
                .margin_top(viewport.height * ((1. - WINDOW_HEIGHT) / 2.)),
        };
        // The header's own close button replaces Kit's, which keeps the arrow cursor.
        let dialog = text::BODY_SM
            .apply(dialog.close_button(false))
            .bg(Level::Modal.fill(cx.theme()))
            .rounded(px(radius::XL));
        let dialog = match self.header {
            Some(header) => dialog.title(header),
            None => dialog,
        };
        let dialog = match self.footer {
            Some(footer) => dialog.footer(footer),
            None => dialog,
        };
        match self.body {
            // Kit runs the content builder once each time it draws the dialog, and a dialog's
            // builder makes a new frame for every draw.
            Some(body) => {
                let body = Cell::new(Some(body));
                dialog.content(move |content, _, _| content.min_h_0().children(body.take()))
            }
            None => dialog,
        }
    }
}

#[cfg(test)]
mod tests;
