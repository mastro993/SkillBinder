//! The header's tabs, as Library's folders and Settings' sections show them: ghost buttons with
//! muted text, the chosen one on a soft muted fill, and a separator after the first.

use super::NativeView;
use gpui_kit::component::ActiveTheme;
use gpui_kit::{Context, IntoElement, ParentElement, Styled, div, px};
use skillbinder_design::tokens::space;
use skillbinder_design::{Button, FOLDER_TAB_SIZE, IconSize, Separator};

/// A tab named `name`; the caller adds what clicking it does.
pub(super) fn header_tab(
    element: impl Into<gpui_kit::ElementId>,
    name: &str,
    selected: bool,
    cx: &Context<NativeView>,
) -> Button {
    let tab = Button::new(element)
        .ghost()
        .small()
        .flex_shrink_0()
        .accessibility_label(name.to_owned())
        .child(tab_label(name))
        .selected(selected);
    // The chosen tab sits on a soft muted fill rather than the kit's darker selected fill.
    // Unchosen tabs are muted; hovering brightens them to the ghost text color.
    if selected {
        tab.bg(cx.theme().muted).text_color(cx.theme().foreground)
    } else {
        tab.text_color(cx.theme().muted_foreground)
    }
}

/// The separator between the first tab and the rest.
pub(super) fn tab_separator() -> impl IntoElement {
    Separator::vertical()
        .flex_shrink_0()
        .h(px(IconSize::Md.points()))
        .mx(px(space::XS))
}

/// A tab's text; a button's own label keeps the kit's size for its button size.
pub(super) fn tab_label(text: &str) -> impl IntoElement {
    div()
        .text_size(px(FOLDER_TAB_SIZE))
        .whitespace_nowrap()
        .child(text.to_owned())
}
