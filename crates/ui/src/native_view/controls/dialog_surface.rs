//! The app's dialog surface: a scrim over the whole window and a dialog 56 px from its top edge,
//! centered across it, with a title row, a body, and a footer. Kit `Dialog` hard-codes a shadow
//! twice as soft, so the app draws the dialogs its design draws exactly.

use super::{app_icon, shadow};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    App, Div, ElementId, FontWeight, InteractiveElement, ParentElement, Role, SharedString,
    Stateful, StatefulInteractiveElement, Styled, div, px, transparent_black,
};

/// The scrim with id `id`, which takes every pointer event over the window and holds `dialog`.
pub(crate) fn scrim(id: impl Into<ElementId>, dialog: Div, cx: &App) -> Stateful<Div> {
    div()
        .id(id)
        .absolute()
        .inset_0()
        .occlude()
        .flex()
        .justify_center()
        .items_start()
        .pt(px(56.))
        .px(px(16.))
        .pb(px(16.))
        .bg(cx.theme().overlay)
        .child(dialog)
}

/// A `width` dialog: the page background, a `border` hairline, radius 8, and the large shadow.
pub(crate) fn surface(width: f32, cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .w(px(width))
        .max_w_full()
        .border_1()
        .border_color(theme.border)
        .rounded(px(8.))
        .bg(theme.background)
        .text_color(theme.foreground)
        .text_size(px(13.))
        .line_height(px(18.2))
        .shadow(shadow::large())
}

/// The title row: `title` at 15 px semibold, then `close`.
pub(crate) fn header(title: impl Into<SharedString>, close: Stateful<Div>) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .pt(px(14.))
        .px(px(16.))
        .pb(px(6.))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(15.))
                .line_height(px(21.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(title.into()),
        )
        .child(close)
}

/// The body under the title row.
pub(crate) fn body() -> Div {
    div().flex().flex_col().pt(px(4.)).px(px(16.)).pb(px(12.))
}

/// The footer under a body: its buttons in a row from the left.
pub(crate) fn footer() -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .pt(px(4.))
        .px(px(16.))
        .pb(px(16.))
}

/// The title row's close button: a 24 px square with the `close` glyph in the muted color, which
/// takes the muted fill and the text color under the pointer.
pub(crate) fn close_button(id: impl Into<ElementId>, cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    let (muted, fill, ink) = (theme.muted_foreground, theme.muted, theme.foreground);
    div()
        .id(id)
        .role(Role::Button)
        .aria_label("Close")
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(24.))
        .border_1()
        .border_color(transparent_black())
        .rounded(px(6.))
        .text_color(muted)
        .cursor_pointer()
        .hover(move |style| style.bg(fill).text_color(ink))
        .child(app_icon("close").size(px(16.)))
}
