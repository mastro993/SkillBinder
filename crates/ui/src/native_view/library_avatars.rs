//! Agent avatars in an overlapping stack, which Library's placement button and a workspace page's
//! agents button show, and the warning glyph of a skill's copies that are not applied.

use super::controls::text::sized;
use super::controls::{Logo, app_icon, tooltip};
use super::copy_dialogs::{CopiesListing, unsettled_summary};
use super::library_items::ITEM_GROUP;
use super::{DialogState, NativeView};
use gpui_kit::component::{ActiveTheme, Icon, Sizable, avatar::Avatar};
use gpui_kit::{
    AnyElement, App, Context, Div, FontWeight, Hsla, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, Text, div, px, white,
};
use skillbinder_proto::SkillId;

/// An avatar's visible circle.
const SIZE: f32 = 18.;
/// The ring around the circle, a border in the fill behind it.
const RING: f32 = 2.;
/// How far each circle overlaps the one before it.
const OVERLAP: f32 = 5.;
/// Avatars shown in a stack before the rest collapse into `+N`.
const SHOWN: usize = 4;

/// The fill behind an avatar, which its ring takes so an overlapping stack reads as one: at
/// rest, and while the row or tile holding it is hovered.
#[derive(Clone, Copy)]
pub(super) struct Ring {
    pub rest: Hsla,
    pub hovered: Option<Hsla>,
}

impl Ring {
    /// `element`, a circle `RING` wider on every side than its visible one, ringed.
    fn around<E: Styled + InteractiveElement>(self, element: E) -> E {
        let element = element
            .size(px(SIZE + 2. * RING))
            .border(px(RING))
            .border_color(self.rest);
        match self.hovered {
            Some(color) => element.group_hover(ITEM_GROUP, move |style| style.border_color(color)),
            None => element,
        }
    }
}

/// Kit's avatar showing `logo`: a color logo on white, else the glyph in the text color on the
/// muted fill. Kit clips the logo to the circle.
pub(super) fn avatar(logo: Logo, ring: Ring, cx: &App) -> Avatar {
    let theme = cx.theme();
    let avatar = ring.around(Avatar::new().with_size(px(SIZE + 2. * RING)));
    match logo {
        Logo::Color(path) => avatar.src(path).p(px(RING)).bg(white()),
        Logo::Glyph(path) | Logo::Mono(path) => avatar
            .placeholder(
                Icon::empty()
                    .path(path)
                    .with_size(px(SIZE * 0.7))
                    .text_color(theme.foreground),
            )
            .bg(theme.muted),
    }
}

/// The `+N` circle after the shown avatars; its text is `more`.
fn more(count: usize, ring: Ring, cx: &App) -> Div {
    let theme = cx.theme();
    ring.around(sized(div(), 9.5))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(theme.muted)
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme.muted_foreground)
        .child(Text::new("more".into(), format!("+{count}").into()))
}

/// `items`, each an avatar, each after the first overlapping the one before by `OVERLAP` and
/// painted over it. The row is as wide and as tall as the circles, so the rings take no room.
/// Each item is placed absolutely, since GPUI's flexbox sizes a row of items with negative
/// margins as zero wide.
fn overlap<E: IntoElement>(items: impl IntoIterator<Item = E>) -> Div {
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

/// An overlapping stack of `logos`, the first `SHOWN` as avatars and the rest as `+N`.
pub(super) fn stack(logos: &[Logo], ring: Ring, cx: &App) -> Div {
    let hidden = logos.len().saturating_sub(SHOWN);
    overlap(
        logos
            .iter()
            .take(SHOWN)
            .map(|logo| avatar(*logo, ring, cx).into_any_element())
            .chain((hidden > 0).then(|| more(hidden, ring, cx).into_any_element())),
    )
}

/// An overlapping stack of `logos`, the first `SHOWN` as avatars and the rest as `+N`, whose
/// tooltip lists the rest's `names`, one per line; `id` names the `+N` circle.
pub(super) fn named_stack(id: String, logos: &[(Logo, String)], ring: Ring, cx: &App) -> Div {
    let hidden = logos.get(SHOWN..).unwrap_or_default();
    let rest = (!hidden.is_empty()).then(|| {
        let names: Vec<&str> = hidden.iter().map(|(_, name)| name.as_str()).collect();
        div()
            .id(id)
            .child(more(hidden.len(), ring, cx))
            .tooltip(tooltip(names.join("\n")))
            .into_any_element()
    });
    overlap(
        logos
            .iter()
            .take(SHOWN)
            .map(|(logo, _)| avatar(*logo, ring, cx).into_any_element())
            .chain(rest),
    )
}

impl NativeView {
    /// The warning glyph of a skill with copies that are not applied; clicking it lists them.
    pub(super) fn unsettled_glyph(
        &self,
        id: &SkillId,
        count: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let skill = id.clone();
        div()
            .id(format!("unsettled-{id}"))
            .flex_shrink_0()
            .cursor_pointer()
            .child(
                app_icon("alert-circle")
                    .small()
                    .text_color(cx.theme().warning),
            )
            .on_click(cx.listener(move |view, _, _, cx| {
                cx.stop_propagation();
                view.dialog = DialogState::Copies(CopiesListing::Skill(skill.clone()));
                cx.notify();
            }))
            .tooltip(tooltip(unsettled_summary(count)))
            .into_any_element()
    }
}
