//! App buttons and checkboxes: GPUI Kit keeps the arrow cursor on them, except on link and text
//! button variants.

use gpui_kit::component::{
    ActiveTheme, Disableable, Icon, avatar::Avatar, button::Button, checkbox::Checkbox,
};
use gpui_kit::{App, ElementId, Styled, prelude::FluentBuilder, white};

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

/// An agent's avatar: its full-color logo on white, else its monochrome logo or the square
/// terminal glyph.
pub(super) fn agent_avatar(id: &str, cx: &App) -> Avatar {
    match crate::assets::agent_color_logo(id) {
        Some(path) => Avatar::new().src(path).bg(white()).p_px(),
        None => glyph_avatar(agent_icon(id), cx),
    }
}

/// An avatar showing a glyph in the theme's text color.
pub(super) fn glyph_avatar(icon: Icon, cx: &App) -> Avatar {
    Avatar::new()
        .placeholder(icon)
        .bg(cx.theme().muted)
        .text_color(cx.theme().foreground)
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
