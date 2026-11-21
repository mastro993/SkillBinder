//! The engine's side of app updates: checks and downloads on their own thread, the channel, the
//! automatic-check switch, and the staged update saved through the worker, and the install at
//! quit.

use super::Engine;
use crate::{QuitCause, Relaunch, app_update::Ended};
use skillbinder_proto::*;

impl Engine {
    /// Checks the channel's manifest and waits for the check to end, unless the build never
    /// checks or a check is running; where the app can replace itself, that includes downloading
    /// and staging a newer version. Only a `manual` check keeps its failure. A check the channel
    /// changed under is followed by one of the new channel.
    pub async fn check_app_update(&self, manual: bool) -> AppResult<()> {
        loop {
            let Some(finished) = self.owner.app_updates.check(manual)? else {
                return Ok(());
            };
            match finished.await.map_err(|_| AppError::storage())? {
                Ended::Checked { check, ready } => {
                    return self
                        .call(move |state| state.save_update_check(&check, ready.as_ref()))
                        .await;
                }
                Ended::Superseded => {}
                Ended::Failed => return Ok(()),
            }
        }
    }
    /// The running version, the channel, and what the last check found, without the worker.
    pub fn current_app_update(&self) -> AppResult<AppUpdateSnapshot> {
        self.owner.app_updates.current()
    }
    /// Saves the channel app update checks read.
    pub async fn set_update_channel(&self, channel: Channel) -> AppResult<()> {
        self.call(move |state| state.save_update_channel(channel))
            .await?;
        self.owner.app_updates.set_channel(channel)
    }
    /// Saves whether the app checks by itself.
    pub async fn set_update_auto_check(&self, on: bool) -> AppResult<()> {
        self.call(move |state| state.save_update_auto_check(on))
            .await?;
        self.owner.app_updates.set_auto_check(on)
    }
    /// Moves the app into Applications on a thread of its own (spec rule 21). The app then
    /// quits, and the desktop starts the moved copy through [`Self::finish_app_update`].
    pub async fn move_to_applications(&self) -> AppResult<()> {
        let updates = self.owner.app_updates.clone();
        let (done, moved) = tokio::sync::oneshot::channel();
        std::thread::Builder::new()
            .name("skillbinder-move".into())
            .spawn(move || {
                let _ = done.send(updates.move_to_applications());
            })
            .map_err(|_| AppError::storage())?;
        moved.await.map_err(|_| AppError::storage())?
    }
    /// Answers the offer to add SkillBinder to the applications menu (spec rule 23): `add` writes
    /// the menu entry. Either answer is remembered.
    pub async fn answer_applications_menu(&self, add: bool) -> AppResult<()> {
        self.call(move |state| state.answer_applications_menu(add))
            .await
    }
    /// Restart: opens the app again once it quits and installs the ready app update.
    pub fn request_restart(&self) -> AppResult<()> {
        self.owner.app_updates.request_restart()
    }
    /// Call once the app has quit for `cause` and the engine drained: installs a ready app
    /// update (spec rule 12), and says what to start, the moved copy after Move to Applications
    /// or the app again after Restart, once the instance lock is released. On Windows it starts
    /// the setup program instead, unless the session is ending.
    pub fn finish_app_update(&self, cause: QuitCause) -> Option<Relaunch> {
        self.owner.app_updates.finish(cause)
    }
}
