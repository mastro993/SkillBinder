//! Text buttons drawn as the app's design draws them. Kit `Button` sets its label size on an inner
//! row it does not expose (14 px at small, with no weight) and keeps its own outline colors and
//! hover, so the app draws these itself.

use super::exact_text::exact_text;
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    App, Div, ElementId, InteractiveElement, ParentElement, Role, SharedString, Stateful,
    StatefulInteractiveElement, Styled, div, px, transparent_black,
};
use skillbinder_theme::MEDIUM;

/// How a text button is filled.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Look {
    /// `primary` fill with its foreground text.
    Primary,
    /// The page background with a `border` hairline.
    Outline,
}

/// How large a text button is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Height {
    /// 24 high, 8 px sides, 12 px text: buttons inside a row or a pill.
    Small,
    /// 28 high, 10 px sides, 13 px text: dialog footers.
    Regular,
}

impl Height {
    /// Height, side padding, and text size.
    fn metrics(self) -> (f32, f32, f32) {
        match self {
            Self::Small => (24., 8., 12.),
            Self::Regular => (28., 10., 13.),
        }
    }
}

/// A `look` button reading `label`, `height` high, with a 1 px border, radius 6, medium text on a
/// line as tall as the text, and a hand cursor. The label is its accessible name, which the label
/// line does not repeat.
pub(crate) fn text_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    look: Look,
    height: Height,
    cx: &App,
) -> Stateful<Div> {
    let theme = cx.theme();
    let (size, side, text) = height.metrics();
    let (fill, ink, edge, hover) = match look {
        Look::Primary => (
            theme.primary,
            theme.primary_foreground,
            transparent_black(),
            theme.background.blend(theme.primary.opacity(0.86)),
        ),
        Look::Outline => (
            theme.background,
            theme.foreground,
            theme.border,
            theme.muted,
        ),
    };
    let label = label.into();
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label.clone())
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .h(px(size))
        .px(px(side))
        .border_1()
        .border_color(edge)
        .rounded(px(6.))
        .bg(fill)
        .text_color(ink)
        .text_size(px(text))
        .line_height(px(text))
        .font_weight(MEDIUM)
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .child(exact_text("label", label).presentational())
}
