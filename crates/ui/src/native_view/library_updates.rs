//! Skill updates in Library: the Update buttons of skills whose source has an update, Update all,
//! the last update's result, and why a check or update failed.

use super::without_git::{CHECK_NEEDS_GIT, needs_git};
use super::{NativeView, Route, snapshot_query};
use gpui_kit::component::{ActiveTheme, Sizable, Size, progress::ProgressCircle, v_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, px,
};
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{Button, IconSize, Notice, NoticeSize, Spinner, icon, tooltip};
use skillbinder_proto::{AppSnapshot, SkillId, UpdateCheck, UpdateOutcome};

/// The library skills with an update, in slug order.
pub(crate) fn available(snapshot: &AppSnapshot) -> Vec<SkillId> {
    snapshot
        .updates
        .iter()
        .filter(|(_, update)| update.available())
        .map(|(id, _)| id.clone())
        .collect()
}

/// The outcome of an Update all that finished between `shown` and `latest`: a new outcome, or
/// the same as the last one once a run ends.
fn finished_update_all<'a>(
    shown: &AppSnapshot,
    latest: &'a AppSnapshot,
) -> Option<&'a UpdateOutcome> {
    latest.update_all_outcome.as_ref().filter(|outcome| {
        shown.updating.is_some() || shown.update_all_outcome.as_ref() != Some(*outcome)
    })
}

/// "Updated 2 · 1 failed".
pub(crate) fn summary(outcome: &UpdateOutcome) -> String {
    format!(
        "Updated {} · {} failed",
        outcome.updated.len(),
        outcome.failed.len()
    )
}

impl NativeView {
    /// Whether skill `id` has an update.
    pub(super) fn has_update(&self, id: &SkillId) -> bool {
        snapshot_query::has_update(&self.snapshot, id)
    }

    /// A skill's Update button; its click opens the review of that skill's update.
    pub(super) fn update_button(
        &self,
        key: &str,
        id: &SkillId,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let skill = id.clone();
        let update = Button::new(format!("update-{key}-{id}"))
            .label("Update")
            .small();
        if self.git_missing() {
            return needs_git(
                format!("update-{key}-{id}-needs-git"),
                CHECK_NEEDS_GIT,
                update,
            );
        }
        update
            .inactive(self.working())
            .on_click(cx.listener(move |view, _, _, cx| {
                // The row or card under the button would open the skill's page.
                cx.stop_propagation();
                view.review_update(skill.clone(), cx);
            }))
            .into_any_element()
    }

    /// Library's header update control, then Scan for skills.
    pub(super) fn library_header_actions(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        vec![self.library_update_button(cx), self.scan_button(cx)]
    }

    /// The header's update control. It checks when nothing is known, updates when a skill has an
    /// update, and shows the run's progress while either is in flight.
    fn library_update_button(&self, cx: &mut Context<Self>) -> AnyElement {
        if let Some((done, total)) = self.snapshot.updating {
            return div()
                .id("library-update-progress")
                .flex()
                .items_center()
                .justify_center()
                .size(px(space::CONTROL_SM))
                .tooltip(tooltip(format!("Updating {done} of {total} skills")))
                .child(
                    ProgressCircle::new("library-update-circle")
                        .value(Self::percent(done, total))
                        .with_size(Size::Size(px(space::CONTROL_SM))),
                )
                .into_any_element();
        }
        if self.snapshot.checking {
            return div()
                .id("library-checking")
                .flex()
                .items_center()
                .justify_center()
                .size(px(space::CONTROL_SM))
                .tooltip(tooltip("Checking skill updates"))
                .child(spinner())
                .into_any_element();
        }
        if self.git_missing() {
            let check = Button::new("library-check-updates")
                .small()
                .ghost()
                .icon(icon("refresh-04"));
            return needs_git("library-check-updates-needs-git", CHECK_NEEDS_GIT, check);
        }
        if available(&self.snapshot).is_empty() {
            Button::new("library-check-updates")
                .small()
                .ghost()
                .icon(icon("refresh-04"))
                .inactive(self.working())
                .tooltip("Check for skill updates")
                .on_click(cx.listener(|view, _, _, cx| view.check_for_updates(cx)))
                .into_any_element()
        } else {
            Button::new("library-update-skills")
                .small()
                .ghost()
                .icon(icon("upload-circle"))
                .inactive(self.working())
                .tooltip("Update skills")
                .on_click(cx.listener(|view, _, _, cx| view.update_all(cx)))
                .into_any_element()
        }
    }

