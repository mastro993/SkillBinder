//! Library skills as rows inside one card, or as grid tiles on their own surfaces. Each shows its
//! description, its folder while Library lists every skill, the agents it is applied for, and
//! its activation in the active preset.

use super::controls::{agent_avatar, app_icon, glyph_avatar};
use super::copy_dialogs::unsettled_summary;
use super::library_copies::{AgentRow, Placement, Reader};
use super::library_drift::drift_glyph;
use super::sidebar_row::{activate, with_menu};
use super::{DialogState, NativeView, Route};
use gpui_kit::component::{ActiveTheme, Sizable, h_flex, tooltip::Tooltip, v_flex};
use gpui_kit::{
    AnyElement, App, Context, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, prelude::FluentBuilder, px,
};
use skillbinder_proto::{Skill, SkillId};
use skillbinder_theme::library::{
    AVATAR_OVERLAP, CARD_RADIUS, DIVIDER_INSET, ROW_PADDING_X, ROW_PADDING_Y, TILE_RADIUS,
};

/// Agent avatars shown before the rest collapse into `+N`.
const SHOWN_HOLDERS: usize = 4;
/// Diameter of an agent avatar, a step above the kit's extra-small avatar so the agent logos read
/// at a glance.
const AVATAR_SIZE: f32 = 20.;

/// One Library skill with its description, folder, and copies.
pub(super) struct SkillItem<'a> {
    pub skill: &'a Skill,
    pub description: &'a str,
    /// The skill's folder, shown while Library lists every skill.
    pub folder: Option<&'a str>,
    /// The skill has an update, so it shows an Update button.
    pub update: bool,
    /// The skill's content differs from its record, so it shows a warning glyph.
    pub drifted: bool,
    /// Where the skill's copies stand; `None` while it has none.
    pub placement: Option<&'a Placement>,
}

impl SkillItem<'_> {
    fn name(&self) -> String {
        self.skill.id.to_string()
    }
}

impl NativeView {
    /// The list's card: one bordered surface holding every row, divided by inset hairlines.
    pub(super) fn skill_list(&self, items: &[SkillItem<'_>], cx: &mut Context<Self>) -> AnyElement {
        let last = items.len().saturating_sub(1);
        v_flex()
            .border_1()
            .rounded(px(CARD_RADIUS))
            .border_color(cx.theme().border)
            .bg(surface(cx))
            .overflow_hidden()
            .children(
                items
                    .iter()
                    .enumerate()
                    .map(|(index, item)| self.skill_row(item, index == last, cx)),
            )
            .into_any_element()
    }

    /// A row of the list card: name, one description line, then the agents it is applied for,
    /// with its activation at the trailing edge.
    fn skill_row(&self, item: &SkillItem<'_>, last: bool, cx: &mut Context<Self>) -> AnyElement {
        let id = item.skill.id.clone();
        let owner = format!("skill-row-{id}");
        let hover = cx.theme().muted;
        let ring = surface(cx);
        let row = div()
            .id(owner.clone())
            .relative()
            .flex()
            .flex_col()
            .w_full()
            .px(px(ROW_PADDING_X))
            .py(px(ROW_PADDING_Y))
            .cursor_pointer()
            .hover(move |row| row.bg(hover))
            .child(
                h_flex()
                    .w_full()
                    .items_start()
                    .gap_3()
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        div()
                                            .min_w_0()
                                            .text_size(px(14.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .truncate()
                                            .child(item.name()),
                                    )
                                    .when(item.update, |line| {
                                        line.child(update_glyph(&item.skill.id, cx))
                                    })
                                    .when(item.drifted, |line| {
                                        line.child(drift_glyph(&item.skill.id, cx))
                                    }),
                            )
                            .when_some(item.folder, |lines, folder| {
                                lines.child(folder_line(item, folder, cx))
                            })
                            .child(description(item.description, 1, cx)),
                    )
                    .child(self.activation_controls(&item.skill.id, cx)),
            )
            .child(div().pt(px(6.)).child(placement(item, ring, cx)))
            .when(!last, |row| row.child(divider(cx)));
        frame(self, row, &id, owner, item, cx)
    }

