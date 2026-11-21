//! A check (spec rules 2–5, 8, 13): reads and verifies the channel's manifest on a thread of its
//! own. A newer version downloads on the same thread where the app can replace itself, and is
//! offered as download-only where it can't. Only one check or download runs at a time.

use super::{
    AppUpdates, Shared, UpdateSource,
    manifest::{self, Found, Offer, Refusal, TARGET},
};
use crate::{
    catalog::{Answer, Fetcher},
    persistence,
};
use serde::{Deserialize, Serialize};
use skillbinder_proto::{AppError, AppResult, AppUpdateState, Channel, CheckFailure};
use std::path::PathBuf;
use tokio::sync::oneshot;

/// The largest manifest or signature read.
const LIMIT: u64 = 64 * 1024;

/// The last check that read a channel's manifest: which channel, when, and the version it
/// offered, saved so a relaunch shows the offer until the next check.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct LastCheck {
    /// A check that ended as the channel switched can save after the switch, so a restore
    /// trusts only the saved channel's.
    pub(super) channel: Channel,
    pub(super) checked: u64,
    pub(super) offer: Option<Offer>,
}

/// How a check ended.
pub(crate) enum Ended {
    /// It read the manifest, and staged the version in `ready` if it downloaded one.
    Checked {
        check: LastCheck,
        ready: Option<Offer>,
    },
    /// The channel changed while it ran, so its answer was dropped and the new channel waits.
    Superseded,
    /// It failed, or a failure kept the state it found.
    Failed,
}

/// The last verified answer for a channel, kept so a 304 changes nothing.
pub(super) struct Known {
    channel: Channel,
    etag: String,
    found: Option<Found>,
}

/// What a check read.
enum Outcome {
    /// The manifest verified, offering a newer version or none.
    Verified {
        found: Option<Found>,
        etag: Option<String>,
    },
    /// The manifest is the one last verified for this channel.
    NotModified,
    Failed(CheckFailure),
}

/// What follows a check that read the manifest.
enum Next {
    Ended(Ended),
    /// Download `found` and stage it in `folder`.
    Download {
        check: LastCheck,
        found: Found,
        folder: PathBuf,
    },
}

impl AppUpdates {
    /// Checks the channel's manifest on its own thread unless the build never checks or a check
    /// is running. The receiver answers with how the check ended; `None` when none started.
    /// Only a `manual` check keeps its failure; an automatic one that fails changes nothing.
    pub(crate) fn check(&self, manual: bool) -> AppResult<Option<oneshot::Receiver<Ended>>> {
        let Some(source) = self.source.clone() else {
            return Ok(None);
        };
        let (channel, etag, previous, previous_checked) = {
            let mut shared = self.lock()?;
            if shared.running {
                return Ok(None);
            }
            shared.running = true;
            let previous = shared.state.clone();
            // A staged update stays ready while a newer one is looked for.
            if shared.staged.is_none() {
                shared.state = AppUpdateState::Checking;
            }
            let etag = shared
                .known
                .as_ref()
                .filter(|known| known.channel == shared.channel)
                .map(|known| known.etag.clone());
            (shared.channel, etag, previous, shared.last_checked)
        };
        let running = Running {
            updates: Some(self.clone()),
            channel,
            manual,
            previous,
            previous_checked,
        };
        let (done, finished) = oneshot::channel();
        let updates = self.clone();
        std::thread::Builder::new()
            .name("skillbinder-app-update".into())
            .spawn(move || {
                let outcome = updates.fetch(&source, channel, etag.as_deref());
                let _ = done.send(running.finish(outcome));
            })
            .map_err(|_| {
                if let Ok(mut shared) = self.shared.lock() {
                    shared.running = false;
                }
                AppError::storage()
            })?;
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
            Ok(found) => Outcome::Verified { found, etag },
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

    /// Publishes `outcome`, then downloads and stages the version it found when the app can
    /// replace itself.
    fn end(&self, running: &Running, outcome: Outcome) -> Ended {
        let Ok(mut shared) = self.shared.lock() else {
            return Ended::Failed;
        };
        match settle(&mut shared, running, outcome) {
            Next::Ended(ended) => ended,
            Next::Download {
                check,
                found,
                folder,
            } => {
                drop(shared);
                self.download(running, check, found, &folder)
            }
        }
    }

    /// Releases the running check, so another can start.
    fn release(&self) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.running = false;
        }
    }
}

