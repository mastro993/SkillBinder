//! CSS box shadows in GPUI. GPUI's blur radius is a Gaussian sigma, half the CSS blur, so a shadow
//! copied across as-is spreads twice as far; [`css`] converts one.
//!
//! The large drop shadow of the app's own overlays: CSS `0 20px 25px -5px` then `0 8px 10px -6px`,
//! both black at 10%, under the end toast and the app's dialogs.
//!
//! A GPUI shadow keeps its element's corner radius on the spread box, where CSS rounds a spread
//! shadow to the radius plus the spread, so a pill or a card would cast a shadow with the wrong
//! ends. Each shadow is therefore cast by a transparent box of its own, placed and rounded as CSS
//! places and rounds that shadow, with no spread or offset.

use gpui_kit::{BoxShadow, Div, Hsla, IntoElement, ParentElement, Styled, div, hsla, px};

/// The CSS shadow `x y blur spread color`.
pub(crate) fn css(x: f32, y: f32, blur: f32, spread: f32, color: Hsla) -> BoxShadow {
    BoxShadow::new(px(x), px(y), color)
        .blur_radius(px(blur / 2.))
        .spread_radius(px(spread))
}

/// One CSS drop shadow in black at 10%.
struct Css {
    y: f32,
    blur: f32,
    spread: f32,
}

/// The large shadow, first on top as CSS lists it.
const LARGE: [Css; 2] = [
    Css {
        y: 20.,
        blur: 25.,
        spread: -5.,
    },
    Css {
        y: 8.,
        blur: 10.,
        spread: -6.,
    },
];

/// The corners of the surface a shadow follows.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Corners {
    /// Rounded to this radius.
    Radius(f32),
}

/// `surface` under the large shadow, in a holder as large as it. `inset` is how far inside
/// `surface` the box the design shadows lies. The shadows paint before `surface`.
pub(crate) fn under_large(surface: impl IntoElement, inset: f32, corners: Corners) -> Div {
    div()
        .relative()
        .flex()
        .max_w_full()
        .children(
            LARGE
                .iter()
                .rev()
                .map(|shadow| caster(shadow, inset, corners)),
        )
        .child(surface)
}

/// The transparent box that casts `shadow` for a surface `inset` inside its holder.
fn caster(shadow: &Css, inset: f32, corners: Corners) -> Div {
    let side = inset - shadow.spread;
    let ink = hsla(0., 0., 0., 0.1);
    let caster = div()
        .absolute()
        .left(px(side))
        .right(px(side))
        .top(px(side + shadow.y))
        .bottom(px(side - shadow.y))
        .shadow(vec![css(0., 0., shadow.blur, 0., ink)]);
    let Corners::Radius(radius) = corners;
    caster.rounded(px((radius + shadow.spread).max(0.)))
}
