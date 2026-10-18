//! The engine's skill source operations: update checks on their own thread, Update, and
//! Reinstall.

use super::Engine;
use skillbinder_proto::*;
use tokio::sync::oneshot;

impl Engine {
    /// How many scans completed since launch.
    pub fn completed_scans(&self) -> AppResult<u64> {
        self.owner.scans.completed()
    }
    /// Gives library skills without a source the one the lock file of a found copy with the
    /// same content records, then checks the sources of the library skills not checked for their
    /// current record within `max_age`, on a thread of its own, and waits for the results.
    /// Returns `false`, doing nothing, while another check runs, so the caller can retry.
    pub async fn check_updates(&self, max_age: std::time::Duration) -> AppResult<bool> {
        let started = self
            .call(move |state| {
                let Some(git) = state.git.clone() else {
                    return Ok(Ok(None));
                };
                if state.updates.running()? {
                    return Ok(Err(()));
                }
                if state.bootstrap()?.step != OnboardingStep::Complete {
                    return Ok(Ok(None));
                }
                if !state.recovery_required
                    && let Err(error) = state.learn_sources()
                {
                    state
                        .log
                        .record(format!("Learning sources failed: {error}"));
                }
                let skills = state.read_library()?.skills;
                let Some(due) = state.updates.begin(&skills, max_age.as_secs())? else {
                    return Ok(Ok(None));
                };
                // Started on the worker, so no check is ever marked running without a thread.
                let (updates, log) = (state.updates.clone(), state.log.clone());
                let data_dir = state.config.data_dir.clone();
                let (done, finished) = oneshot::channel();
                let spawned = std::thread::Builder::new()
                    .name("skillbinder-updates".into())
                    .spawn({
                        let updates = updates.clone();
                        move || {
                            // A check that panics still ends, so later checks can start.
                            let results = std::panic::catch_unwind(|| {
                                crate::sources::check(&git, &data_dir, due, &log)
                            });
                            updates.finish(results.unwrap_or_default());
                            let _ = done.send(());
                        }
                    });
                if spawned.is_err() {
                    updates.finish(Vec::new());
                    return Err(AppError::storage());
                }
                Ok(Ok(Some(finished)))
            })
            .await?;
        match started {
            Err(()) => Ok(false),
            Ok(None) => Ok(true),
            Ok(Some(finished)) => finished
                .await
                .map(|()| true)
                .map_err(|_| AppError::storage()),
        }
    }
    /// Replaces each selected library skill's content with its source's current content.
    pub async fn update_skills(&self, ids: Vec<SkillId>) -> AppResult<UpdateOutcome> {
        self.call(move |state| state.update_skills(ids)).await
    }
    /// Restores a drifted library skill's recorded content.
    pub async fn reinstall_skill(&self, id: SkillId) -> AppResult<()> {
        self.call(move |state| state.reinstall_skill(id)).await
    }
}
