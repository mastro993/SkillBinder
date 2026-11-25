//! The picker's Selected pane: a card per picked skill in pick order, with its agent pills in a
//! workspace, then Cancel and `Add <N>`. A card's All pill holds the skill for every agent the
//! workspace uses; the other pills tick directories, and pills reading one folder are one pill.

use super::super::NativeView;
use super::super::controls::reader_logo;
use super::super::library_places::place_pill;
use super::super::snapshot_query::Entry;
use super::{Pills, Resolved, Selection, SkillPicker};
use gpui_kit::component::{ActiveTheme, h_flex, v_flex};
use gpui_kit::{
    AnyElement, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, prelude::FluentBuilder, px, relative,
};
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{
    Button, DialogFooter, Level, Notice, ScrollArea, SectionHeader, Status, Surface, Text,
    ToggleChip, tooltip,
};
use skillbinder_proto::{SkillId, Workspace};
use std::collections::BTreeSet;

/// The Selected pane's share of the picker's width: layout measured from the approved skill
/// picker prototype.
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
        let no_agents =
            matches!(resolved, Resolved::Workspace { entries, .. } if entries.is_empty());
        let header = SectionHeader::new("selected", "Selected")
            .px(px(space::LG))
            .pt(px(space::LG));
        let header = if skills.is_empty() {
            header
        } else {
            header.count(skills.len())
        };
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
            ScrollArea::new("picker-cards")
                .flex_1()
                .min_h_0()
                .fill(Level::Flat.fill(theme))
                .child(v_flex().gap(px(space::MD)).p(px(space::LG)).children(cards))
                .into_any_element()
        };
        Surface::new(Level::Flat)
            .apply(v_flex(), theme)
            .id("picker-selected")
            .w(relative(PANE))
            .flex_none()
            .min_h_0()
            .child(header)
            .child(body)
            .when(no_agents, |pane| {
                pane.child(
                    Notice::new("picker-no-agents", "You have no agents.")
                        .px(px(space::LG))
                        .pb(px(space::MD)),
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
        let deselect = Button::close(
            format!("picker-deselect-{skill}"),
            format!("Deselect {skill}"),
        )
        .tooltip("Deselect")
        .on_click(cx.listener(move |view, _, _, cx| {
            let id = id.clone();
            view.edit_picker(|picker, uses| picker.selection.pick(id, uses), cx);
        }));
        Surface::new(Level::Flat)
            .apply(v_flex(), theme)
            .id(format!("picker-selected-{skill}"))
            .gap(px(space::MD))
            .p(px(space::LG))
            .child(
                h_flex()
                    .justify_between()
                    .gap(px(space::MD))
                    .child(
                        Text::labeled("slug", skill.to_string())
                            .level(text::TITLE_SM)
                            .min_w_0()
                            .truncate(),
                    )
                    .child(deselect),
            )
            .children(pills)
            .when(warn, |card| {
                card.child(
                    Notice::new(format!("picker-pick-{skill}"), "Pick an agent")
                        .status(Status::Warning),
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
        let all = all_pill(format!("picker-pill-{skill}-all"), pills.all)
            .on_change(move |_, event, window, cx| press(event, window, cx));
        let mut row = vec![all.into_any_element()];
        for entry in entries {
            let ticked = pills.ticked(&entry.keys);
            let id = format!("picker-pill-{skill}-{}", entry.keys.join("+"));
            let logo = reader_logo(&entry.row.reader);
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
            .gap(px(space::XS))
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
        let add = Button::new("picker-add")
            .label(label)
            .primary()
            .flex_1()
            .inactive(blocked.is_some() || self.blocked())
            .when_some(blocked, |add, reason| add.tooltip(reason))
            .on_click(cx.listener(|view, _, _, cx| view.add_picked(cx)));
        let cancel = Button::new("picker-cancel")
            .label("Cancel")
            .on_click(cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx)));
        DialogFooter::new()
            .secondary(cancel)
            .primary(add)
            .p(px(space::LG))
            .border_t_1()
            .border_color(cx.theme().border)
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

/// The All pill: a toggle chip reading `All`, accent while on and dashed while off, like a
/// directory pill.
fn all_pill(id: String, on: bool) -> ToggleChip {
    ToggleChip::new(id, on)
        .dashed()
        .accessibility_label("All")
        .h(px(space::CONTROL_SM))
        .child("All")
        .tooltip(tooltip("Includes agents added later"))
}
