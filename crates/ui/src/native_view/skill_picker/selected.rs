//! The picker's Selected pane: a card per picked skill in pick order, with its agent pills in a
//! workspace, then Cancel and `Add <N>`. A card's All pill holds the skill for every agent the
//! workspace uses; the other pills tick directories, and pills reading one folder are one pill.

use super::super::NativeView;
use super::super::controls::{Inactive, Logo, app_icon, button, text_button, tooltip};
use super::super::library_places::{dashed, place_pill};
use super::super::snapshot_query::Entry;
use super::{Pills, Resolved, Selection, SkillPicker};
use gpui_kit::base::Toggle;
use gpui_kit::component::{
    ActiveTheme, IconName, Sizable, Size, button::ButtonVariants, h_flex,
    scroll::ScrollableElement, v_flex,
};
use gpui_kit::{
    AnyElement, App, ClickEvent, Context, FontWeight, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, Text, div, prelude::FluentBuilder, px,
    relative,
};
use skillbinder_proto::{SkillId, Workspace};
use skillbinder_theme::MEDIUM;
use std::collections::BTreeSet;

/// The Selected pane's share of the picker's width.
const PANE: f32 = 0.38;

impl NativeView {
    /// The Selected pane for `picker` at `resolved`.
    pub(super) fn picker_selected(
        &self,
        picker: &SkillPicker,
        resolved: &Resolved<'_>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let skills = picker.selection.skills();
        let cards: Vec<AnyElement> = match (&picker.selection, resolved) {
            (Selection::Workspace { cards, .. }, Resolved::Workspace { workspace, entries }) => {
                cards
                    .iter()
                    .map(|(skill, pills)| {
                        let pill_row = (!entries.is_empty())
                            .then(|| self.pill_row(skill, pills, workspace, entries, cx));
                        self.picker_card(skill, pill_row, pills.dirs.is_empty(), cx)
                    })
                    .collect()
            }
            (selection, _) => selection
                .skills()
                .into_iter()
                .map(|skill| self.picker_card(skill, None, false, cx))
                .collect(),
        };
        let theme = cx.theme();
        let undetected =
            matches!(resolved, Resolved::Workspace { entries, .. } if entries.is_empty());
        let header = h_flex()
            .gap(px(6.))
            .px(px(12.))
            .pt(px(12.))
            .font_weight(FontWeight::SEMIBOLD)
            .child(Text::new("selected".into(), "Selected".into()))
            .when(!skills.is_empty(), |header| {
                header.child(
                    div()
                        .font_weight(FontWeight::NORMAL)
                        .text_color(theme.muted_foreground)
                        .child(skills.len().to_string()),
                )
            });
        let body = if cards.is_empty() {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.muted_foreground)
                .child("Nothing selected")
                .into_any_element()
        } else {
            div()
                .id("picker-cards")
                .flex_1()
                .min_h_0()
                .child(v_flex().gap(px(8.)).p(px(12.)).children(cards))
                .overflow_y_scrollbar()
                .into_any_element()
        };
        v_flex()
            .id("picker-selected")
            .w(relative(PANE))
            .flex_none()
            .min_h_0()
            .bg(theme.sidebar)
            .border_1()
            .border_color(theme.border)
            .rounded(px(10.))
            .child(header)
            .child(body)
            .when(undetected, |pane| {
                pane.child(
                    div()
                        .px(px(12.))
                        .pb(px(8.))
                        .text_size(px(12.))
                        .text_color(theme.muted_foreground)
                        .child("No agents are detected on this device."),
                )
            })
            .child(self.picker_footer(&picker.selection, cx))
            .into_any_element()
    }

    /// One picked skill: its slug and ×, then its `pills` and, with none ticked, "Pick an agent".
    fn picker_card(
        &self,
        skill: &SkillId,
        pills: Option<AnyElement>,
        warn: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let id = skill.clone();
        let deselect = button(format!("picker-deselect-{skill}"))
            .ghost()
            .small()
            .icon(IconName::Close)
            .tooltip("Deselect")
            .accessibility_label(format!("Deselect {skill}"))
            .on_click(cx.listener(move |view, _, _, cx| {
                let id = id.clone();
                view.edit_picker(|picker, uses| picker.selection.pick(id, uses), cx);
            }));
        v_flex()
            .id(format!("picker-selected-{skill}"))
            .gap(px(8.))
            .p(px(10.))
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .rounded(px(8.))
            .child(
                h_flex()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(Text::new("slug".into(), skill.to_string().into())),
                    )
                    .child(deselect),
            )
            .children(pills)
            .when(warn, |card| {
                card.child(
                    h_flex()
                        .gap(px(6.))
                        .text_size(px(12.))
                        .text_color(theme.warning)
                        .child(app_icon("alert-circle").size(px(14.)))
                        .child("Pick an agent"),
                )
            })
            .into_any_element()
    }

    /// The All pill, then a pill per folder of `entries`, wrapping onto further lines.
    fn pill_row(
        &self,
        skill: &SkillId,
        pills: &Pills,
        workspace: &Workspace,
        entries: &[Entry],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let edit = |skill: SkillId, group: Option<Vec<String>>| {
            cx.listener(move |view: &mut Self, _: &ClickEvent, _, cx| {
                let (skill, group) = (skill.clone(), group.clone());
                view.edit_picker(
                    |picker, uses| {
                        picker.selection.edit_pills(&skill, |pills| match &group {
                            Some(group) => pills.toggle(group, uses),
                            None => pills.toggle_all(uses),
                        });
                    },
                    cx,
                );
            })
        };
        let press = edit(skill.clone(), None);
        let all = all_pill(format!("picker-pill-{skill}-all"), pills.all, cx)
            .on_change(move |_, event, window, cx| press(event, window, cx));
        let mut row = vec![all.into_any_element()];
        for entry in entries {
            let ticked = pills.ticked(&entry.keys);
            let id = format!("picker-pill-{skill}-{}", entry.keys.join("+"));
            let logo = Logo::reader(&entry.row.reader);
            let pill = place_pill(id, logo, ticked, false, pill_tooltip(entry, workspace), cx);
            let press = edit(skill.clone(), Some(entry.keys.clone()));
            row.push(
                pill.on_change(move |_, event, window, cx| press(event, window, cx))
                    .into_any_element(),
            );
        }
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(4.))
            .children(row)
            .into_any_element()
    }

    /// Cancel and `Add <N>`, which says why it is disabled.
    fn picker_footer(&self, selection: &Selection, cx: &mut Context<Self>) -> AnyElement {
        let count = selection.skills().len();
        let label = if count == 0 {
            "Add".to_owned()
        } else {
            format!("Add {count}")
        };
        let blocked = selection.blocked();
        let add = text_button("picker-add", label, Size::Medium)
            .primary()
            .flex_1()
            .inactive(blocked.is_some() || self.blocked())
            .when_some(blocked, |add, reason| add.tooltip(reason))
            .on_click(cx.listener(|view, _, _, cx| view.add_picked(cx)));
        h_flex()
            .gap(px(8.))
            .p(px(12.))
            .border_t_1()
            .border_color(cx.theme().border)
            .child(
                text_button("picker-cancel", "Cancel", Size::Medium)
                    .outline()
                    .on_click(cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx))),
            )
            .child(add)
            .into_any_element()
    }
}

