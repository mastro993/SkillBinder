//! Library skills as rows inside one card, or as grid tiles on their own surfaces. Each shows its
//! description, when it last changed, how many files it holds, and which agents hold a copy.

use super::controls::{agent_avatar, app_icon, glyph_avatar};
use super::library_dates::{exact, relative};
use super::library_drift::drift_glyph;
use super::library_presence::{Holder, Reader};
use super::sidebar_row::{activate, with_menu};
use super::{NativeView, Route};
use chrono::{DateTime, Utc};
use gpui_kit::component::{ActiveTheme, Sizable, Size, h_flex, tooltip::Tooltip, v_flex};
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
/// Diameter of a presence avatar, the kit's extra-small avatar.
const AVATAR_SIZE: f32 = 16.;

/// One Library skill with its description, dates, and presence.
pub(super) struct SkillItem<'a> {
    pub skill: &'a Skill,
    pub description: &'a str,
    /// Absent for skills imported before import times were recorded, or that arrived by Pull.
    pub imported: Option<DateTime<Utc>>,
    /// The newest managed file's change time; absent when the payload could not be read.
    pub modified: Option<DateTime<Utc>>,
    /// The skill's folder, shown while Library lists every skill.
    pub folder: Option<&'a str>,
    /// The skill has an update, so it shows an Update button.
    pub update: bool,
    /// The skill's content differs from its record, so it shows a warning glyph.
    pub drifted: bool,
    /// The Agents rows holding a copy; `None` until the first scan completes.
    pub holders: Option<&'a [Holder]>,
}

impl SkillItem<'_> {
    fn name(&self) -> String {
        self.skill.id.to_string()
    }

    /// "320 files", or "1 file".
    fn files(&self) -> String {
        match self.skill.file_count {
            1 => "1 file".to_owned(),
            count => format!("{count} files"),
        }
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

    /// A row of the list card: name, one description line, then presence and dates.
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
                v_flex()
                    .w_full()
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
                    .child(meta_line(item, cx))
                    .child(description(item.description, 1, cx)),
            )
            .child(div().pt(px(6.)).child(presence(item, ring, cx)))
            .when(!last, |row| row.child(divider(cx)));
        frame(self, row, &id, owner, item, cx)
    }

    /// A grid tile: name with its Update button, two description lines, then presence and dates.
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
            .child(meta_line(item, cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(description(item.description, 2, cx)),
            )
            .child(h_flex().items_center().child(presence(item, ring, cx)));
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

/// The list card's surface, which the presence avatars punch out of.
fn surface(cx: &App) -> Hsla {
    skillbinder_theme::library::surface(cx.theme())
}

/// "13 hours ago · 320 files", with the skill's folder first while Library lists every skill. The
/// tooltip holds the exact times.
fn meta_line(item: &SkillItem<'_>, cx: &App) -> impl IntoElement {
    let tooltip = exact_times(item);
    let mut parts: Vec<String> = Vec::new();
    if let Some(folder) = item.folder {
        parts.push(folder.to_owned());
    }
    if let Some(modified) = item.modified {
        parts.push(relative(modified, Utc::now()));
    }
    parts.push(item.files());
    div()
        .id(format!("meta-{}", item.skill.id))
        .text_size(px(11.5))
        .text_color(cx.theme().muted_foreground.opacity(0.8))
        .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
        .child(parts.join(" · "))
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

/// The exact local times behind a row's relative date.
fn exact_times(item: &SkillItem<'_>) -> String {
    let mut lines: Vec<String> = Vec::new();
    if let Some(modified) = item.modified {
        lines.push(format!("Modified {}", exact(modified)));
    }
    if let Some(imported) = item.imported {
        lines.push(format!("Imported {}", exact(imported)));
    }
    lines.join("\n")
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

/// Agent avatars, Universal first, for the Agents rows whose global skill directory holds a copy.
/// A row's avatars overlap; each is ringed in the surface behind it so the stack reads as one.
fn presence(item: &SkillItem<'_>, ring: Hsla, cx: &mut Context<NativeView>) -> AnyElement {
    let muted = |text: &'static str| {
        div()
            .text_size(px(11.5))
            .text_color(cx.theme().muted_foreground)
            .child(text)
            .into_any_element()
    };
    let holders = match item.holders {
        None => return muted("Not scanned yet"),
        Some([]) => return muted("Not in any agent"),
        Some(holders) => holders,
    };
    let hidden = &holders[holders.len().min(SHOWN_HOLDERS)..];
    h_flex()
        .flex_shrink_0()
        .children(
            holders
                .iter()
                .take(SHOWN_HOLDERS)
                .enumerate()
                .map(|(index, holder)| holder_avatar(item, holder, index > 0, ring, cx)),
        )
        .when(!hidden.is_empty(), |row| {
            row.child(more_avatars(item, hidden, ring, cx))
        })
        .into_any_element()
}

/// One agent's avatar; its tooltip shows the directory holding the copy, and for Universal the
/// agents reading it. Clicking it opens the agent's page.
fn holder_avatar(
    item: &SkillItem<'_>,
    holder: &Holder,
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
                .with_size(Size::XSmall)
                .rounded_full()
                .border_2()
                .border_color(ring),
        )
        .into_any_element()
}

/// The `+N` circle naming the agents the stack leaves out.
fn more_avatars(
    item: &SkillItem<'_>,
    hidden: &[Holder],
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
        .text_size(px(8.))
        .text_color(cx.theme().muted_foreground)
        // The circle opens nothing, and the click must not open the skill either.
        .on_click(|_, _, cx| cx.stop_propagation())
        .tooltip(move |window, cx| Tooltip::new(names.clone()).build(window, cx))
        .child(format!("+{}", hidden.len()))
        .into_any_element()
}
