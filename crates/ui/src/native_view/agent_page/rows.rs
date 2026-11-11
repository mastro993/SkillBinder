//! An agent page row's trailing controls and its Delete button, which takes the page's directory
//! out of the active preset's entry for the skill at once. The toast then offers Undo, which saves
//! the entry the preset had, as [`super::super::undo`] describes.

use std::collections::BTreeSet;

use super::super::controls::{Inactive, app_icon, button};
use super::super::library_copies::{AgentRow, agent_rows};
use super::super::library_items::SkillItem;
use super::super::library_targets::{held, toggled};
use super::super::skill_picker::{Place, Resolved};
use super::super::undo::{Removal, Undo};
use super::super::{Effect, NativeView};
use gpui_kit::component::{Sizable, button::ButtonVariants, h_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, KeyDownEvent, ParentElement,
    StatefulInteractiveElement, Styled, prelude::FluentBuilder, px,
};
use skillbinder_proto::{AppSnapshot, OrganizationChange, PresetId, SkillId, Targets};

/// What an agent page's row shows beyond a Library row.
pub(in crate::native_view) struct AgentPageRow<'a> {
    pub place: &'a Place,
    /// The page's name, as Delete's label and the edited mark read it.
    pub name: &'a str,
    /// The skill's copy in the page's directory was changed there.
    pub edited: bool,
}

impl NativeView {
    /// The review glyph while a copy of the skill needs review, and Delete. Clicks and Enter or
    /// Space on them never open the skill's page.
    pub(in crate::native_view) fn agent_row_end(
        &self,
        item: &SkillItem<'_>,
        row: &AgentPageRow<'_>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = &item.skill.id;
        let unsettled = item
            .placement
            .map_or(0, |placement| placement.unsettled.len());
        let label = format!("Remove from {}", row.name);
        let (place, skill) = (row.place.clone(), id.clone());
        let remove = button(format!("agent-remove-{id}"))
            .ghost()
            .small()
            .icon(app_icon("delete"))
            .tooltip(label.clone())
            .accessibility_label(label)
            .inactive(self.blocked())
            .on_click(cx.listener(move |view, _, _, cx| {
                cx.stop_propagation();
                view.remove_from_agents(&place, skill.clone(), cx);
            }));
        h_flex()
            .id(format!("agent-row-end-{id}"))
            .flex_none()
            .items_center()
            .gap(px(8.))
            .on_click(|_, _, cx| cx.stop_propagation())
            .on_key_down(|event: &KeyDownEvent, _, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    cx.stop_propagation();
                }
            })
            .when(unsettled > 0, |end| {
                end.child(self.unsettled_glyph(id, unsettled, cx))
            })
            .child(remove)
            .into_any_element()
    }

    /// Takes `place`'s directory out of the active preset's entry for `skill` at once, keeping
    /// the entry for Undo. Nothing happens unless the preset holds the skill there now.
    pub(in crate::native_view) fn remove_from_agents(
        &mut self,
        place: &Place,
        skill: SkillId,
        cx: &mut Context<Self>,
    ) {
        if self.blocked() {
            return;
        }
        let Some(preset) = self.active_preset_id() else {
            return;
        };
        let Some(plan) = plan_removal(&self.snapshot, &preset, place, skill) else {
            return;
        };
        let RemovalPlan {
            targets,
            undo,
            row: AgentRow { name, resolved, .. },
        } = plan;
        let change = OrganizationChange::SetTargets {
            id: preset,
            skill: undo.skill().clone(),
            targets,
        };
        let removal = Removal {
            undo,
            dirs: BTreeSet::from([resolved]),
            place: name,
            asked: self.asked_copies.clone(),
        };
        let client = self.client.clone();
        self.run(
            async move {
                client.change_organization(change).await?;
                Ok(Effect::Removed(removal))
            },
            cx,
        );
    }
}

/// What Delete on `place`'s page saves for one skill, and what Undo saves back.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct RemovalPlan {
    /// The preset's entry for the skill after the removal.
    pub targets: Targets,
    pub undo: Undo,
    /// The page's Agents row, whose directory the removal leaves.
    pub row: AgentRow,
}

/// Takes `place`'s directory out of `preset`'s entry for `skill`: from all agents, the entry
/// becomes the other detected agents; from a list, the list without the agents reading the
/// directory. An empty list takes the skill out of the preset. `None` while the preset does not
/// hold the skill there, since toggling would then add it, as a click on a stale page could.
pub(super) fn plan_removal(
    snapshot: &AppSnapshot,
    preset: &PresetId,
    place: &Place,
    skill: SkillId,
) -> Option<RemovalPlan> {
    let Some(Resolved::Agents { row, ids }) = Resolved::of(snapshot, place) else {
        return None;
    };
    let previous = snapshot
        .library
        .as_ref()?
        .library
        .skills
        .iter()
        .find(|held| held.id == skill)?
        .presets
        .get(preset)?
        .clone();
    if !held(Some(&previous), &ids) {
        return None;
    }
    let rows = agent_rows(&snapshot.agents, &snapshot.home);
    let every: Vec<String> = rows.iter().flat_map(|row| row.ids.clone()).collect();
    Some(RemovalPlan {
        targets: toggled(Some(&previous), &ids, &every),
        undo: Undo::Preset {
            id: preset.clone(),
            skill,
            previous,
        },
        row,
    })
}
