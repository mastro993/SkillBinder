//! App updates (ADR 0017, ADR 0018): reads the channel's signed manifest from the catalog's site
//! on a thread of its own, never the worker, and offers a newer version as download-only. A build
//! without the release stamp never checks. The channel, the automatic-check switch, and when the
//! last check ran are device settings; this module holds them in memory for snapshots. It also
//! finds where the app sits, and on macOS moves it into Applications (ADR 0019).

mod location;
mod manifest;
mod relocate;
#[cfg(test)]
mod tests;

use crate::{
    catalog::{Answer, CatalogSource, Fetcher},
    logging::Logger,
    persistence,
    worker::State,
};
pub use location::AppPlacement;
use location::Location;
use manifest::{Offer, Refusal, TARGET};
pub use relocate::Relaunch;
use serde::{Deserialize, Serialize};
use skillbinder_proto::{
    AppError, AppResult, AppUpdateSnapshot, AppUpdateState, AppVersion, Channel, CheckFailure,
    ErrorCategory,
};
use std::sync::{Arc, Mutex, MutexGuard};
use tokio::sync::oneshot;

/// The public key release manifests are signed with, `skillbinder-update.pub` (docs/release.md),
/// key id 41973E438186A81C.
pub(crate) const RELEASE_KEY: &str = "RWQcqIaBQz6XQfXzvvT6bIWtRbew0/txY9K20zqsnVXm3aodwyH+fIws";
/// Whether `build.yml` stamped this build as a release; only a stamped build checks.
pub(crate) const STAMPED: bool = match option_env!("SKILLBINDER_RELEASE") {
    Some(stamp) => !stamp.is_empty(),
    None => false,
};
/// The largest manifest or signature read.
const LIMIT: u64 = 64 * 1024;

const CHANNEL: &str = "update_channel";
const AUTO_CHECK: &str = "update_auto_check";
const LAST_CHECK: &str = "update_last_check";
const LAST_RUN: &str = "last_run_version";

/// Where a stamped build reads its manifests and the key that signs them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateSource {
    /// The site laid out like the catalog's, with `updates/v1/<channel>.json` and `.minisig`.
    pub site: CatalogSource,
    /// The minisign public key, in base64.
    pub public_key: String,
}

/// The last check that read a channel's manifest: which channel, when, and the version it
/// offered, saved so a relaunch shows the offer until the next check.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct LastCheck {
    /// A check that ended as the channel switched can save after the switch, so a restore
    /// trusts only the saved channel's.
    channel: Channel,
    checked: u64,
    offer: Option<Offer>,
}

/// How a check ended.
pub(crate) enum Ended {
    /// It read the manifest.
    Checked(LastCheck),
    /// The channel changed while it ran, so its answer was dropped and the new channel waits.
    Superseded,
    /// It failed, or a failure kept the state it found.
    Failed,
}

/// The last verified answer for a channel, kept so a 304 changes nothing.
struct Known {
    channel: Channel,
    etag: String,
    offer: Option<Offer>,
}

struct Shared {
    channel: Channel,
    auto_check: bool,
    state: AppUpdateState,
    last_checked: Option<u64>,
    known: Option<Known>,
    /// Where the running app sits, found once at launch.
    location: Location,
    /// What to start at quit once Move to Applications has moved the app.
    relaunch: Option<Relaunch>,
}

/// App updates, shared by the worker's snapshots, the client, and the check thread.
#[derive(Clone)]
pub(crate) struct AppUpdates {
    shared: Arc<Mutex<Shared>>,
    /// `None` in a build that never checks.
    source: Option<UpdateSource>,
    current: AppVersion,
    placement: AppPlacement,
    log: Logger,
}

impl AppUpdates {
    /// Off without a source; otherwise idle on the build's channel with automatic checks on,
    /// until [`State::restore_app_updates`] reads the saved settings. Finds where the app sits.
    pub(crate) fn new(
        source: Option<UpdateSource>,
        current: AppVersion,
        placement: AppPlacement,
        log: Logger,
    ) -> Self {
        let shared = Shared {
            channel: Channel::of(&current),
            auto_check: true,
            state: match source {
                Some(_) => AppUpdateState::Idle { last_failure: None },
                None => AppUpdateState::Off,
            },
            last_checked: None,
            known: None,
            location: location::locate(&placement),
            relaunch: None,
        };
        Self {
            shared: Arc::new(Mutex::new(shared)),
            source,
            current,
            placement,
            log,
        }
    }

