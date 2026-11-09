//! Chips, tags, dots, and the small outline button: Kit's tag and small button draw other sizes,
//! paddings, radii, and text sizes than the prototypes, so the app draws its own.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    App, Div, ElementId, Hsla, InteractiveElement, ParentElement, SharedString, Stateful, Styled,
    div, px,
};

use super::text;

/// A rounded chip `height` high: hairline border, page fill, 12 px medium text. The caller sets
/// its padding and gap.
pub(crate) fn chip(id: impl Into<ElementId>, height: f32, cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    text::sized(div().id(id), 12.)
        .flex()
        .flex_none()
        .items_center()
        .h(px(height))
        .border_1()
        .border_color(theme.border)
        .rounded_full()
        .bg(theme.background)
        .font_weight(text::MEDIUM)
        .text_color(theme.foreground)
        .whitespace_nowrap()
}

/// A folder tag: 18 px high, 11 px medium text on the secondary fill.
pub(crate) fn tag(label: impl Into<SharedString>, cx: &App) -> Div {
    let theme = cx.theme();
    text::sized(div(), 11.)
        .flex()
        .flex_none()
        .items_center()
        .h(px(18.))
        .px(px(6.))
        .rounded(px(4.))
        .bg(theme.secondary)
        .font_weight(text::MEDIUM)
        .text_color(theme.foreground)
        .whitespace_nowrap()
        .child(label.into())
}

/// An 8 px dot in `color`.
pub(crate) fn dot(color: Hsla) -> Div {
    div().flex_none().size(px(8.)).rounded_full().bg(color)
}

/// A small outline button: 24 px high, 12 px medium text, on the page fill; muted while hovered.
pub(crate) fn outline_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    cx: &App,
) -> Stateful<Div> {
    let theme = cx.theme();
    let hover = theme.muted;
    text::sized(div().id(id), 12.)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .h(px(24.))
        .px(px(8.))
        .border_1()
        .border_color(theme.border)
        .rounded(px(6.))
        .bg(theme.background)
        .font_weight(text::MEDIUM)
        .text_color(theme.foreground)
        .whitespace_nowrap()
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .child(label.into())
}