    /// A grid tile: name with its Update button, two description lines, then the agents it is
    /// applied for beside its activation.
    pub(super) fn skill_card(&self, item: &SkillItem<'_>, cx: &mut Context<Self>) -> AnyElement {
        let id = item.skill.id.clone();
        let owner = format!("skill-card-{id}");
        // A tile wears the list card's surface and hairline; only the hover fill differs.
        let (rest, hover, border) = (surface(cx), cx.theme().muted, cx.theme().border);
        let ring = rest;
        let card = div()
            .id(owner.clone())
            .flex()
            .flex_col()
            .w_full()
            .min_w_0()
            .gap(px(9.))
            .min_h(px(152.))
            .rounded(px(TILE_RADIUS))
            .bg(rest)
            .border_1()
            .border_color(border)
            .p(px(16.))
            .cursor_pointer()
            .hover(move |card| card.bg(hover))
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        h_flex()
                            .flex_1()
                            .min_w_0()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .min_w_0()
                                    .text_size(px(14.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .truncate()
                                    .child(item.name()),
                            )
                            .when(item.update, |line| {
                                line.child(update_glyph(&item.skill.id, cx))
                            })
                            .when(item.drifted, |line| {
                                line.child(drift_glyph(&item.skill.id, cx))
                            }),
                    )
                    .when(item.update, |line| {
                        line.child(self.update_button("card", &item.skill.id, cx))
                    }),
            )
            .when_some(item.folder, |card, folder| {
                card.child(folder_line(item, folder, cx))
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(description(item.description, 2, cx)),
            )
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(div().min_w_0().child(placement(item, ring, cx)))
                    .child(self.activation_controls(&item.skill.id, cx)),
            );
        frame(self, card, &id, owner, item, cx)
    }
}

/// Opens the skill's page on click or Enter, and its menu on right-click.
fn frame(
    view: &NativeView,
    element: gpui_kit::Stateful<gpui_kit::Div>,
    id: &skillbinder_proto::SkillId,
    owner: String,
    item: &SkillItem<'_>,
    cx: &mut Context<NativeView>,
) -> AnyElement {
    let route = Route::Skill(id.clone());
    let menu_id = id.clone();
    let element = activate(element, item.name(), cx, move |view, window, cx| {
        view.navigate(route.clone(), window, cx);
    });
    with_menu(element, owner, true, view, cx, move |view, _, cx| {
        view.skill_menu(&menu_id, cx)
    })
    // The wrapper is the grid item, so it must take the column's width rather than the text's.
    .w_full()
    .min_w_0()
    .into_any_element()
}

/// A hairline inset from both card edges, so it starts where the name starts.
fn divider(cx: &App) -> AnyElement {
    div()
        .absolute()
        .left(px(DIVIDER_INSET))
        .right(px(DIVIDER_INSET))
        .bottom(px(0.))
        .h(px(1.))
        .bg(cx.theme().border)
        .into_any_element()
}

/// The list card's surface, which the agent avatars punch out of.
fn surface(cx: &App) -> Hsla {
    skillbinder_theme::library::surface(cx.theme())
}

/// The skill's folder, shown while Library lists every skill.
fn folder_line(item: &SkillItem<'_>, folder: &str, cx: &App) -> impl IntoElement {
    div()
        .id(format!("folder-{}", item.skill.id))
        .text_size(px(11.5))
        .text_color(cx.theme().muted_foreground.opacity(0.8))
        .child(folder.to_owned())
}

/// The warning glyph of a skill whose source has an update.
fn update_glyph(id: &SkillId, cx: &App) -> AnyElement {
    div()
        .id(format!("update-{id}"))
        .flex_shrink_0()
        .child(
            app_icon("upload-circle")
                .small()
                .text_color(cx.theme().warning),
        )
        .tooltip(|window, cx| Tooltip::new("Update available").build(window, cx))
        .into_any_element()
}

fn description(text: &str, lines: usize, cx: &App) -> impl IntoElement {
    div()
        .text_size(px(12.5))
        .text_color(cx.theme().muted_foreground)
        .map(|text| {
            if lines == 1 {
                text.truncate()
            } else {
                text.line_clamp(lines).text_ellipsis()
            }
        })
        .child(text.to_owned())
}

