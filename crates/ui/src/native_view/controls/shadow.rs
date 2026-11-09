//! The large drop shadow of the app's own overlays: CSS `0 20px 25px -5px` then `0 8px 10px -6px`,
//! both black at 10%, under the progress pill, the end toast, and the app's dialogs.
//!
//! A GPUI shadow keeps its element's corner radius on the spread box, where CSS rounds a spread
//! shadow to the radius plus the spread, so a pill or a card would cast a shadow with the wrong
//! ends. Each shadow is therefore cast by a transparent box of its own, placed and rounded as CSS
//! places and rounds that shadow, with no spread or offset; GPUI's blur radius is a Gaussian
//! sigma, half the CSS blur.

use gpui_kit::{BoxShadow, Div, IntoElement, ParentElement, Styled, div, hsla, px};

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
    /// Fully rounded, as a pill.
    Full,
    /// Rounded to this radius.
    Radius(f32),
}

/// `surface` under the large shadow, in a holder as large as it. `inset` is how far inside
/// `surface` the box the design shadows lies, such as 1 px for a hairline ring drawn around it.
/// The shadows paint before `surface`, so a translucent ring composites over them as in CSS.
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
        .shadow(vec![
            BoxShadow::new(px(0.), px(0.), ink).blur_radius(px(shadow.blur / 2.)),
        ]);
    match corners {
        Corners::Full => caster.rounded_full(),
        Corners::Radius(radius) => caster.rounded(px((radius + shadow.spread).max(0.))),
    }
}
