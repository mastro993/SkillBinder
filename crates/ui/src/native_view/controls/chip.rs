//! Chips, tags, and dots: Kit's tag draws another size, padding, radius, and text size than the
//! prototypes, so the app draws its own. The small outline button is a text button.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    App, Div, ElementId, Hsla, InteractiveElement, ParentElement, SharedString, Stateful, Styled,
    div, px,
};
use skillbinder_theme::MEDIUM;

use super::exact_text::{exact_text, sized};

/// A rounded chip `height` high: hairline border, page fill, 12 px medium text. The caller sets
/// its padding and gap.
pub(crate) fn chip(id: impl Into<ElementId>, height: f32, cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    sized(div().id(id), 12.)
        .flex()
        .flex_none()
        .items_center()
        .h(px(height))
        .border_1()
        .border_color(theme.border)
        .rounded_full()
        .bg(theme.background)
        .font_weight(MEDIUM)
        .text_color(theme.foreground)
}

/// A folder tag: 18 px high, 11 px medium text on the secondary fill; its line is `tag`.
pub(crate) fn tag(label: impl Into<SharedString>, cx: &App) -> Div {
    let theme = cx.theme();
    sized(div(), 11.)
        .flex()
        .flex_none()
        .items_center()
        .h(px(18.))
        .px(px(6.))
        .rounded(px(4.))
        .bg(theme.secondary)
        .font_weight(MEDIUM)
        .text_color(theme.foreground)
        .child(exact_text("tag", label))
}

/// An 8 px dot in `color`.
pub(crate) fn dot(color: Hsla) -> Div {
    div().flex_none().size(px(8.)).rounded_full().bg(color)
}
