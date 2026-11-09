//! A segmented control: 24 px segments on a 28 px track, the chosen one raised on the page color.
//! Kit's segmented tab bar is 32 px high with wider segments and an animated indicator, so the
//! app draws its own.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    App, Div, ElementId, InteractiveElement, ParentElement, Stateful, StatefulInteractiveElement,
    Styled, div, hsla, prelude::FluentBuilder, px,
};

use super::logo::{Ink, Logo};
use super::{shadow, text};

/// The hover group of one segment.
const GROUP: &str = "segment";

/// The track holding the segments.
pub(crate) fn track(id: impl Into<ElementId>, cx: &App) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .h(px(28.))
        .p(px(2.))
        .gap(px(2.))
        .rounded(px(8.))
        .bg(cx.theme().tab_bar_segmented)
}

/// One segment, selected for assistive technology while chosen; the caller adds its label or
/// icon, its click, and its accessible name.
pub(crate) fn segment(id: impl Into<ElementId>, chosen: bool, cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    let foreground = theme.foreground;
    text::sized(div().id(id), 12.5)
        .aria_selected(chosen)
        .group(GROUP)
        .flex()
        .flex_none()
        .items_center()
        .gap(px(5.))
        .h(px(24.))
        .px(px(9.))
        .rounded(px(6.))
        .font_weight(text::MEDIUM)
        .cursor_pointer()
        .when(chosen, |segment| {
            segment
                .bg(theme.background)
                .text_color(foreground)
                // CSS `0 1px 2px #00000014, 0 0 0 1px fg@6%`, listed bottom first.
                .shadow(vec![
                    shadow::css(0., 0., 0., 1., foreground.alpha(0.06)),
                    shadow::css(0., 1., 2., 0., hsla(0., 0., 0., 0.08)),
                ])
        })
        .when(!chosen, |segment| {
            segment
                .text_color(theme.muted_foreground)
                .hover(move |style| style.text_color(foreground))
        })
}

/// A segment's count: muted 11 px tabular figures.
pub(crate) fn count(value: usize, cx: &App) -> Div {
    text::sized(div(), 11.)
        .font_features(text::tabular())
        .text_color(cx.theme().muted_foreground)
        .child(value.to_string())
}

/// A segment's 15 px glyph: the text color while chosen, else muted until the segment is hovered.
pub(crate) fn glyph(name: &'static str, chosen: bool, cx: &App) -> gpui_kit::AnyElement {
    let theme = cx.theme();
    let ink = if chosen {
        Ink::from(theme.foreground)
    } else {
        Ink {
            rest: theme.muted_foreground,
            hover: Some((GROUP, theme.foreground)),
        }
    };
    Logo::Glyph(name).element(15., ink, false)
}
