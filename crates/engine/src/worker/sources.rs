//! The engine's skill source operations: update checks on their own thread, Update, Reinstall,
//! and Sync's Restore and Discard, which replace a skill's content the same way.

use super::{
    Engine,
    action::{Action, list},
};
use crate::{
    logging::{Area, Level},
    storage,
};
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
        self.action(Action::CheckUpdates, Vec::new(), |engine| async move {
            engine.check_sources(max_age).await
        })
        .await
    }
    /// Checks as [`Self::check_updates`] does, within its action.
    async fn check_sources(&self, max_age: std::time::Duration) -> AppResult<bool> {
        let started = self
            .call(move |state| {
                let Some(git) = state.git.clone() else {
                    return Ok(Ok(None));
                };
                if state.updates.running()? {
                    return Ok(Err(()));
                }
                if !state.onboarding_finished()? {
                    return Ok(Ok(None));
                }
                if !state.recovery_required
                    && let Err(error) = state.learn_sources()
                {
                    state
                        .log
                        .event(Level::Warn, Area::Library, "source learning failed")
                        .error(&error);
                }
                let skills = state.read_library()?.skills;
                let Some((due, running)) = state.updates.begin(&skills, max_age.as_secs())? else {
                    return Ok(Ok(None));
                };
                // Started on the worker, so no check is ever marked running without a thread.
                // The thread owns `running`, which ends the check however the thread exits.
                let log = state.log.clone();
                let data_dir = state.config.data_dir.clone();
                let (done, finished) = oneshot::channel();
                std::thread::Builder::new()
                    .name("skillbinder-updates".into())
                    .spawn(move || {
                        running.finish(crate::sources::check(&git, &data_dir, due, &log));
                        let _ = done.send(());
                    })
                    .map_err(storage::error("starting the update check thread"))?;
                Ok(Ok(Some(finished)))
            })
            .await?;
        match started {
            Err(()) => Ok(false),
            Ok(None) => Ok(true),
            Ok(Some(finished)) => finished
                .await
                .map(|()| true)
                .map_err(storage::error("waiting for the update check")),
        }
    }
    /// Replaces each selected library skill's content with its source's current content.
    pub async fn update_skills(&self, ids: Vec<SkillId>) -> AppResult<UpdateOutcome> {
        let subject = vec![("slugs", list(&ids))];
        self.action(Action::UpdateSkills, subject, |engine| async move {
            engine.call(move |state| state.update_skills(ids)).await
        })
        .await
    }
    /// Restores a drifted library skill's recorded content.
    pub async fn reinstall_skill(&self, id: SkillId) -> AppResult<()> {
        let subject = vec![("slug", id.to_string())];
        self.action(Action::ReinstallSkill, subject, |engine| async move {
            engine.call(move |state| state.reinstall_skill(id)).await
        })
        .await
    }
    /// Puts skill `id` back to its version in the History change `commit`, or brings it back
    /// from the change before when `commit` removed it, as a change on this device (ADR 0040).
    pub async fn restore_skill(&self, id: SkillId, commit: String) -> AppResult<()> {
        let subject = vec![("slug", id.to_string()), ("commit", commit.clone())];
        self.action(Action::RestoreSkill, subject, |engine| async move {
            engine
                .call(move |state| state.restore_version(id.as_str(), &commit))
                .await
        })
        .await
    }
    /// Undoes this device's change to skill `id` not pushed yet, putting it back as it was at
    /// the base; one added since then leaves the library, its copies kept unmanaged (ADR 0040).
    pub async fn discard_change(&self, id: SkillId) -> AppResult<()> {
        let subject = vec![("slug", id.to_string())];
        self.action(Action::DiscardChange, subject, |engine| async move {
            engine
                .call(move |state| state.discard_change(id.as_str()))
                .await
        })
        .await
    }
}
