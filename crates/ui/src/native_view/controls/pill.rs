//! A 26 by 24 icon pill toggling one skill directory. Off, it has a dashed outline and a muted,
//! grayscale logo at half opacity; on, a solid outline in the text color on the page fill and the
//! logo in its own colors. Hover adds the muted fill.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    App, Div, ElementId, InteractiveElement, ParentElement, Stateful, Styled, div,
    prelude::FluentBuilder, px,
};

use super::dashed;
use super::logo::{Ink, Logo};

/// The hover group of one pill.
const GROUP: &str = "pill";
/// The dashed outline's ink.
pub(crate) const DASHED_INK: f32 = 0.24;

/// The pill; the caller adds its click, tooltip, and accessible name.
pub(crate) fn pill(id: impl Into<ElementId>, logo: Logo, on: bool, cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    let (foreground, muted, hover) = (theme.foreground, theme.muted_foreground, theme.muted);
    let ink = if on {
        Ink::from(foreground)
    } else {
        Ink {
            rest: muted.opacity(0.5),
            hover: Some((GROUP, foreground.opacity(0.5))),
        }
    };
    let mark = div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(16.))
        .when(!on && logo.is_color(), |mark| mark.opacity(0.5))
        .child(logo.element(15., ink, !on));
    div()
        .id(id)
        .group(GROUP)
        .relative()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .w(px(26.))
        .h(px(24.))
        .rounded_full()
        .when(on, |pill| {
            pill.border_1()
                .border_color(foreground)
                .bg(theme.background)
        })
        .when(!on, |pill| {
            pill.child(dashed::outline(foreground.alpha(DASHED_INK)))
        })
        .hover(move |style| style.bg(hover))
        .child(mark)
}