    fn lock(&self) -> AppResult<MutexGuard<'_, Shared>> {
        self.shared.lock().map_err(|_| AppError::storage())
    }

    pub(crate) fn current(&self) -> AppResult<AppUpdateSnapshot> {
        let shared = self.lock()?;
        Ok(AppUpdateSnapshot {
            current: self.current,
            channel: shared.channel,
            auto_check: shared.auto_check,
            state: shared.state.clone(),
            last_checked: shared.last_checked,
            outside_applications: matches!(shared.location, Location::OutsideApplications(_)),
        })
    }

    /// Moves the app from outside Applications into it (spec rule 21); the move starts the new
    /// copy once the app quits, through [`Self::take_relaunch`]. A failure is logged.
    pub(crate) fn move_to_applications(&self) -> AppResult<()> {
        let Location::OutsideApplications(original) = self.lock()?.location.clone() else {
            return Err(AppError::new(
                ErrorCategory::Validation,
                "SkillBinder is already in Applications.",
                "Keep using it from there.",
            ));
        };
        let log = |line| self.log.record(line);
        let relaunch = relocate::move_to_applications(original, &self.placement, self.current, log)
            .map_err(|error| {
                self.log.record(format!(
                    "Couldn't move SkillBinder to Applications: {error}"
                ));
                AppError::storage()
            })?;
        self.lock()?.relaunch = Some(relaunch);
        Ok(())
    }

    /// What Move to Applications left to start at quit, once.
    pub(crate) fn take_relaunch(&self) -> Option<Relaunch> {
        self.lock().ok()?.relaunch.take()
    }

    /// Checks the channel's manifest on its own thread unless the build never checks or a check
    /// is running. The receiver answers with how the check ended; `None` when none started.
    /// Only a `manual` check keeps its failure; an automatic one that fails changes nothing.
    pub(crate) fn check(&self, manual: bool) -> AppResult<Option<oneshot::Receiver<Ended>>> {
        let Some(source) = self.source.clone() else {
            return Ok(None);
        };
        let (channel, etag, previous) = {
            let mut shared = self.lock()?;
            if shared.state == AppUpdateState::Checking {
                return Ok(None);
            }
            let previous = std::mem::replace(&mut shared.state, AppUpdateState::Checking);
            let etag = shared
                .known
                .as_ref()
                .filter(|known| known.channel == shared.channel)
                .map(|known| known.etag.clone());
            (shared.channel, etag, previous)
        };
        let running = Running {
            updates: Some(self.clone()),
            channel,
            manual,
            previous,
        };
        let (done, finished) = oneshot::channel();
        let updates = self.clone();
        std::thread::Builder::new()
            .name("skillbinder-app-update".into())
            .spawn(move || {
                let outcome = updates.fetch(&source, channel, etag.as_deref());
                let _ = done.send(running.finish(outcome));
            })
            .map_err(|_| AppError::storage())?;
        Ok(Some(finished))
    }

    /// Reads and verifies the channel's manifest, sending `etag` from its last verified answer.
    fn fetch(&self, source: &UpdateSource, channel: Channel, etag: Option<&str>) -> Outcome {
        let fetcher = Fetcher::new(&source.site);
        let path = format!("updates/v1/{}.json", channel.name());
        let (manifest, etag) = match fetcher.get(&path, etag, LIMIT) {
            Answer::Body { bytes, etag } => (bytes, etag),
            Answer::NotModified => return Outcome::NotModified,
            Answer::Offline => return Outcome::Failed(CheckFailure::Offline),
            Answer::NotFound | Answer::Failed => return Outcome::Failed(CheckFailure::Failed),
        };
        let signature = match fetcher.get(&format!("{path}.minisig"), None, LIMIT) {
            Answer::Body { bytes, .. } => bytes,
            Answer::Offline => return Outcome::Failed(CheckFailure::Offline),
            Answer::NotModified | Answer::NotFound | Answer::Failed => {
                return Outcome::Failed(CheckFailure::Failed);
            }
        };
        match manifest::offer(
            &manifest,
            &signature,
            &source.public_key,
            self.current,
            TARGET,
        ) {
            Ok(offer) => Outcome::Verified { offer, etag },
            Err(refusal) => {
                let why = match refusal {
                    Refusal::Signature => "its signature does not verify",
                    Refusal::Unreadable => "it does not read as an update manifest",
                };
                self.log
                    .record(format!("App update check refused {path}: {why}."));
                Outcome::Failed(CheckFailure::Failed)
            }
        }
    }

    /// Follows `channel` from now on; what was found for the other channel, and when, no longer
    /// applies.
    pub(crate) fn set_channel(&self, channel: Channel) -> AppResult<()> {
        let mut shared = self.lock()?;
        if shared.channel == channel {
            return Ok(());
        }
        shared.channel = channel;
        shared.last_checked = None;
        if shared.state != AppUpdateState::Checking && shared.state != AppUpdateState::Off {
            shared.state = AppUpdateState::Idle { last_failure: None };
        }
        Ok(())
    }

    pub(crate) fn set_auto_check(&self, on: bool) -> AppResult<()> {
        self.lock()?.auto_check = on;
        Ok(())
    }
}

