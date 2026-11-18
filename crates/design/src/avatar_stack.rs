//! Molecule: Kit avatars showing [`Logo`]s in an overlapping stack, the first few as avatars and
//! the rest as a `+N` circle, each ringed in the fill behind it so the stack reads as one.
//!
//! Kit's `AvatarGroup` shows an ellipsis rather than `+N`, and overlaps its avatars with negative
//! margins, which GPUI's flexbox sizes as zero wide; the stack places Kit's avatars itself.

use crate::tokens::text;
use crate::{IconSize, Level, Logo, Text, tooltip};
use gpui_kit::component::{ActiveTheme, Icon, Sizable, avatar::Avatar};
use gpui_kit::{
    AnyElement, App, Div, ElementId, Hsla, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, px, white,
};

/// An avatar's visible circle.
const SIZE: f32 = 18.;
/// The ring around the circle, a border in the fill behind it.
const RING: f32 = 2.;
/// How far each circle overlaps the one before it.
const OVERLAP: f32 = 5.;
/// Avatars a stack shows before the rest collapse into `+N`.
const LIMIT: usize = 4;

/// The fill behind the avatars, which their rings take: at rest, and while the element `group`
/// names, such as the row holding them, is hovered.
#[derive(Clone)]
struct Ring {
    rest: Hsla,
    hovered: Option<(SharedString, Hsla)>,
}

impl Ring {
    /// `element`, a circle `RING` wider on every side than its visible one, ringed.
    fn around<E: Styled + InteractiveElement>(&self, element: E) -> E {
        let element = element
            .size(px(SIZE + 2. * RING))
            .border(px(RING))
            .border_color(self.rest);
        match self.hovered.clone() {
            Some((group, color)) => {
                element.group_hover(group, move |style| style.border_color(color))
            }
            None => element,
        }
    }
}

/// A stack of agent or directory avatars.
#[derive(IntoElement)]
pub struct AvatarStack {
    logos: Vec<Logo>,
    limit: usize,
    rest: Option<Hsla>,
    hovered: Option<(SharedString, Hsla)>,
    /// The `+N` circle's id and the name of every logo, which its tooltip lists for the hidden.
    names: Option<(ElementId, Vec<SharedString>)>,
}

impl AvatarStack {
    /// A stack of `logos`, the first four shown, ringed in the flat surface's fill.
    pub fn new(logos: impl IntoIterator<Item = Logo>) -> Self {
        Self {
            logos: logos.into_iter().collect(),
            limit: LIMIT,
            rest: None,
            hovered: None,
            names: None,
        }
    }

    /// Shows the first `limit` logos as avatars and the rest as `+N`.
    pub fn limit(self, limit: usize) -> Self {
        Self { limit, ..self }
    }

    /// Rings the avatars in `fill`, the fill behind them.
    pub fn ring(self, fill: Hsla) -> Self {
        Self {
            rest: Some(fill),
            ..self
        }
    }

    /// Rings the avatars in `fill` while the element of `group` is hovered and fills with it.
    pub fn hover_ring(self, group: impl Into<SharedString>, fill: Hsla) -> Self {
        Self {
            hovered: Some((group.into(), fill)),
            ..self
        }
    }

    /// Names each logo, in order; the `+N` circle, `id`, lists the hidden ones in its tooltip.
    pub fn names(
        self,
        id: impl Into<ElementId>,
        names: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        Self {
            names: Some((id.into(), names.into_iter().map(Into::into).collect())),
            ..self
        }
    }
}

impl RenderOnce for AvatarStack {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let ring = Ring {
            rest: self.rest.unwrap_or_else(|| Level::Flat.fill(cx.theme())),
            hovered: self.hovered,
        };
        let hidden = self.logos.len().saturating_sub(self.limit);
        let more = (hidden > 0).then(|| {
            let circle = more(hidden, &ring, cx);
            match self.names {
                Some((id, names)) => {
                    let rest: Vec<&str> = names
                        .iter()
                        .skip(self.limit)
                        .map(SharedString::as_ref)
                        .collect();
                    div()
                        .id(id)
                        .child(circle)
                        .tooltip(tooltip(rest.join("\n")))
                        .into_any_element()
                }
                None => circle.into_any_element(),
            }
        });
        overlap(
            self.logos
                .into_iter()
                .take(self.limit)
                .map(|logo| avatar(logo, &ring, cx).into_any_element())
                .chain(more),
        )
    }
}

/// Kit's avatar showing `logo`: a color logo on white, else the glyph in the text color on the
/// muted fill. Kit clips the logo to the circle.
fn avatar(logo: Logo, ring: &Ring, cx: &App) -> Avatar {
    let theme = cx.theme();
    let avatar = ring.around(Avatar::new().with_size(px(SIZE + 2. * RING)));
    match logo {
        Logo::Color(path) => avatar.src(path).p(px(RING)).bg(white()),
        Logo::Glyph(path) | Logo::Mono(path) => avatar
            .placeholder(
                Icon::empty()
                    .path(path)
                    .with_size(IconSize::Xs)
                    .text_color(theme.foreground),
            )
            .bg(theme.muted),
    }
}

/// The `+N` circle after the shown avatars.
fn more(count: usize, ring: &Ring, cx: &App) -> Div {
    let theme = cx.theme();
    ring.around(div())
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(theme.muted)
        .text_color(theme.foreground)
        .child(Text::labeled("more", format!("+{count}")).level(text::MICRO))
}

/// `items`, each an avatar, each after the first overlapping the one before by `OVERLAP` and
/// painted over it. The row is as wide and as tall as the circles, so the rings take no room.
/// Each item is placed absolutely, since GPUI's flexbox sizes a row of items with negative
/// margins as zero wide.
fn overlap(items: impl IntoIterator<Item = AnyElement>) -> Div {
    let step = SIZE - OVERLAP;
    let mut count = 0u16;
    let row = div()
        .relative()
        .flex_none()
        .h(px(SIZE))
        .children(items.into_iter().map(|item| {
            let left = step * f32::from(count) - RING;
            count += 1;
            div().absolute().top(px(-RING)).left(px(left)).child(item)
        }));
    row.w(px(count
        .checked_sub(1)
        .map_or(0., |rest| SIZE + step * f32::from(rest))))
}

#[cfg(test)]
mod tests;
