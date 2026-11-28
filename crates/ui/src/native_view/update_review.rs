//! The "Update <slug>" review dialog (ADR 0037, R1–R13): Update on one skill, from its card, its
//! page header, or its row's context menu, opens it. It fetches the source and shows the changes
//! from the skill's library content to the fetched content as one stacked list of files, and
//! nothing is written until its Update is clicked. Update lands only the version shown; when the
//! source or the library content moved since, the dialog shows the changes now and asks again.
//! A fetch that failed can be retried; content an update would refuse says why, without Retry.

mod notes;
mod render;

use super::{DialogState, Effect, NativeView, Route};
use gpui_kit::component::diff::DiffState;
use gpui_kit::{Context, Entity};
use skillbinder_design::{FileNote, diff_state};
use skillbinder_proto::{
    AppResult, Moved, ReviewedOutcome, ReviewedUpdate, SkillId, UpdateOutcome, UpdateReview,
};
use std::rc::Rc;

/// The open review dialog.
pub(super) struct ReviewDialog {
    id: SkillId,
    phase: Phase,
}

/// What the dialog shows.
enum Phase {
    /// The source is being fetched (R7).
    Fetching,
    /// The changes, while the dialog asks to update (R1–R4); `updating` while Update runs (R11).
    Ready {
        review: Box<UpdateReview>,
        diff: Entity<DiffState>,
        /// Why files show without their lines (R5).
        notes: Rc<[FileNote]>,
        /// Why these changes replaced the reviewed ones (R9).
        moved: Option<Moved>,
        updating: bool,
    },
    /// The review or Update failed (R10).
    Failed {
        message: String,
        /// Whether fetching again can help, as it can after a fetch failure but not a refusal.
        retry: bool,
    },
}

impl NativeView {
    /// Opens skill `id`'s review dialog and fetches its source.
    pub(super) fn review_update(&mut self, id: SkillId, cx: &mut Context<Self>) {
        self.dialog = DialogState::UpdateReview(Box::new(ReviewDialog {
            id: id.clone(),
            phase: Phase::Fetching,
        }));
        let client = self.client.clone();
        let fetched = id.clone();
        self.ask(
            async move { client.review_update(fetched).await },
            move |view, review, cx| view.reviewed_changes(&id, review, None, cx),
            cx,
        );
    }

    /// Whether Update in the dialog is refused: while another change, an Update all, or a Git
    /// operation runs (behavior 7).
    fn update_refused(&self) -> bool {
        self.blocked() || self.snapshot.syncing.is_some()
    }

    /// Updates the skill the dialog shows, as reviewed.
    fn update_reviewed(&mut self, cx: &mut Context<Self>) {
        if self.update_refused() {
            return;
        }
        let DialogState::UpdateReview(dialog) = &mut self.dialog else {
            return;
        };
        let Phase::Ready {
            review, updating, ..
        } = &mut dialog.phase
        else {
            return;
        };
        *updating = true;
        let reviewed = ReviewedUpdate::from(review.as_ref());
        let client = self.client.clone();
        self.run(
            async move {
                let outcome = client.update_reviewed(reviewed).await;
                Ok(Effect::Reviewed(Box::new(outcome)))
            },
            cx,
        );
    }

    /// Shows what Update did: an update that landed or failed closes the dialog, and the page
    /// shows the result as Update's always did (A5); changes that moved replace the reviewed ones.
    /// An error before any update, such as Git gone, shows as itself, since no fetch failed.
    pub(super) fn update_landed(
        &mut self,
        outcome: AppResult<ReviewedOutcome>,
        cx: &mut Context<Self>,
    ) {
        let DialogState::UpdateReview(dialog) = &self.dialog else {
            return;
        };
        let id = dialog.id.clone();
        let outcome = match outcome {
            Ok(outcome) => outcome,
            Err(error) => {
                if let DialogState::UpdateReview(dialog) = &mut self.dialog {
                    dialog.phase = Phase::Failed {
                        message: error.message,
                        retry: false,
                    };
                }
                cx.notify();
                return;
            }
        };
        match outcome {
            ReviewedOutcome::Updated(outcome) => {
                self.dialog = DialogState::None;
                self.show_update_outcome(outcome, cx);
            }
            ReviewedOutcome::Moved { moved, review } => {
                self.reviewed_changes(&id, Ok(*review), Some(moved), cx);
            }
        }
    }

    /// Shows an update's outcome on the page; an updated skill's page shows its new content.
    fn show_update_outcome(&mut self, outcome: UpdateOutcome, cx: &mut Context<Self>) {
        if let Route::Skill(id) = &self.route
            && outcome.updated.contains(id)
        {
            self.load_skill(id.clone(), cx);
        }
        self.update_outcome = Some(outcome);
    }

    /// Shows `review` in skill `id`'s open dialog, noting why when it `moved`; a dialog closed or
    /// opened for another skill meanwhile ignores it.
    fn reviewed_changes(
        &mut self,
        id: &SkillId,
        review: AppResult<UpdateReview>,
        moved: Option<Moved>,
        cx: &mut Context<Self>,
    ) {
        let DialogState::UpdateReview(dialog) = &mut self.dialog else {
            return;
        };
        if &dialog.id != id {
            return;
        }
        dialog.phase = match review {
            Ok(review) => {
                let notes = notes::notes(&review);
                match diff_state(&review.patch, &notes, cx) {
                    Ok(diff) => Phase::Ready {
                        review: Box::new(review),
                        diff,
                        notes,
                        moved,
                        updating: false,
                    },
                    Err(_) => Phase::Failed {
                        message: "These changes could not be shown.".to_owned(),
                        retry: false,
                    },
                }
            }
            Err(error) => Phase::Failed {
                message: fetch_failed(id, &error.message),
                retry: error.retryable,
            },
        };
        cx.notify();
    }
}

/// Why `id`'s changes can't be shown: "Couldn't fetch <slug>'s source: <reason>", which the
/// dialog and `update --diff` both say.
pub(crate) fn fetch_failed(id: &SkillId, reason: &str) -> String {
    format!("Couldn't fetch {id}'s source: {reason}")
}
