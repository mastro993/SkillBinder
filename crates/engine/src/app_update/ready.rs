//! A ready app update (spec rules 5–8, 12, 17): downloaded into `<data>/updates/<version>/`,
//! staged beside the installed bundle or AppImage, or for the Windows setup program in
//! `<data>/updates`, restored at launch while it is newer than the running version, and installed
//! at quit: by renames, which the next launch cleans up after, or by the setup program run
//! silently.

use super::{
    AppUpdates, Ended, QuitCause, Relaunch, Staged,
    check::{LastCheck, Running, failed},
    download,
    install::{self, swap},
    location::Location,
    manifest::{Found, Offer},
    stage::{self, OLD},
};
use crate::logging::{Area, Level};
use skillbinder_proto::{AppResult, AppUpdateState, AppVersion, CheckFailure};
use std::{
    fs,
    path::{Path, PathBuf},
};

impl AppUpdates {
    /// Downloads and stages `found` in `folder` for the `running` check. A failure is logged and
    /// ends the check as failed, so it neither reads as up to date nor saves the check, and the
    /// next check tries again.
    pub(super) fn download(
        &self,
        running: &Running,
        check: LastCheck,
        found: Found,
        folder: &Path,
    ) -> Ended {
        let version = found.offer.version;
        let staged = self.fetch_and_stage(&found, folder);
        let Ok(mut shared) = self.shared.lock() else {
            return Ended::Failed;
        };
        if let Err(why) = &staged {
            self.log
                .event(Level::Info, Area::AppUpdate, "update discarded")
                .field("version", version)
                .field("cause", why);
        }
        // A channel switched during the download drops it, whatever came of it, and the new
        // channel is checked next.
        if shared.channel != running.channel {
            if let Ok(staged) = &staged {
                stage::remove(staged);
            }
            shared.state = shared.resting();
            return Ended::Superseded;
        }
        let Ok(staged) = staged else {
            shared.last_checked = running.previous_checked;
            return failed(&mut shared, running, CheckFailure::Failed);
        };
        let replaced = shared.staged.replace(Staged {
            offer: found.offer.clone(),
            bundle: staged.clone(),
        });
        if let Some(replaced) = replaced
            && replaced.bundle != staged
        {
            stage::remove(&replaced.bundle);
        }
        shared.state = shared.resting();
        Ended::Checked {
            check,
            ready: Some(found.offer),
        }
    }

    fn fetch_and_stage(&self, found: &Found, folder: &Path) -> Result<PathBuf, String> {
        let source = self.source.as_ref().ok_or("updates are off")?;
        let version = found.offer.version;
        let work = self.downloads.join(version.to_string());
        let archive = work.join("archive");
        let staged = fs::create_dir_all(&work)
            .map_err(|error| error.to_string())
            .and_then(|()| {
                download::download(&source.site, &found.asset, &archive, |percent| {
                    self.downloading(version, percent);
                })
            })
            .and_then(|()| {
                stage::stage(&source.payload, &archive, folder, version)
                    .map_err(|error| error.to_string())
            });
        let _ = fs::remove_dir_all(&work);
        staged
    }

    /// Shows how much of `version` has arrived while it is the one downloading.
    fn downloading(&self, version: AppVersion, percent: u8) {
        if let Ok(mut shared) = self.shared.lock()
            && let AppUpdateState::Downloading {
                version: shown,
                percent: shown_percent,
            } = &mut shared.state
            && *shown == version
        {
            *shown_percent = Some(percent);
        }
    }

    /// At launch, removes what installs and downloads left, and keeps the staged update `saved`
    /// as ready while its bundle is there and it is newer than the running version. Returns
    /// whether it was kept.
    pub(super) fn restore_staged(&self, saved: Option<Offer>) -> AppResult<bool> {
        stage::clear_downloads(&self.downloads);
        let mut shared = self.lock()?;
        let Ok(folder) = shared.location.staging() else {
            return Ok(false);
        };
        let mut kept = None;
        for (version, path) in stage::leftovers(folder) {
            match &saved {
                Some(offer)
                    if kept.is_none()
                        && self.source.is_some()
                        && offer.version == version
                        && version > self.current =>
                {
                    kept = Some(Staged {
                        offer: offer.clone(),
                        bundle: path,
                    });
                }
                _ => stage::remove(&path),
            }
        }
        let restored = kept.is_some();
        shared.staged = kept;
        if restored {
            shared.state = shared.resting();
        }
        Ok(restored)
    }

    /// At quit, after the engine drained: installs the staged update, and says what to start,
    /// the moved copy after Move to Applications, or the app again after Restart. A failed
    /// install keeps the running version and is logged; the update is offered again at the next
    /// launch. On Windows the setup program installs it once the app has exited, and opens it
    /// again after Restart; a quit as the session ends starts nothing, so Windows can't end the
    /// setup program halfway, and the next quit installs it.
    pub(crate) fn finish(&self, cause: QuitCause) -> Option<Relaunch> {
        let mut shared = self.shared.lock().ok()?;
        if let Some(relaunch) = shared.relaunch.take() {
            return Some(relaunch);
        }
        let restart = shared.restart;
        let staged = shared.staged.take();
        let relaunch = match (shared.location.clone(), staged) {
            (Location::Installed(bundle), staged) => {
                if let Some(staged) = staged {
                    self.swap_in(&staged, &bundle);
                }
                restart.then(|| Relaunch::reopen(bundle))
            }
            (Location::SetUp { executable, .. }, Some(staged)) => self
                .start_setup(&staged, cause, restart)
                .then(|| Relaunch::reopen(executable)),
            (
                Location::SetUp { .. }
                | Location::NotInstalled
                | Location::OutsideApplications(_)
                | Location::NotWritable,
                _,
            ) => None,
        };
        self.log.flush();
        relaunch
    }

    /// Puts the staged bundle or AppImage in place of the installed `bundle`, logging how it went.
    fn swap_in(&self, staged: &Staged, bundle: &Path) {
        // Set aside under its own extension: `.app` for a bundle, `.AppImage` for an AppImage.
        let extension = bundle
            .extension()
            .map(|extension| format!(".{}", extension.to_string_lossy()))
            .unwrap_or_default();
        let old = bundle.with_file_name(format!("{OLD}{}{extension}", self.current));
        let version = staged.offer.version;
        match swap(&staged.bundle, bundle, &old) {
            Ok(()) => {
                self.log
                    .event(Level::Info, Area::AppUpdate, "update installed")
                    .field("version", version);
            }
            Err(error) => {
                self.log
                    .event(Level::Error, Area::AppUpdate, "update install failed")
                    .field("version", version)
                    .field("cause", error);
            }
        }
    }

    /// Starts the staged setup program unless the session is ending, logging how it went.
    /// Returns whether the app must open itself again: after Restart, when setup didn't start.
    fn start_setup(&self, staged: &Staged, cause: QuitCause, restart: bool) -> bool {
        let version = staged.offer.version;
        if cause == QuitCause::SessionEnd {
            self.log
                .event(Level::Info, Area::AppUpdate, "update deferred to next quit")
                .field("version", version);
            return false;
        }
        match install::start_setup(&staged.bundle, std::process::id(), restart) {
            Ok(()) => {
                self.log
                    .event(Level::Info, Area::AppUpdate, "setup started")
                    .field("version", version);
                false
            }
            Err(error) => {
                self.log
                    .event(Level::Error, Area::AppUpdate, "setup start failed")
                    .field("version", version)
                    .field("cause", error);
                restart
            }
        }
    }
}
