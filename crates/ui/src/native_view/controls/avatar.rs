//! Agent avatars: 18 px circles, each ringed 2 px outside its circle in the fill behind it, so an
//! overlapping stack reads as one. Kit's avatar draws its ring inside a larger box and pads its
//! logo, so the app draws its own.

use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    App, Div, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement, SharedString,
    Styled, div, prelude::FluentBuilder, px, white,
};

use super::logo::Logo;
use super::text;

/// An avatar's diameter.
const SIZE: f32 = 18.;
/// How far each avatar overlaps the one before it.
const OVERLAP: f32 = 5.;
/// The ring's width outside the circle.
const RING: f32 = 2.;
/// A color logo's share of the circle, on white.
const COLOR_SCALE: f32 = 0.78;
/// A glyph's or monochrome logo's share of the circle, on the muted fill.
const GLYPH_SCALE: f32 = 0.7;

/// A fill at rest, and while the named group is hovered.
#[derive(Clone)]
pub(crate) struct Fill {
    pub rest: Hsla,
    pub hover: Option<(SharedString, Hsla)>,
}

/// What a ring takes: the fill behind the avatar, such as a row's, then an optional translucent
/// layer over it, such as the highlight of a hovered or open button. GPUI gives an element one
/// group hover, so each layer follows its own group.
#[derive(Clone)]
pub(crate) struct Ring {
    pub fill: Fill,
    pub tint: Option<Fill>,
}

impl From<Hsla> for Ring {
    fn from(rest: Hsla) -> Self {
        Self {
            fill: Fill { rest, hover: None },
            tint: None,
        }
    }
}

/// `element` filled as `fill` says.
fn filled(element: Div, fill: &Fill) -> Div {
    let element = element.bg(fill.rest);
    match &fill.hover {
        Some((group, color)) => {
            let color = *color;
            element.group_hover(group.clone(), move |style| style.bg(color))
        }
        None => element,
    }
}

/// An 18 px box holding `circle` over its ring, a circle `RING` wider on every side. The ring is
/// placed absolutely, so it takes no layout space; GPUI's flexbox sizes a row of items with
/// negative margins as zero wide, so nothing here uses them.
fn ringed(ring: &Ring, circle: Div) -> Div {
    let halo = div()
        .absolute()
        .left(px(-RING))
        .top(px(-RING))
        .size(px(SIZE + 2. * RING))
        .rounded_full();
    div()
        .relative()
        .flex_none()
        .size(px(SIZE))
        .child(
            filled(halo, &ring.fill).when_some(ring.tint.as_ref(), |halo, tint| {
                halo.child(filled(
                    div().absolute().top_0().left_0().size_full().rounded_full(),
                    tint,
                ))
            }),
        )
        .child(circle.absolute().top_0().left_0())
}

/// One avatar showing `logo`: a color logo on white, else the glyph in the text color on the
/// muted fill.
pub(crate) fn avatar(logo: Logo, ring: &Ring, cx: &App) -> Div {
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
    ringed(ring, circle)
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
    ringed(ring, circle)
}

/// Avatars shown in a stack before the rest collapse into `+N`.
pub(crate) const SHOWN: usize = 4;

/// `items`, each an 18 px avatar, each after the first overlapping the one before by `OVERLAP`
/// and painted over it; the row is as wide as they reach.
pub(crate) fn overlap<E: IntoElement>(items: impl IntoIterator<Item = E>) -> Div {
    let step = SIZE - OVERLAP;
    let mut count = 0u16;
    let row = div()
        .relative()
        .flex_none()
        .h(px(SIZE))
        .children(items.into_iter().map(|item| {
            let left = step * f32::from(count);
            count += 1;
            div().absolute().top_0().left(px(left)).child(item)
        }));
    let width = if count == 0 {
        0.
    } else {
        SIZE + step * f32::from(count - 1)
    };
    row.w(px(width))
}

/// An overlapping stack of `logos`, the first `SHOWN` as avatars and the rest as `+N`.
pub(crate) fn stack(logos: &[Logo], ring: &Ring, cx: &App) -> Div {
    let hidden = logos.len().saturating_sub(SHOWN);
    overlap(
        logos
            .iter()
            .take(SHOWN)
            .map(|logo| avatar(*logo, ring, cx))
            .chain((hidden > 0).then(|| more(hidden, ring, cx))),
    )
}