impl State {
    /// Takes the saved channel, switch, and last check over, and records the running version.
    pub(crate) fn restore_app_updates(&self) -> AppResult<()> {
        let channel: Option<Channel> = self.setting(CHANNEL)?;
        let auto_check: Option<bool> = self.setting(AUTO_CHECK)?;
        let last_check: Option<LastCheck> = self.setting(LAST_CHECK)?;
        self.set_setting(LAST_RUN, &self.config.app_version)?;
        let mut shared = self.app_updates.lock()?;
        if let Some(channel) = channel {
            shared.channel = channel;
        }
        if let Some(auto_check) = auto_check {
            shared.auto_check = auto_check;
        }
        let last_check = last_check.filter(|check| check.channel == shared.channel);
        if let Some(LastCheck { checked, offer, .. }) = last_check {
            shared.last_checked = Some(checked);
            // The offer shows until the next check, unless this build already passed it.
            if let Some(Offer { version, notes_url }) = offer
                && shared.state != AppUpdateState::Off
                && version > self.config.app_version
            {
                shared.state = AppUpdateState::DownloadOnly { version, notes_url };
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
    pub(crate) fn save_update_check(&self, check: &LastCheck) -> AppResult<()> {
        self.set_setting(LAST_CHECK, check)
    }
}

/// What a check ended with.
enum Outcome {
    /// The manifest verified, offering a newer version or none.
    Verified {
        offer: Option<Offer>,
        etag: Option<String>,
    },
    /// The manifest is the one last verified for this channel.
    NotModified,
    Failed(CheckFailure),
}

/// A running check. Finishing it publishes its outcome; dropping it unfinished, as a panicking
/// thread does, ends it as failed so a later check can start.
struct Running {
    updates: Option<AppUpdates>,
    channel: Channel,
    manual: bool,
    /// The state before the check, which a silent failure keeps.
    previous: AppUpdateState,
}

impl Running {
    /// Publishes `outcome` and returns how the check ended.
    fn finish(mut self, outcome: Outcome) -> Ended {
        match self.updates.take() {
            Some(updates) => updates.end(&self, outcome),
            None => Ended::Failed,
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        if let Some(updates) = self.updates.take() {
            updates.end(self, Outcome::Failed(CheckFailure::Failed));
        }
    }
}

impl AppUpdates {
    fn end(&self, running: &Running, outcome: Outcome) -> Ended {
        let Ok(mut shared) = self.shared.lock() else {
            return Ended::Failed;
        };
        let offer = match outcome {
            Outcome::Verified { offer, etag } => {
                shared.known = etag.map(|etag| Known {
                    channel: running.channel,
                    etag,
                    offer: offer.clone(),
                });
                Ok(offer)
            }
            Outcome::NotModified => match &shared.known {
                Some(known) if known.channel == running.channel => Ok(known.offer.clone()),
                _ => Err(CheckFailure::Failed),
            },
            Outcome::Failed(failure) => Err(failure),
        };
        // A channel switched while the check ran leaves the answer for the old one unused.
        if shared.channel != running.channel {
            shared.state = AppUpdateState::Idle { last_failure: None };
            return Ended::Superseded;
        }
        match offer {
            Ok(offer) => {
                let checked = persistence::now();
                shared.last_checked = Some(checked);
                shared.state = match offer.clone() {
                    Some(Offer { version, notes_url }) => {
                        AppUpdateState::DownloadOnly { version, notes_url }
                    }
                    None => AppUpdateState::Idle { last_failure: None },
                };
                Ended::Checked(LastCheck {
                    channel: running.channel,
                    checked,
                    offer,
                })
            }
            Err(failure) => {
                shared.state = if running.manual {
                    AppUpdateState::Idle {
                        last_failure: Some(failure),
                    }
                } else {
                    running.previous.clone()
                };
                Ended::Failed
            }
        }
    }
}

/// The running build's version, as Cargo stamped it; a test proves the workspace's parses.
pub(crate) fn running_version() -> AppVersion {
    env!("CARGO_PKG_VERSION").parse().unwrap_or(AppVersion {
        major: 0,
        minor: 0,
        patch: 0,
        beta: None,
    })
}
