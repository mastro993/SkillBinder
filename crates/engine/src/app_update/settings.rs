//! The device settings app updates keep: the channel, the automatic-check switch, the last check,
//! the staged version, and the version that last ran.

use super::{check::LastCheck, manifest::Offer};
use crate::worker::State;
use skillbinder_proto::{AppResult, AppUpdateState, AppVersion, Channel};

pub(super) const CHANNEL: &str = "update_channel";
const AUTO_CHECK: &str = "update_auto_check";
pub(super) const LAST_CHECK: &str = "update_last_check";
const LAST_RUN: &str = "last_run_version";
/// The staged update, saved so a relaunch shows it ready.
const READY: &str = "update_ready";
/// Where a version's release notes are, when no manifest named them.
const RELEASES: &str = "https://github.com/mastro993/SkillBinder/releases/tag/v";

impl State {
    /// Takes the saved channel, switch, last check, and staged update over, notes whether this
    /// is the first launch of a newer version, and records the running version.
    pub(crate) fn restore_app_updates(&self) -> AppResult<()> {
        let channel: Option<Channel> = self.setting(CHANNEL)?;
        let auto_check: Option<bool> = self.setting(AUTO_CHECK)?;
        let last_check: Option<LastCheck> = self.setting(LAST_CHECK)?;
        let ready: Option<Offer> = self.setting(READY)?;
        let current = self.config.app_version;
        // A value an older build saved that no longer reads is not a version that ran.
        let last_run: Option<AppVersion> = self.setting(LAST_RUN).ok().flatten();
        self.set_setting(LAST_RUN, &current)?;
        let updated = last_run.is_some_and(|last| last < current).then(|| {
            ready
                .as_ref()
                .filter(|offer| offer.version == current)
                .map_or_else(
                    || format!("{RELEASES}{current}"),
                    |offer| offer.notes_url.clone(),
                )
        });
        let updates = &self.app_updates;
        if !updates.restore_staged(ready.clone())? && ready.is_some() {
            self.remove_setting(READY)?;
        }
        let mut shared = updates.lock()?;
        shared.updated = updated;
        if let Some(channel) = channel {
            shared.channel = channel;
        }
        if let Some(auto_check) = auto_check {
            shared.auto_check = auto_check;
        }
        let last_check = last_check.filter(|check| check.channel == shared.channel);
        if let Some(LastCheck { checked, offer, .. }) = last_check {
            shared.last_checked = Some(checked);
            // A download-only offer shows until the next check, unless this build passed it.
            if let Some(Offer { version, notes_url }) = offer
                && shared.state == (AppUpdateState::Idle { last_failure: None })
                && version > current
                && let Err(reason) = shared.location.staging()
            {
                shared.state = AppUpdateState::DownloadOnly {
                    version,
                    notes_url,
                    reason,
                };
            }
        }
        Ok(())
    }
    /// Saves `channel`, forgetting when the other channel was last checked.
    pub(crate) fn save_update_channel(&self, channel: Channel) -> AppResult<()> {
        if self.setting::<Channel>(CHANNEL)? != Some(channel) {
            self.remove_setting(LAST_CHECK)?;
        }
        self.set_setting(CHANNEL, &channel)
    }
    pub(crate) fn save_update_auto_check(&self, on: bool) -> AppResult<()> {
        self.set_setting(AUTO_CHECK, &on)
    }
    /// Saves a check that read the manifest, and the update it staged, if any.
    pub(crate) fn save_update_check(
        &self,
        check: &LastCheck,
        ready: Option<&Offer>,
    ) -> AppResult<()> {
        self.set_setting(LAST_CHECK, check)?;
        match ready {
            Some(offer) => self.set_setting(READY, offer),
            None => Ok(()),
        }
    }
}
