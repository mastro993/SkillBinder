//! Atom: the button that closes a dialog, a notice, or a picked item.

use crate::{Button, icon};
use gpui_kit::{ElementId, SharedString};

impl Button {
    /// A small ghost button with the Hugeicons `close` glyph and the tooltip "Close", named
    /// `label` for assistive technology, such as "Close dialog" or "Deselect pdf-tools".
    pub fn close(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self::new(id)
            .ghost()
            .small()
            .icon(icon("close"))
            .accessibility_label(label)
            .tooltip("Close")
    }
}

#[cfg(test)]
mod tests;