/// Agent avatars, Universal first, for the Agents rows whose global skill directory holds the
/// skill's applied copy, then a warning glyph while any of its copies is not applied. A row's
/// avatars overlap; each is ringed in the surface behind it so the stack reads as one.
fn placement(item: &SkillItem<'_>, ring: Hsla, cx: &mut Context<NativeView>) -> AnyElement {
    let (applied, unsettled) = item.placement.map_or((&[][..], 0), |placement| {
        (placement.applied.as_slice(), placement.unsettled.len())
    });
    if applied.is_empty() && unsettled == 0 {
        return div()
            .text_size(px(11.5))
            .text_color(cx.theme().muted_foreground)
            .child("Not in any agent")
            .into_any_element();
    }
    let hidden = &applied[applied.len().min(SHOWN_HOLDERS)..];
    h_flex()
        .flex_shrink_0()
        .items_center()
        .children(
            applied
                .iter()
                .take(SHOWN_HOLDERS)
                .enumerate()
                .map(|(index, holder)| holder_avatar(item, holder, index > 0, ring, cx)),
        )
        .when(!hidden.is_empty(), |row| {
            row.child(more_avatars(item, hidden, ring, cx))
        })
        .when(unsettled > 0, |row| {
            row.child(unsettled_glyph(
                &item.skill.id,
                unsettled,
                !applied.is_empty(),
                cx,
            ))
        })
        .into_any_element()
}

/// The warning glyph of a skill with copies that are not applied; clicking it lists them.
fn unsettled_glyph(
    id: &SkillId,
    count: usize,
    after_avatars: bool,
    cx: &mut Context<NativeView>,
) -> AnyElement {
    let skill = id.clone();
    div()
        .id(format!("unsettled-{id}"))
        .flex_shrink_0()
        .cursor_pointer()
        .when(after_avatars, |glyph| glyph.ml(px(6.)))
        .child(
            app_icon("alert-circle")
                .small()
                .text_color(cx.theme().warning),
        )
        .tooltip(move |window, cx| Tooltip::new(unsettled_summary(count)).build(window, cx))
        .on_click(cx.listener(move |view, _, _, cx| {
            cx.stop_propagation();
            view.dialog = DialogState::Copies(Some(skill.clone()));
            cx.notify();
        }))
        .into_any_element()
}

/// One agent's avatar; its tooltip shows the directory holding the copy, and for Universal the
/// agents reading it. Clicking it opens the agent's page.
fn holder_avatar(
    item: &SkillItem<'_>,
    holder: &AgentRow,
    overlap: bool,
    ring: Hsla,
    cx: &mut Context<NativeView>,
) -> AnyElement {
    let route = holder.route();
    let (tip, key, avatar) = match &holder.reader {
        Reader::Universal(names) => (
            format!("{}\nUsed by {}", holder.dir, names.join(", ")),
            "universal",
            glyph_avatar(app_icon("layers"), cx),
        ),
        Reader::Agent(id) => (holder.dir.clone(), id.as_str(), agent_avatar(id, cx)),
    };
    div()
        .id(format!("holder-{}-{key}", item.skill.id))
        .flex_shrink_0()
        .cursor_pointer()
        .when(overlap, |chip| chip.ml(px(-AVATAR_OVERLAP)))
        .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx))
        .on_click(cx.listener(move |view, _, window, cx| {
            cx.stop_propagation();
            view.navigate(route.clone(), window, cx);
        }))
        .child(
            avatar
                .with_size(px(AVATAR_SIZE))
                .rounded_full()
                .border_2()
                .border_color(ring),
        )
        .into_any_element()
}

/// The `+N` circle naming the agents the stack leaves out.
fn more_avatars(
    item: &SkillItem<'_>,
    hidden: &[AgentRow],
    ring: Hsla,
    cx: &mut Context<NativeView>,
) -> AnyElement {
    let names = hidden
        .iter()
        .map(|holder| holder.name.clone())
        .collect::<Vec<_>>()
        .join("\n");
    div()
        .id(format!("holders-more-{}", item.skill.id))
        .flex_shrink_0()
        .ml(px(-AVATAR_OVERLAP))
        .flex()
        .items_center()
        .justify_center()
        .size(px(AVATAR_SIZE))
        .rounded_full()
        .border_2()
        .border_color(ring)
        .bg(cx.theme().muted)
        .text_size(px(9.5))
        .text_color(cx.theme().muted_foreground)
        // The circle opens nothing, and the click must not open the skill either.
        .on_click(|_, _, cx| cx.stop_propagation())
        .tooltip(move |window, cx| Tooltip::new(names.clone()).build(window, cx))
        .child(format!("+{}", hidden.len()))
        .into_any_element()
}
