//! The engine's side of app updates: checks on their own thread, and the channel and the
//! automatic-check switch saved through the worker.

use super::Engine;
use crate::{Relaunch, app_update::Ended};
use skillbinder_proto::*;

impl Engine {
    /// Checks the channel's manifest and waits for the check to end, unless the build never
    /// checks or a check is running. Only a `manual` check keeps its failure. A check the
    /// channel changed under is followed by one of the new channel.
    pub async fn check_app_update(&self, manual: bool) -> AppResult<()> {
        loop {
            let Some(finished) = self.owner.app_updates.check(manual)? else {
                return Ok(());
            };
            match finished.await.map_err(|_| AppError::storage())? {
                Ended::Checked(check) => {
                    return self
                        .call(move |state| state.save_update_check(&check))
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
    /// quits, and the desktop starts the moved copy through [`Self::take_relaunch`].
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
    /// What to start once the app has quit after Move to Applications, once.
    pub fn take_relaunch(&self) -> Option<Relaunch> {
        self.owner.app_updates.take_relaunch()
    }
}