/// A pill's tooltip: its name, the agents that read it when the name does not say so, then its
/// directory, " · not used here yet" when the workspace does not use it.
fn pill_tooltip(entry: &Entry, workspace: &Workspace) -> String {
    let uses: &BTreeSet<String> = &workspace.dirs;
    let unused = !entry.keys.iter().any(|dir| uses.contains(dir));
    let mut lines = vec![entry.row.name.clone()];
    lines.extend(entry.detail());
    let dir = entry.keys.join(", ");
    lines.push(if unused {
        format!("{dir} · not used here yet")
    } else {
        dir
    });
    lines.join("\n")
}

/// The All pill: Kit's base toggle reading `All`, solid while on and dashed while off, like a
/// directory pill.
fn all_pill(id: String, on: bool, cx: &App) -> Toggle {
    let theme = cx.theme();
    let (foreground, hover) = (theme.foreground, theme.muted);
    Toggle::new(id)
        .pressed(on)
        .accessibility_label("All")
        .flex_none()
        .h(px(24.))
        .px(px(10.))
        .rounded_full()
        .text_size(px(12.))
        .font_weight(MEDIUM)
        .map(|pill| {
            if on {
                pill.border_1()
                    .border_color(foreground)
                    .bg(theme.background)
                    .text_color(foreground)
            } else {
                dashed(pill, cx).text_color(theme.muted_foreground)
            }
        })
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .child("All")
        .tooltip(tooltip("Includes agents added later"))
}
