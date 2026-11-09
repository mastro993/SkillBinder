//! Agent avatars: 18 px circles, each ringed 2 px outside its circle in the fill behind it, so an
//! overlapping stack reads as one. Kit's avatar draws its ring inside a larger box and pads its
//! logo, so the app draws its own.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    App, Div, FontWeight, Hsla, InteractiveElement, ParentElement, SharedString, Styled, div,
    prelude::FluentBuilder, px, white,
};

use super::logo::Logo;
use super::text;

/// An avatar's diameter.
pub(crate) const SIZE: f32 = 18.;
/// How far each avatar overlaps the one before it.
pub(crate) const OVERLAP: f32 = 5.;
/// The ring's width outside the circle.
const RING: f32 = 2.;
/// A color logo's share of the circle, on white.
const COLOR_SCALE: f32 = 0.78;
/// A glyph's or monochrome logo's share of the circle, on the muted fill.
const GLYPH_SCALE: f32 = 0.7;

/// The fill a ring takes: at rest, and while each named group is hovered, later ones winning.
#[derive(Clone)]
pub(crate) struct Ring {
    pub rest: Hsla,
    pub hovers: Vec<(SharedString, Hsla)>,
}

impl From<Hsla> for Ring {
    fn from(rest: Hsla) -> Self {
        Self {
            rest,
            hovers: Vec::new(),
        }
    }
}

/// The ring, a circle `RING` wider than the avatar on every side, pulled back by negative margins
/// so it takes no layout space; `first` avatars do not overlap the one before.
fn ringed(ring: &Ring, first: bool, circle: Div) -> Div {
    let mut wrap = div()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(px(SIZE + 2. * RING))
        .m(px(-RING))
        .when(!first, |wrap| wrap.ml(px(-RING - OVERLAP)))
        .rounded_full()
        .bg(ring.rest);
    for (group, fill) in &ring.hovers {
        let fill = *fill;
        wrap = wrap.group_hover(group.clone(), move |style| style.bg(fill));
    }
    wrap.child(circle)
}

/// One avatar showing `logo`: a color logo on white, else the glyph in the text color on the
/// muted fill.
pub(crate) fn avatar(logo: Logo, ring: &Ring, first: bool, cx: &App) -> Div {
    let theme = cx.theme();
    let (fill, scale) = if logo.is_color() {
        (white(), COLOR_SCALE)
    } else {
        (theme.muted, GLYPH_SCALE)
    };
    let circle = div()
        .flex()
        .items_center()
        .justify_center()
        .size(px(SIZE))
        .rounded_full()
        .bg(fill)
        .child(logo.element(SIZE * scale, theme.foreground.into(), false));
    ringed(ring, first, circle)
}

/// The `+N` circle after the shown avatars.
pub(crate) fn more(count: usize, ring: &Ring, cx: &App) -> Div {
    let theme = cx.theme();
    let circle = text::sized(div(), 9.5)
        .flex()
        .items_center()
        .justify_center()
        .size(px(SIZE))
        .rounded_full()
        .bg(theme.muted)
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme.muted_foreground)
        .child(format!("+{count}"));
    ringed(ring, false, circle)
}

/// Avatars shown in a stack before the rest collapse into `+N`.
pub(crate) const SHOWN: usize = 4;

/// An overlapping stack of `logos`, the first `SHOWN` as avatars and the rest as `+N`.
pub(crate) fn stack(logos: &[Logo], ring: &Ring, cx: &App) -> Div {
    let hidden = logos.len().saturating_sub(SHOWN);
    div()
        .flex()
        .flex_none()
        .items_center()
        .children(
            logos
                .iter()
                .take(SHOWN)
                .enumerate()
                .map(|(index, logo)| avatar(*logo, ring, index == 0, cx)),
        )
        .when(hidden > 0, |stack| stack.child(more(hidden, ring, cx)))
}
