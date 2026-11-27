//! Reviewing one skill's update before it lands (ADR 0037): the review dialog's calls.

use crate::Client;
use skillbinder_proto::*;

impl Client {
    /// What updating skill `id` from its source would change; nothing is written.
    pub async fn review_update(&self, id: SkillId) -> AppResult<UpdateReview> {
        self.shared.engine.review_update(id).await
    }

    /// Updates the reviewed skill while its source and library content are what the review
    /// showed, then scans again, since the agents' copies no longer match it; otherwise returns
    /// the changes now, having written nothing.
    pub async fn update_reviewed(&self, reviewed: ReviewedUpdate) -> AppResult<ReviewedOutcome> {
        let outcome = self.shared.engine.update_reviewed(reviewed).await?;
        if let ReviewedOutcome::Updated(_) = &outcome {
            let outcome = self.refresh_preserving_outcome(Ok(outcome)).await?;
            self.spawn_scan();
            return Ok(outcome);
        }
        Ok(outcome)
    }
}
