//! App buttons: GPUI Kit keeps the arrow cursor on buttons except link and text variants.

use gpui_kit::component::{Disableable, Icon, button::Button};
use gpui_kit::{ElementId, Styled, prelude::FluentBuilder};

/// Button that shows a hand cursor while it is enabled.
pub(super) fn button(id: impl Into<ElementId>) -> Button {
    Button::new(id).cursor_pointer()
}

/// Retained Hugeicons artwork mapped under `icons/` in the asset source.
pub(super) fn app_icon(name: &str) -> Icon {
    Icon::empty().path(format!("icons/{name}.svg"))
}

/// An agent's official logo, or the square terminal glyph for agents without bundled artwork.
pub(super) fn agent_icon(id: &str) -> Icon {
    crate::assets::agent_logo(id).map_or_else(
        || app_icon("square-terminal"),
        |path| Icon::empty().path(path),
    )
}

pub(super) trait Inactive {
    /// Disables the button and restores the arrow cursor while it is disabled.
    fn inactive(self, inactive: bool) -> Self;
}

impl Inactive for Button {
    fn inactive(self, inactive: bool) -> Self {
        self.disabled(inactive)
            .when(inactive, |button| button.cursor_default())
    }
}
