//! Library skills as list rows or grid cards, each showing when it was installed and changed and
//! which agents hold a copy.

use super::controls::{agent_avatar, app_icon, glyph_avatar};
use super::library_dates::{exact, relative};
use super::library_presence::{Holder, Reader};
use super::sidebar_row::{activate, with_menu};
use super::{NativeView, Route};
use chrono::{DateTime, Utc};
use gpui_kit::component::{ActiveTheme, Sizable, Size, h_flex, tag::Tag, tooltip::Tooltip, v_flex};
use gpui_kit::{
    AnyElement, App, Context, FontWeight, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, prelude::FluentBuilder, px,
};
use skillbinder_proto::Skill;

/// Agent chips shown before the rest collapse into `+N`.
const SHOWN_HOLDERS: usize = 3;

/// One Library skill with its description, dates, and presence.
pub(super) struct SkillItem<'a> {
    pub skill: &'a Skill,
    pub description: &'a str,
    /// Absent for skills installed before install times were recorded.
    pub imported: Option<DateTime<Utc>>,
    /// The newest managed file's change time; absent when the payload could not be read.
    pub modified: Option<DateTime<Utc>>,
    /// The skill's folder, shown while Library lists every skill.
    pub folder: Option<&'a str>,
    /// The Agents rows holding a copy; `None` until the first scan completes.
    pub holders: Option<&'a [Holder]>,
}

impl SkillItem<'_> {
    fn name(&self) -> String {
        self.skill.id.to_string()
    }
}

impl NativeView {
    /// A full-width row: name, one description line, and dates, then presence.
    pub(super) fn skill_row(&self, item: &SkillItem<'_>, cx: &mut Context<Self>) -> AnyElement {
        self.skill_frame("skill-row", item, cx)
            .items_center()
            .gap_3()
            .px_3()
            .py_2()
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .truncate()
                            .child(item.name()),
                    )
                    .child(description(item.description, 1, cx))
                    .child(dates(item, cx)),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .max_w(px(420.))
                    .child(presence(item, cx)),
            )
            .into_any_element()
    }

    /// A card: name, two description lines, dates, then presence.
    pub(super) fn skill_card(&self, item: &SkillItem<'_>, cx: &mut Context<Self>) -> AnyElement {
        self.skill_frame("skill-card", item, cx)
            .flex_col()
            .gap_2()
            .p_3()
            .min_h(px(132.))
            .child(
                div()
                    .min_w_0()
                    .font_weight(FontWeight::MEDIUM)
                    .truncate()
                    .child(item.name()),
            )
            .child(div().flex_1().child(description(item.description, 2, cx)))
            .child(dates(item, cx))
            .child(presence(item, cx))
            .into_any_element()
    }

    /// The bordered frame of a row or card; clicking it, or Enter or Space while it is focused,
    /// opens the skill's page, and right-clicking it opens its menu.
    fn skill_frame(
        &self,
        kind: &str,
        item: &SkillItem<'_>,
        cx: &mut Context<NativeView>,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        let id = item.skill.id.clone();
        let owner = format!("{kind}-{id}");
        let frame = skill_frame(kind, item, cx);
        with_menu(frame, owner, true, self, cx, move |view, _, cx| {
            view.skill_menu(&id, cx)
        })
    }
}

fn skill_frame(
    kind: &str,
    item: &SkillItem<'_>,
    cx: &mut Context<NativeView>,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    let route = Route::Skill(item.skill.id.clone());
    let frame = div()
        .id(format!("{kind}-{}", item.skill.id))
        .flex()
        .min_w_0()
        .border_1()
        .rounded(cx.theme().radius)
        .border_color(cx.theme().colors.border)
        .cursor_pointer()
        .hover(|frame| frame.bg(cx.theme().secondary));
    activate(frame, item.name(), cx, move |view, window, cx| {
        view.navigate(route.clone(), window, cx);
    })
}

/// "Folder · Installed 3 days ago · Modified 2 hours ago"; each date's tooltip shows the exact
/// time.
fn dates(item: &SkillItem<'_>, cx: &App) -> impl IntoElement {
    let now = Utc::now();
    let parts = [
        ("installed", "Installed", item.imported),
        ("modified", "Modified", item.modified),
    ];
    // Wraps so a narrow card keeps both dates inside it.
    let mut row = h_flex()
        .flex_wrap()
        .gap_x_1()
        .text_xs()
        .text_color(cx.theme().muted_foreground);
    let filed = item.folder.is_some();
    if let Some(folder) = item.folder {
        row = row.child(
            h_flex()
                .gap_1()
                .child(app_icon("folder").xsmall())
                .child(folder.to_owned()),
        );
    }
    for (index, (key, label, time)) in parts
        .into_iter()
        .filter_map(|(key, label, time)| Some((key, label, time?)))
        .enumerate()
    {
        let tooltip = exact(time);
        row = row.when(filed || index > 0, |row| row.child("·")).child(
            div()
                .id(format!("{key}-{}", item.skill.id))
                .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
                .child(format!("{label} {}", relative(time, now))),
        );
    }
    row
}

fn description(text: &str, lines: usize, cx: &App) -> impl IntoElement {
    div()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .map(|text| {
            if lines == 1 {
                text.truncate()
            } else {
                text.line_clamp(lines)
            }
        })
        .child(text.to_owned())
}

/// Agent chips, Universal first, for the Agents rows whose global skill directory holds a copy.
fn presence(item: &SkillItem<'_>, cx: &mut Context<NativeView>) -> AnyElement {
    let muted = |text: &'static str| {
        div()
            .text_xs()
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
        .flex_wrap()
        .gap_1()
        .children(
            holders
                .iter()
                .take(SHOWN_HOLDERS)
                .map(|holder| holder_chip(item, holder, cx)),
        )
        .when(!hidden.is_empty(), |row| {
            let names = hidden
                .iter()
                .map(|holder| holder.name.clone())
                .collect::<Vec<_>>()
                .join("\n");
            row.child(
                div()
                    .id(format!("holders-more-{}", item.skill.id))
                    .flex_shrink_0()
                    // The chip opens nothing, and the click must not open the skill either.
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .tooltip(move |window, cx| Tooltip::new(names.clone()).build(window, cx))
                    .child(Tag::secondary().small().child(format!("+{}", hidden.len()))),
            )
        })
        .into_any_element()
}

/// One agent's avatar and name; its tooltip shows the directory holding the copy, and for
/// Universal the agents reading it. Clicking it opens the agent's page.
fn holder_chip(item: &SkillItem<'_>, holder: &Holder, cx: &mut Context<NativeView>) -> AnyElement {
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
        .whitespace_nowrap()
        .cursor_pointer()
        .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx))
        .on_click(cx.listener(move |view, _, window, cx| {
            cx.stop_propagation();
            view.navigate(route.clone(), window, cx);
        }))
        .child(
            Tag::secondary()
                .small()
                .gap_1()
                .child(avatar.with_size(Size::XSmall).flex_shrink_0())
                .child(div().flex_shrink_0().child(holder.name.clone())),
        )
        .into_any_element()
}
