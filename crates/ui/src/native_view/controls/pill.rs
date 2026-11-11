//! A 26 by 24 icon pill toggling one skill directory. Off, it has a dashed outline and a muted,
//! grayscale logo at half opacity; on, a solid outline in the text color on the page fill and the
//! logo in its own colors. Hover adds the muted fill, unless the pill is inert because a press
//! would do nothing.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    App, Div, ElementId, InteractiveElement, ParentElement, Stateful, Styled, div,
    prelude::FluentBuilder, px,
};

use super::logo::{Ink, Logo};

/// The hover group of one pill.
const GROUP: &str = "pill";

/// The pill; the caller adds its click, tooltip, and accessible name. An `inert` pill, which a
/// press leaves as it is, shows no hover.
pub(crate) fn pill(
    id: impl Into<ElementId>,
    logo: Logo,
    on: bool,
    inert: bool,
    cx: &App,
) -> Stateful<Div> {
    let theme = cx.theme();
    let (foreground, muted, hover) = (theme.foreground, theme.muted_foreground, theme.muted);
    let ink = if on {
        Ink::from(foreground)
    } else {
        Ink {
            rest: muted.opacity(0.5),
            hover: (!inert).then_some((GROUP, foreground.opacity(0.5))),
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
        .when(!on, |pill| dashed(pill, cx))
        .when(!inert, |pill| pill.hover(move |style| style.bg(hover)))
        .child(mark)
}

/// `element` inside the dashed outline of an off pill or an inactive skill's `+` circle: a 1 px
/// border in the text color at 24%.
pub(crate) fn dashed<E: Styled>(element: E, cx: &App) -> E {
    element
        .border_1()
        .border_dashed()
        .border_color(cx.theme().foreground.alpha(0.24))
}