/// Sets the state `outcome` leads to, and says what follows.
fn settle(shared: &mut Shared, running: &Running, outcome: Outcome) -> Next {
    let found = match outcome {
        Outcome::Verified { found, etag } => {
            shared.known = etag.map(|etag| Known {
                channel: running.channel,
                etag,
                found: found.clone(),
            });
            Ok(found)
        }
        Outcome::NotModified => match &shared.known {
            Some(known) if known.channel == running.channel => Ok(known.found.clone()),
            _ => Err(CheckFailure::Failed),
        },
        Outcome::Failed(failure) => Err(failure),
    };
    // A channel switched while the check ran leaves the answer for the old one unused.
    if shared.channel != running.channel {
        shared.state = shared.resting();
        return Next::Ended(Ended::Superseded);
    }
    let found = match found {
        Ok(found) => found,
        Err(failure) => return Next::Ended(failed(shared, running, failure)),
    };
    let checked = persistence::now();
    shared.last_checked = Some(checked);
    let check = LastCheck {
        channel: running.channel,
        checked,
        offer: found.as_ref().map(|found| found.offer.clone()),
    };
    let ended = |check| Next::Ended(Ended::Checked { check, ready: None });
    let Some(found) = found else {
        shared.state = shared.resting();
        return ended(check);
    };
    let folder = match shared.location.staging() {
        Ok(folder) => folder.to_path_buf(),
        Err(reason) => {
            let Offer { version, notes_url } = found.offer;
            shared.state = AppUpdateState::DownloadOnly {
                version,
                notes_url,
                reason,
            };
            return ended(check);
        }
    };
    // A rule 8 replacement downloads only a version newer than the one staged.
    match &shared.staged {
        Some(staged) if staged.offer.version >= found.offer.version => {
            shared.state = shared.resting();
            ended(check)
        }
        staged => {
            if staged.is_none() {
                shared.state = AppUpdateState::Downloading {
                    version: found.offer.version,
                    percent: None,
                };
            }
            Next::Download {
                check,
                found,
                folder,
            }
        }
    }
}

/// Ends `running` as failed: a staged update stays ready, a Check now shows `failure`, and an
/// automatic check keeps the state it found.
pub(super) fn failed(shared: &mut Shared, running: &Running, failure: CheckFailure) -> Ended {
    shared.state = match (&shared.staged, running.manual) {
        (Some(_), _) => shared.resting(),
        (None, true) => AppUpdateState::Idle {
            last_failure: Some(failure),
        },
        (None, false) => running.previous.clone(),
    };
    Ended::Failed
}

/// A running check. Finishing it publishes its outcome; dropping it unfinished, as a panicking
/// thread does, ends it as failed so a later check can start.
pub(super) struct Running {
    updates: Option<AppUpdates>,
    pub(super) channel: Channel,
    manual: bool,
    /// The state before the check, which a silent failure keeps.
    previous: AppUpdateState,
    /// When the last check read the manifest before this one, kept by a failed download.
    pub(super) previous_checked: Option<u64>,
}

impl Running {
    /// Publishes `outcome`, downloading what it found if it can, and returns how it ended.
    fn finish(mut self, outcome: Outcome) -> Ended {
        let ended = match &self.updates {
            Some(updates) => updates.end(&self, outcome),
            None => Ended::Failed,
        };
        if let Some(updates) = self.updates.take() {
            updates.release();
        }
        ended
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        if let Some(updates) = self.updates.take() {
            if let Ok(mut shared) = updates.shared.lock() {
                let _ = settle(&mut shared, self, Outcome::Failed(CheckFailure::Failed));
            }
            updates.release();
        }
    }
}
