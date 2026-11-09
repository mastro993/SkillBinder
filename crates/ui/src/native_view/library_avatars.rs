//! A skill's copies as avatars: on a workspace's page, the agents line under a row, naming the
//! directories that hold the skill's applied copy; everywhere, the warning glyph of copies that
//! are not applied.

use super::controls::avatar::{self, Ring, SHOWN};
use super::controls::logo::Logo;
use super::controls::{app_icon, text, tooltip::tip};
use super::copy_dialogs::unsettled_summary;
use super::library_copies::{AgentRow, Reader};
use super::library_items::{SkillItem, surface};
use super::{DialogState, NativeView};
use gpui_kit::component::{ActiveTheme, Sizable, h_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, prelude::FluentBuilder, px,
};
use skillbinder_proto::SkillId;

impl NativeView {
    /// Agent avatars, Universal first, for the directories that hold the skill's applied copy,
    /// then the warning glyph of copies that are not applied, or a muted note while neither shows.
    /// Clicking an avatar opens its agent's page.
    pub(super) fn agents_line(&self, item: &SkillItem<'_>, cx: &mut Context<Self>) -> AnyElement {
        let (applied, unsettled) = item.placement.map_or((&[][..], 0), |placement| {
            (placement.applied.as_slice(), placement.unsettled.len())
        });
        if applied.is_empty() && unsettled == 0 {
            return text::sized(div(), 11.5)
                .text_color(cx.theme().muted_foreground)
                .child("Not in any agent")
                .into_any_element();
        }
        let ring = Ring::from(surface(cx));
        let hidden = applied.len().saturating_sub(SHOWN);
        h_flex()
            .flex_shrink_0()
            .items_center()
            .children(
                applied
                    .iter()
                    .take(SHOWN)
                    .enumerate()
                    .map(|(index, holder)| {
                        holder_avatar(&item.skill.id, holder, index == 0, &ring, cx)
                    }),
            )
            .when(hidden > 0, |line| {
                let names = applied[SHOWN..]
                    .iter()
                    .map(|holder| holder.name.clone())
                    .collect::<Vec<_>>()
                    .join("\n");
                let more = div()
                    .id(format!("holders-more-{}", item.skill.id))
                    .flex_none()
                    .child(avatar::more(hidden, &ring, cx))
                    // The circle opens nothing, and the click must not open the skill either.
                    .on_click(|_, _, cx| cx.stop_propagation());
                line.child(tip(more, names))
            })
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
        let glyph = div()
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
                view.dialog = DialogState::Copies(Some(skill.clone()));
                cx.notify();
            }));
        tip(glyph, unsettled_summary(count)).into_any_element()
    }
}

/// One directory's avatar; its tooltip names the directory, and for Universal the agents reading
/// it. Clicking it opens the agent's page.
fn holder_avatar(
    skill: &SkillId,
    holder: &AgentRow,
    first: bool,
    ring: &Ring,
    cx: &mut Context<NativeView>,
) -> AnyElement {
    let route = holder.route();
    let (tooltip, key) = match &holder.reader {
        Reader::Universal(names) => (
            format!("{}\nUsed by {}", holder.dir, names.join(", ")),
            "universal".to_owned(),
        ),
        Reader::Agent(id) => (holder.dir.clone(), id.clone()),
    };
    let chip = div()
        .id(format!("holder-{skill}-{key}"))
        .flex_none()
        .cursor_pointer()
        .child(avatar::avatar(
            Logo::reader(&holder.reader),
            ring,
            first,
            cx,
        ))
        .on_click(cx.listener(move |view, _, window, cx| {
            cx.stop_propagation();
            view.navigate(route.clone(), window, cx);
        }));
    tip(chip, tooltip).into_any_element()
}
