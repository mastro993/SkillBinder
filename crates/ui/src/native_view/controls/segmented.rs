//! A segmented control: 24 px segments on a 28 px track, the chosen one raised on the page color.
//! Kit's segmented tab bar is 32 px high with wider segments and an animated indicator, so the
//! app draws its own.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    App, Div, ElementId, InteractiveElement, ParentElement, Stateful, StatefulInteractiveElement,
    Styled, div, hsla, prelude::FluentBuilder, px,
};

use super::exact_text::{exact_text, sized, tabular};
use super::logo::{Ink, Logo};
use super::shadow;
use skillbinder_theme::MEDIUM;

/// The hover group of one segment.
const GROUP: &str = "segment";
/// A segment's padding on each side.
pub(crate) const SEGMENT_PADDING: f32 = 9.;
/// The gap between a segment's label and its count.
pub(crate) const SEGMENT_GAP: f32 = 5.;
/// The track's padding around its segments, and the gap between them.
pub(crate) const TRACK_PADDING: f32 = 2.;

/// The track holding the segments.
pub(crate) fn track(id: impl Into<ElementId>, cx: &App) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .h(px(28.))
        .p(px(TRACK_PADDING))
        .gap(px(TRACK_PADDING))
        .rounded(px(8.))
        .bg(cx.theme().tab_bar_segmented)
}

/// One segment, selected for assistive technology while chosen; the caller adds its label or
/// icon, its click, and its accessible name.
pub(crate) fn segment(id: impl Into<ElementId>, chosen: bool, cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    let foreground = theme.foreground;
    sized(div().id(id), 12.5)
        .aria_selected(chosen)
        .group(GROUP)
        .flex()
        .flex_none()
        .items_center()
        .gap(px(SEGMENT_GAP))
        .h(px(24.))
        .px(px(SEGMENT_PADDING))
        .rounded(px(6.))
        .font_weight(MEDIUM)
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

/// A segment's count: muted 11 px medium tabular figures; its line is `count`.
pub(crate) fn count(value: usize, cx: &App) -> Div {
    sized(div(), 11.)
        .font_weight(MEDIUM)
        .font_features(tabular())
        .text_color(cx.theme().muted_foreground)
        .child(exact_text("count", value.to_string()))
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
