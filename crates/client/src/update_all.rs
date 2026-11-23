//! Update all and the hourly source check (menu bar spec rules 9–11). Update all updates every
//! library skill with an update, one at a time, on the runtime, so it runs to its end whatever
//! awaits it, the window included. It publishes how far it has got and its outcome on the
//! snapshot, where the window and the menu bar menu both read them.

use crate::{Client, UPDATE_CHECK_INTERVAL};
use skillbinder_proto::*;
use std::{future::Future, sync::Arc};
use tokio::time::{Instant, MissedTickBehavior};

#[cfg(test)]
mod tests;

impl Client {
    /// Updates every skill with an update, then scans once, since the agents' copies no longer
    /// match what was updated. Refused while another Update all runs.
    pub fn update_all(&self) -> impl Future<Output = AppResult<UpdateOutcome>> + Send + 'static {
        let client = self.clone();
        self.start_update_all(move |id| {
            let client = client.clone();
            async move { client.update_skill(id).await }
        })
    }

    /// Update all, updating each skill with `update`. The run claims `updating` before this
    /// returns, so a second call refuses at once.
    pub(crate) fn start_update_all<F, U>(
        &self,
        update: F,
    ) -> impl Future<Output = AppResult<UpdateOutcome>> + Send + 'static
    where
        F: FnMut(SkillId) -> U + Send + 'static,
        U: Future<Output = AppResult<UpdateOutcome>> + Send,
    {
        let client = self.clone();
        let run = self.claim_update_all().map(|ids| {
            (!ids.is_empty()).then(|| {
                self.shared
                    .runtime
                    .spawn(async move { client.run_update_all(ids, update).await })
            })
        });
        async move {
            match run {
                Some(Some(run)) => run.await.unwrap_or_else(|_| Err(AppError::storage())),
                Some(None) => Ok(UpdateOutcome::default()),
                None => Err(AppError::new(
                    ErrorCategory::Busy,
                    "Update all is already running.",
                    "Wait for it to finish.",
                )),
            }
        }
    }

    /// The skills with an update, marking Update all running over them and dropping the last
    /// outcome unless there are none; `None` while one already runs.
    fn claim_update_all(&self) -> Option<Vec<SkillId>> {
        let mut claimed = None;
        self.shared.snapshots.send_if_modified(|current| {
            if current.updating.is_some() {
                return false;
            }
            let ids: Vec<SkillId> = current
                .updates
                .iter()
                .filter(|(_, update)| update.available())
                .map(|(id, _)| id.clone())
                .collect();
            if ids.is_empty() {
                claimed = Some(ids);
                return false;
            }
            let mut next = (**current).clone();
            next.updating = Some((0, ids.len()));
            next.update_all_outcome = None;
            next.generation = current.generation + 1;
            *current = Arc::new(next);
            claimed = Some(ids);
            true
        });
        claimed
    }

    /// Updates `ids` one at a time, counting each, and stops at the first error, which keeps the
    /// outcome so far.
    async fn run_update_all<F, U>(
        &self,
        ids: Vec<SkillId>,
        mut update: F,
    ) -> AppResult<UpdateOutcome>
    where
        F: FnMut(SkillId) -> U,
        U: Future<Output = AppResult<UpdateOutcome>>,
    {
        let total = ids.len();
        let mut outcome = UpdateOutcome::default();
        let mut stopped = None;
        for (index, id) in ids.into_iter().enumerate() {
            match update(id).await {
                Ok(one) => {
                    outcome.updated.extend(one.updated);
                    outcome.failed.extend(one.failed);
                }
                Err(error) => {
                    stopped = Some(error);
                    break;
                }
            }
            self.publish_update_all(Some((index + 1, total)), None);
        }
        if let Err(error) = self.start_scan().await {
            self.background_error(error);
        }
        self.publish_update_all(None, Some(outcome.clone()));
        match stopped {
            Some(error) => Err(error),
            None => Ok(outcome),
        }
    }

    /// Publishes Update all's progress and the last outcome.
    pub(crate) fn publish_update_all(
        &self,
        updating: Option<(usize, usize)>,
        outcome: Option<UpdateOutcome>,
    ) {
        self.shared.snapshots.send_if_modified(|current| {
            if current.updating == updating && current.update_all_outcome == outcome {
                return false;
            }
            let mut next = (**current).clone();
            next.updating = updating;
            next.update_all_outcome = outcome;
            next.generation = current.generation + 1;
            *current = Arc::new(next);
            true
        });
    }

    /// Checks the sources not checked within `max_age`. One that completes drops the last
    /// Update all's outcome, since the check found what is current (spec A3).
    pub(crate) async fn check_sources(&self, max_age: std::time::Duration) -> AppResult<bool> {
        let checked = self.run_check(max_age).await;
        if matches!(checked, Ok(true)) {
            // One step, so a run that finishes meanwhile keeps its `updating` cleared.
            self.shared.snapshots.send_if_modified(|current| {
                if current.update_all_outcome.is_none() {
                    return false;
                }
                let mut next = (**current).clone();
                next.update_all_outcome = None;
                next.generation = current.generation + 1;
                *current = Arc::new(next);
                true
            });
        }
        checked
    }

    /// Every hour from an hour after launch, whose scan already checks, checks the sources not
    /// checked within the hour, whether or not the window is open (spec rule 11).
    pub(crate) fn start_source_checks(&self) {
        let weak = Arc::downgrade(&self.shared);
        self.shared.runtime.spawn(async move {
            let start = Instant::now() + UPDATE_CHECK_INTERVAL;
            let mut interval = tokio::time::interval_at(start, UPDATE_CHECK_INTERVAL);
            interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
            loop {
                interval.tick().await;
                let Some(shared) = weak.upgrade() else {
                    break;
                };
                let client = Self { shared };
                let checked = client.check_sources(UPDATE_CHECK_INTERVAL).await;
                if let Err(error) = client.refresh_preserving_outcome(checked).await {
                    client.background_error(error);
                }
            }
        });
    }
}

/// Runs `check` until it runs: `Ok(false)` says another caller's check was running, so it tries
/// again once that one has had time to end.
pub(crate) async fn once_free<F, C>(mut check: F) -> AppResult<bool>
where
    F: FnMut() -> C,
    C: Future<Output = AppResult<bool>>,
{
    loop {
        match check().await {
            Ok(false) => tokio::time::sleep(std::time::Duration::from_millis(150)).await,
            checked => return checked,
        }
    }
}
