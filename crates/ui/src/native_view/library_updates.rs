//! Skill updates in Library: the Update buttons of skills whose source has an update, Update all,
//! the last update's result, and why a check or update failed.

use super::controls::{Inactive, app_icon, button};
use super::{Effect, NativeView, Route};
use gpui_kit::component::{
    ActiveTheme, Sizable, Size, button::ButtonVariants, progress::ProgressCircle, tooltip::Tooltip,
    v_flex,
};
use gpui_kit::{
    Animation, AnimationExt, AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, px,
};
use skillbinder_proto::{AppSnapshot, SkillId, UpdateCheck, UpdateOutcome};

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

    /// Library's "Import", which opens Discovery, where copies found on this device are reviewed
    /// before they join the library.
    fn import_button(&self, cx: &mut Context<Self>) -> AnyElement {
        button("library-import")
            .small()
            .ghost()
            .icon(app_icon("import"))
            .accessibility_label("Import")
            .tooltip("Import")
            .on_click(cx.listener(|view, _, window, cx| {
                view.navigate(Route::Discovery, window, cx);
            }))
            .into_any_element()
    }

    /// Library's header update control, then Import.
    pub(super) fn library_header_actions(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        vec![self.library_update_button(cx), self.import_button(cx)]
    }

    /// The header's update control. It checks when nothing is known, updates when a skill has an
    /// update, and shows the run's progress while either is in flight.
    fn library_update_button(&self, cx: &mut Context<Self>) -> AnyElement {
        if let Some((done, total)) = self.updating {
            return div()
                .id("library-update-progress")
                .flex()
                .items_center()
                .justify_center()
                .size(px(24.))
                .tooltip(move |window, cx| {
                    Tooltip::new(format!("Updating {done} of {total} skills")).build(window, cx)
                })
                .child(
                    ProgressCircle::new("library-update-circle")
                        .value(Self::percent(done, total))
                        .with_size(Size::Size(px(24.))),
                )
                .into_any_element();
        }
        if self.checking {
            return div()
                .id("library-checking")
                .flex()
                .items_center()
                .justify_center()
                .size(px(24.))
                .tooltip(|window, cx| Tooltip::new("Checking skill updates").build(window, cx))
                .child(Self::spinning_refresh())
                .into_any_element();
        }
        if available(&self.snapshot).is_empty() {
            button("library-check-updates")
                .small()
                .ghost()
                .icon(app_icon("refresh-04"))
                .inactive(self.busy)
                .tooltip("Check for skill updates")
                .on_click(cx.listener(|view, _, _, cx| view.check_for_updates(cx)))
                .into_any_element()
        } else {
            button("library-update-skills")
                .small()
                .ghost()
                .icon(app_icon("upload-circle"))
                .inactive(self.busy)
                .tooltip("Update skills")
                .on_click(cx.listener(|view, _, _, cx| view.update_available(cx)))
                .into_any_element()
        }
    }

    /// Checks every source now and shows the results as the snapshot they publish.
    fn check_for_updates(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.checking {
            return;
        }
        self.checking = true;
        self.error = None;
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = client.check_now().await;
            let _ = this.update(cx, |view, cx| {
                view.checking = false;
                if let Err(error) = result {
                    view.error = Some(error);
                }
                view.receive(view.client.snapshot(), cx);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Updates every skill with an update, one at a time, so the header can count them.
    fn update_available(&mut self, cx: &mut Context<Self>) {
        let ids = available(&self.snapshot);
        if ids.is_empty() || self.busy || self.updating.is_some() {
            return;
        }
        let total = ids.len();
        self.busy = true;
        self.error = None;
        self.updating = Some((0, total));
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let mut outcome = UpdateOutcome::default();
            for (index, id) in ids.into_iter().enumerate() {
                match client.update_skill(id).await {
                    Ok(one) => {
                        outcome.updated.extend(one.updated);
                        outcome.failed.extend(one.failed);
                    }
                    Err(error) => {
                        let _ = this.update(cx, |view, _| view.error = Some(error));
                        break;
                    }
                }
                let _ = this.update(cx, |view, cx| {
                    view.updating = Some((index + 1, total));
                    view.receive(view.client.snapshot(), cx);
                    cx.notify();
                });
            }
            // The agents' copies no longer match what was updated, so scan once at the end.
            client.spawn_scan();
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                view.updating = None;
                if let Route::Skill(id) = &view.route
                    && outcome.updated.contains(id)
                {
                    view.load_skill(id.clone(), cx);
                }
                view.update_outcome = Some(outcome);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// `done` of `total` as a percentage.
    fn percent(done: usize, total: usize) -> f32 {
        if total == 0 {
            100.
        } else {
            done as f32 / total as f32 * 100.
        }
    }

    /// The check glyph turning once a second while a check runs.
    fn spinning_refresh() -> impl IntoElement {
        app_icon("refresh-04").with_animation(
            "library-check-spin",
            Animation::new(std::time::Duration::from_secs(1)).repeat(),
            |icon, delta| icon.rotate(gpui_kit::radians(delta * std::f32::consts::TAU)),
        )
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
