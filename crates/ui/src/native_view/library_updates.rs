//! Skill updates in Library: the Needs update filter, the Update buttons of skills whose source
//! has an update, Update all, the last update's result, and why a check or update failed.

use super::controls::{Inactive, app_icon, button};
use super::{Effect, NativeView, Route};
use gpui_kit::component::{ActiveTheme, Sizable, button::ButtonVariants, h_flex, v_flex};
use gpui_kit::{
    AnyElement, Context, IntoElement, ParentElement, Styled, WeakEntity, div,
    prelude::FluentBuilder,
};
use skillbinder_proto::{AppSnapshot, Skill, SkillId, UpdateCheck, UpdateOutcome};

/// The library skills with an update, in slug order.
pub(super) fn available(snapshot: &AppSnapshot) -> Vec<SkillId> {
    snapshot
        .updates
        .iter()
        .filter(|(_, update)| update.available())
        .map(|(id, _)| id.clone())
        .collect()
}

/// "Updated 2 · 1 failed".
fn summary(outcome: &UpdateOutcome) -> String {
    format!(
        "Updated {} · {} failed",
        outcome.updated.len(),
        outcome.failed.len()
    )
}

impl NativeView {
    /// Whether skill `id` has an update.
    pub(super) fn has_update(&self, id: &SkillId) -> bool {
        self.snapshot
            .updates
            .get(id)
            .is_some_and(|update| update.available())
    }

    /// Updates `ids` from their sources, keeping the result for the page to show.
    pub(super) fn update(&mut self, ids: Vec<SkillId>, cx: &mut Context<Self>) {
        let client = self.client.clone();
        self.run(
            async move { client.update_skills(ids).await.map(Effect::Updated) },
            cx,
        );
    }

    /// A skill's Update button; its click updates only that skill.
    pub(super) fn update_button(
        &self,
        key: &str,
        id: &SkillId,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let skill = id.clone();
        button(format!("update-{key}-{id}"))
            .label("Update")
            .small()
            .inactive(self.busy)
            .on_click(cx.listener(move |view, _, _, cx| {
                // The row or card under the button would open the skill's page.
                cx.stop_propagation();
                view.update(vec![skill.clone()], cx);
            }))
            .into_any_element()
    }

    /// The All and Needs update filters; Needs update counts the `skills` with an update.
    pub(super) fn update_filter_tabs(
        &self,
        skills: &[&Skill],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let weak = cx.entity().downgrade();
        let count = skills
            .iter()
            .filter(|skill| self.has_update(&skill.id))
            .count();
        let (muted, foreground) = (cx.theme().muted, cx.theme().foreground);
        let (border, muted_foreground) = (cx.theme().border, cx.theme().muted_foreground);
        let chip = |index: usize, label: String, weak: WeakEntity<NativeView>| {
            let selected = usize::from(self.needs_update_only) == index;
            button(format!("update-filter-{index}"))
                .small()
                .ghost()
                .label(label)
                .when(selected, |chip| chip.bg(muted).text_color(foreground))
                .when(!selected, |chip| {
                    chip.border_1()
                        .border_color(border)
                        .text_color(muted_foreground)
                })
                .on_click(move |_, _, cx| {
                    let _ = weak.update(cx, |view, cx| {
                        view.needs_update_only = index == 1;
                        cx.notify();
                    });
                })
        };
        h_flex()
            .gap_1()
            .child(chip(0, format!("All {}", skills.len()), weak.clone()))
            .child(chip(1, format!("Needs update {count}"), weak))
            .into_any_element()
    }

    /// Library's "Import", which opens Discovery, where copies found on this device are reviewed
    /// before they join the library.
    fn import_button(&self, cx: &mut Context<Self>) -> AnyElement {
        button("library-import")
            .small()
            .ghost()
            .icon(app_icon("arrow-down-to-line"))
            .label("Import")
            .on_click(cx.listener(|view, _, window, cx| {
                view.navigate(Route::Discovery, window, cx);
            }))
            .into_any_element()
    }

    /// Library's "Update all (N)" while N is at least one, then Import.
    pub(super) fn library_header_actions(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let mut actions = Vec::new();
        let ids = available(&self.snapshot);
        if !ids.is_empty() {
            actions.push(
                button("update-all")
                    .icon(app_icon("upload-circle"))
                    .label(format!("Update all ({})", ids.len()))
                    .small()
                    .primary()
                    .inactive(self.busy)
                    .on_click(cx.listener(move |view, _, _, cx| view.update(ids.clone(), cx)))
                    .into_any_element(),
            );
        }
        actions.push(self.import_button(cx));
        actions
    }

    /// A skill page's Reinstall button, while it can restore the skill's recorded content, and
    /// its Update button, while the skill has an update.
    pub(super) fn skill_header_actions(
        &self,
        id: &SkillId,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let mut actions: Vec<AnyElement> = vec![self.activation_toggle(id, cx)];
        actions.extend(self.reinstall_button(id, cx));
        if self.has_update(id) {
            actions.push(self.update_button("page", id, cx));
        }
        actions
    }

    /// The last update's result, until the page changes.
    pub(super) fn render_update_outcome(&self, cx: &Context<Self>) -> Option<AnyElement> {
        let outcome = self.update_outcome.as_ref()?;
        Some(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(summary(outcome))
                .into_any_element(),
        )
    }

    /// Why skill `id`'s last check or update failed, if either did.
    pub(super) fn render_update_notes(
        &self,
        id: &SkillId,
        cx: &Context<Self>,
    ) -> Option<AnyElement> {
        let update = self.snapshot.updates.get(id)?;
        let check = match &update.check {
            UpdateCheck::Failed(message) => Some(message.clone()),
            UpdateCheck::UpToDate | UpdateCheck::Available => None,
        };
        let failure = update
            .failure
            .as_ref()
            .map(|message| format!("Couldn't update: {message}"));
        if check.is_none() && failure.is_none() {
            return None;
        }
        Some(
            v_flex()
                .gap_1()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .children(check)
                .children(failure)
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_result_counts_updated_and_failed_skills() {
        let id = |slug: &str| SkillId(slug.to_owned());
        let outcome = UpdateOutcome {
            updated: vec![id("a"), id("b")],
            failed: vec![id("c")],
        };
        assert_eq!(summary(&outcome), "Updated 2 · 1 failed");
    }
}
