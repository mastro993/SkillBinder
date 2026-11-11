//! The app's ways into GPUI Kit's controls. Buttons and checkboxes get the hand cursor, which Kit
//! keeps only on its link and text button variants; a text button gets the design's medium label;
//! an element that is not a Kit control gets Kit's tooltip; and an agent or a skill directory gets
//! its logo, which Kit has no picker for. The submodules hold the controls the app still draws
//! itself.

pub(super) mod exact_text;

use super::library_copies::Reader;
use gpui_kit::component::{
    Disableable, Icon, Sizable, Size, button::Button, checkbox::Checkbox, tooltip::Tooltip,
};
use gpui_kit::{
    AnyElement, AnyView, App, ElementId, FocusHandle, IntoElement, ParentElement, SharedString,
    Styled, StyledImage, Window, div, img, prelude::FluentBuilder, px,
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

/// Whether `focus` has focus that a key gave it, the only focus that shows Kit's focus ring on the
/// app's own controls: a click that focuses one shows none.
pub(super) fn keyboard_focused(focus: &FocusHandle, window: &Window) -> bool {
    focus.is_focused(window) && window.last_input_was_keyboard()
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

/// What an avatar or a pill draws for an agent or a skill directory: a retained glyph, an agent's
/// monochrome logo, or its full-color logo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Logo {
    /// A retained Hugeicons glyph by its asset path, drawn in the text color.
    Glyph(&'static str),
    /// A monochrome logo by its asset path, drawn in the text color.
    Mono(&'static str),
    /// A full-color logo by its asset path, drawn in its own colors.
    Color(&'static str),
}

impl Logo {
    /// An agent's full-color logo, else its monochrome logo, else the square terminal glyph.
    pub(super) fn agent(id: &str) -> Self {
        crate::assets::agent_color_logo(id)
            .map(Self::Color)
            .or_else(|| crate::assets::agent_logo(id).map(Self::Mono))
            .unwrap_or(Self::Glyph("icons/square-terminal.svg"))
    }

    /// The `layers` glyph for Universal, otherwise the reading agent's logo.
    pub(super) fn reader(reader: &Reader) -> Self {
        match reader {
            Reader::Universal(_) => Self::Glyph("icons/layers.svg"),
            Reader::Agent(id) => Self::agent(id),
        }
    }

    pub(super) fn is_color(self) -> bool {
        matches!(self, Self::Color(_))
    }

    /// The logo at `size` px: a glyph or monochrome logo is Kit's icon in the text color it
    /// inherits; a color logo keeps its colors, or turns grayscale when `gray`.
    pub(super) fn element(self, size: f32, gray: bool) -> AnyElement {
        match self {
            Self::Glyph(path) | Self::Mono(path) => {
                Icon::empty().path(path).size(px(size)).into_any_element()
            }
            Self::Color(path) => img(path)
                .flex_none()
                .size(px(size))
                .grayscale(gray)
                .into_any_element(),
        }
    }
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
