//! App buttons and checkboxes: GPUI Kit keeps the arrow cursor on them, except on link and text
//! button variants. The submodules hold the app-owned controls drawn to the approved prototypes
//! where Kit's own cannot match them pixel for pixel.

pub(super) mod avatar;
pub(super) mod dialog_surface;
pub(super) mod exact_text;
pub(super) mod logo;
pub(super) mod motion;
pub(super) mod pill;
pub(super) mod popover;
pub(super) mod press;
pub(super) mod progress_bar;
pub(super) mod shadow;
pub(super) mod switch;
pub(super) mod toast;

use gpui_kit::component::{
    Disableable, Icon, Sizable, Size, button::Button, checkbox::Checkbox, tooltip::Tooltip,
};
use gpui_kit::{
    AnyView, App, ElementId, ParentElement, SharedString, Styled, Window, div,
    prelude::FluentBuilder, px,
};
use skillbinder_theme::MEDIUM;

/// Button that shows a hand cursor while it is enabled.
pub(super) fn button(id: impl Into<ElementId>) -> Button {
    Button::new(id).cursor_pointer()
}

/// A `size` button reading `label` in medium weight, 12 px at Kit's small size and 13 px at the
/// others. Kit sets the size of its own label on a row it does not expose, so the text is a child;
/// the label is the button's accessible name, which the text does not repeat.
pub(super) fn text_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    size: Size,
) -> Button {
    let label = label.into();
    let text = if matches!(size, Size::Small) {
        12.
    } else {
        13.
    };
    button(id)
        .with_size(size)
        .accessibility_label(label.clone())
        .child(div().text_size(px(text)).font_weight(MEDIUM).child(label))
}

/// Checkbox that shows a hand cursor while it is enabled.
pub(super) fn checkbox(id: impl Into<ElementId>) -> Checkbox {
    Checkbox::new(id).cursor_pointer()
}

/// Kit's tooltip reading `text`, one line per `\n`, for GPUI's `tooltip` on an element that is
/// not a Kit control: it shows below and right of the pointer once the pointer rests on the
/// element for GPUI's delay.
pub(super) fn tooltip(
    text: impl Into<SharedString>,
) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text = text.into();
    move |window, cx| Tooltip::new(text.clone()).build(window, cx)
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