    /// Checks every source now and shows the results as the snapshot they publish.
    fn check_for_updates(&mut self, cx: &mut Context<Self>) {
        if self.blocked() || self.snapshot.checking {
            return;
        }
        self.error = None;
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = client.check_now().await;
            let _ = this.update(cx, |view, cx| {
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

    /// Starts Update all in the client, which counts it on the snapshot whatever window or menu
    /// is open meanwhile; its outcome reaches the page from there too, as one the menu started
    /// does. Refused while a change, short work, or a Git operation runs.
    pub(crate) fn update_all(&mut self, cx: &mut Context<Self>) {
        if self.blocked() || self.snapshot.syncing.is_some() {
            return;
        }
        self.error = None;
        let run = self.client.update_all();
        // The run has claimed the snapshot already, so the header counts it from this frame.
        self.receive(self.client.snapshot(), cx);
        cx.spawn(async move |this, cx| {
            if let Err(error) = run.await {
                let _ = this.update(cx, |view, cx| {
                    view.error = Some(error);
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Library with the Updates filter on and the last Update all's outcome, where the menu's
    /// `Updated n · m failed` leads.
    pub(crate) fn show_library_updates(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.library_filter.updates = true;
        self.navigate(Route::Library, window, cx);
        self.update_outcome = self.snapshot.update_all_outcome.clone();
    }

    /// Shows a finished Update all's outcome, from the window or the menu, as the page's last
    /// update result; an updated skill's page shows its new content.
    pub(super) fn follow_update_all(&mut self, latest: &AppSnapshot, cx: &mut Context<Self>) {
        let Some(outcome) = finished_update_all(&self.snapshot, latest) else {
            return;
        };
        if let Route::Skill(id) = &self.route
            && outcome.updated.contains(id)
        {
            self.load_skill(id.clone(), cx);
        }
        self.update_outcome = Some(outcome.clone());
    }

    /// `done` of `total` as a percentage.
    fn percent(done: usize, total: usize) -> f32 {
        if total == 0 {
            100.
        } else {
            done as f32 / total as f32 * 100.
        }
    }

    /// A skill page's activation, its Reinstall button, while it can restore the skill's recorded
    /// content, and its Update button, while the skill has an update.
    pub(super) fn skill_header_actions(
        &self,
        id: &SkillId,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let mut actions: Vec<AnyElement> = self.header_place_trigger(id, cx).into_iter().collect();
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
            text::BODY_MD
                .apply(div())
                .text_color(cx.theme().muted_foreground)
                .child(summary(outcome))
                .into_any_element(),
        )
    }

    /// Why skill `id`'s last check or update failed, if either did.
    pub(super) fn render_update_notes(&self, id: &SkillId) -> Option<AnyElement> {
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
                .gap(px(space::XS))
                .children(
                    check.map(|check| Notice::new("update-check-note", check).size(NoticeSize::Md)),
                )
                .children(failure.map(|failure| {
                    Notice::new("update-failure-note", failure).size(NoticeSize::Md)
                }))
                .into_any_element(),
        )
    }
}

/// The check glyph turning while a check, a scan, or a Git setup runs.
pub(super) fn spinner() -> Spinner {
    Spinner::new(IconSize::Md).glyph("refresh-04")
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

    #[test]
    fn a_finished_update_all_shows_its_outcome_once() {
        let id = |slug: &str| SkillId(slug.to_owned());
        let snapshot = |updating, outcome: Option<&UpdateOutcome>| {
            let mut snapshot = crate::native_view::fixture::Fixture::new().build();
            snapshot.updating = updating;
            snapshot.update_all_outcome = outcome.cloned();
            snapshot
        };
        let updated = UpdateOutcome {
            updated: vec![id("a")],
            failed: Vec::new(),
        };
        let done = snapshot(None, Some(&updated));
        assert_eq!(
            finished_update_all(&snapshot(Some((1, 2)), None), &done),
            Some(&updated)
        );
        // A run whose start the window missed ends with the outcome it showed last.
        assert_eq!(
            finished_update_all(&snapshot(Some((1, 2)), Some(&updated)), &done),
            Some(&updated)
        );
        assert_eq!(finished_update_all(&done, &done), None);
        let failed = UpdateOutcome {
            updated: Vec::new(),
            failed: vec![id("b")],
        };
        let later = snapshot(None, Some(&failed));
        assert_eq!(finished_update_all(&done, &later), Some(&failed));
        // A source check that drops the outcome leaves the page's as it is.
        assert_eq!(finished_update_all(&done, &snapshot(None, None)), None);
    }
}
