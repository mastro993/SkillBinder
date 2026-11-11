//! App buttons and checkboxes: GPUI Kit keeps the arrow cursor on them, except on link and text
//! button variants. The submodules hold the app-owned controls drawn to the approved prototypes
//! where Kit's own cannot match them pixel for pixel.

pub(super) mod avatar;
pub(super) mod chip;
pub(super) mod dialog_surface;
pub(super) mod exact_text;
pub(super) mod logo;
pub(super) mod motion;
pub(super) mod pill;
pub(super) mod popover;
pub(super) mod press;
pub(super) mod progress_bar;
pub(super) mod segmented;
pub(super) mod shadow;
pub(super) mod switch;
pub(super) mod text_button;
pub(super) mod toast;
pub(super) mod tooltip;

use gpui_kit::component::{Disableable, Icon, button::Button, checkbox::Checkbox};
use gpui_kit::{ElementId, Styled, prelude::FluentBuilder};

/// Button that shows a hand cursor while it is enabled.
pub(super) fn button(id: impl Into<ElementId>) -> Button {
    Button::new(id).cursor_pointer()
}

/// Checkbox that shows a hand cursor while it is enabled.
pub(super) fn checkbox(id: impl Into<ElementId>) -> Checkbox {
    Checkbox::new(id).cursor_pointer()
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

impl Inactive for Checkbox {
    fn inactive(self, inactive: bool) -> Self {
        self.disabled(inactive)
            .when(inactive, |checkbox| checkbox.cursor_default())
    }
}

#[cfg(test)]
mod tests;
