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
    /// current content within `max_age`, on a thread of its own, and waits for the results.
    /// Does nothing while another check runs.
    pub async fn check_updates(&self, max_age: std::time::Duration) -> AppResult<()> {
        let Some((git, data_dir, due, log)) = self
            .call(move |state| {
                let (Some(git), OnboardingStep::Complete) =
                    (state.git.clone(), state.bootstrap()?.step)
                else {
                    return Ok(None);
                };
                if !state.recovery_required
                    && let Err(error) = state.learn_sources()
                {
                    state
                        .log
                        .record(format!("Learning sources failed: {error}"));
                }
                let skills = state.read_library()?.skills;
                Ok(state
                    .updates
                    .begin(&skills, max_age.as_secs())?
                    .map(|due| (git, state.config.data_dir.clone(), due, state.log.clone())))
            })
            .await?
        else {
            return Ok(());
        };
        let updates = self.owner.updates.clone();
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
        finished.await.map_err(|_| AppError::storage())
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
