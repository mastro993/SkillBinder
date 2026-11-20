//! The engine's side of app updates: checks on their own thread, and the channel and the
//! automatic-check switch saved through the worker.

use super::Engine;
use skillbinder_proto::*;

impl Engine {
    /// Checks the channel's manifest and waits for the check to end, unless the build never
    /// checks or a check is running. Only a `manual` check keeps its failure.
    pub async fn check_app_update(&self, manual: bool) -> AppResult<()> {
        let Some(finished) = self.owner.app_updates.check(manual)? else {
            return Ok(());
        };
        match finished.await.map_err(|_| AppError::storage())? {
            Some(checked) => {
                self.call(move |state| state.save_update_checked(checked))
                    .await
            }
            None => Ok(()),
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
}
