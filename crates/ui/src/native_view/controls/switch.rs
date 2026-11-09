//! A 30 by 18 switch with a 14 px shadowed thumb. Kit's switch comes only 28 by 16 or 36 by 20,
//! without a thumb shadow, so the app draws its own.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    App, Div, ElementId, InteractiveElement, ParentElement, Stateful, Styled, div, hsla, px,
};

use super::shadow;

const WIDTH: f32 = 30.;
const HEIGHT: f32 = 18.;
const THUMB: f32 = 14.;
const INSET: f32 = 2.;

/// The switch, on or off; the caller adds its click and accessible name.
pub(crate) fn switch(id: impl Into<ElementId>, on: bool, cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    let track = if on { theme.primary } else { theme.switch };
    let left = if on { WIDTH - THUMB - INSET } else { INSET };
    div()
        .id(id)
        .relative()
        .flex_none()
        .w(px(WIDTH))
        .h(px(HEIGHT))
        .rounded_full()
        .bg(track)
        .child(
            div()
                .absolute()
                .top(px(INSET))
                .left(px(left))
                .size(px(THUMB))
                .rounded_full()
                .bg(theme.background)
                .shadow(vec![shadow::css(0., 1., 2., 0., hsla(0., 0., 0., 0.25))]),
        )
}
