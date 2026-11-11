//! A skill's copies as avatars: on a workspace's page, the agents line under a row, naming the
//! directories that hold the skill's applied copy; everywhere, the warning glyph of copies that
//! are not applied. The avatar stack, which Library's placement button shows too, lives here.

use super::controls::text::sized;
use super::controls::{Logo, app_icon, tooltip};
use super::copy_dialogs::{CopiesListing, unsettled_summary};
use super::library_copies::{AgentRow, Reader};
use super::library_items::{ITEM_GROUP, SkillItem, surface};
use super::{DialogState, NativeView};
use gpui_kit::component::{ActiveTheme, Icon, Sizable, avatar::Avatar, h_flex};
use gpui_kit::{
    AnyElement, App, Context, Div, FontWeight, Hsla, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, Text, div, prelude::FluentBuilder, px,
    white,
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

impl NativeView {
    /// Agent avatars, Universal first, for the directories that hold the skill's applied copy,
    /// then the warning glyph of copies that are not applied, or a muted note while neither shows.
    /// Clicking an avatar opens its agent's page.
    pub(super) fn agents_line(&self, item: &SkillItem<'_>, cx: &mut Context<Self>) -> AnyElement {
        let (applied, unsettled) = item.placement.map_or((&[][..], 0), |placement| {
            (placement.applied.as_slice(), placement.unsettled.len())
        });
        if applied.is_empty() && unsettled == 0 {
            return sized(div().id("no-agents"), 11.5)
                .text_color(cx.theme().muted_foreground)
                .child("Not in any agent")
                .into_any_element();
        }
        let ring = Ring {
            rest: surface(cx),
            hovered: None,
        };
        let hidden = applied.len().saturating_sub(SHOWN);
        let more = (hidden > 0).then(|| {
            let names = applied[SHOWN..]
                .iter()
                .map(|holder| holder.name.clone())
                .collect::<Vec<_>>()
                .join("\n");
            div()
                .id(format!("holders-more-{}", item.skill.id))
                .child(more(hidden, ring, cx))
                // The circle opens nothing, and the click must not open the skill either.
                .on_click(|_, _, cx| cx.stop_propagation())
                .tooltip(tooltip(names))
                .into_any_element()
        });
        let avatars: Vec<AnyElement> = applied
            .iter()
            .take(SHOWN)
            .map(|holder| holder_avatar(&item.skill.id, holder, ring, cx))
            .chain(more)
            .collect();
        h_flex()
            .flex_shrink_0()
            .items_center()
            .when(!avatars.is_empty(), |line| line.child(overlap(avatars)))
            .when(unsettled > 0, |line| {
                line.child(
                    div()
                        .when(!applied.is_empty(), |glyph| glyph.ml(px(6.)))
                        .child(self.unsettled_glyph(&item.skill.id, unsettled, cx)),
                )
            })
            .into_any_element()
    }

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

/// One directory's avatar; its tooltip names the directory, and for Universal the agents reading
/// it. Clicking it opens the agent's page.
fn holder_avatar(
    skill: &SkillId,
    holder: &AgentRow,
    ring: Ring,
    cx: &mut Context<NativeView>,
) -> AnyElement {
    let route = holder.route();
    let (text, key) = match &holder.reader {
        Reader::Universal(names) => (
            format!("{}\nUsed by {}", holder.dir, names.join(", ")),
            "universal".to_owned(),
        ),
        Reader::Agent(id) => (holder.dir.clone(), id.clone()),
    };
    div()
        .id(format!("holder-{skill}-{key}"))
        .cursor_pointer()
        .child(avatar(Logo::reader(&holder.reader), ring, cx))
        .on_click(cx.listener(move |view, _, window, cx| {
            cx.stop_propagation();
            view.navigate(route.clone(), window, cx);
        }))
        .tooltip(super::controls::tooltip(text))
        .into_any_element()
}
