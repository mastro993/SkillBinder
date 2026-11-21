//! A ready app update (spec rules 5–8, 12, 17): downloaded into `<data>/updates/<version>/`,
//! staged beside the installed app, restored at launch while it is newer than the running version,
//! and installed at quit by renames, which the next launch cleans up after.

use super::{
    AppUpdates, Ended, Relaunch, Staged,
    check::LastCheck,
    download,
    install::swap,
    location::Location,
    manifest::{Found, Offer},
    stage::{self, OLD},
};
use skillbinder_proto::{AppResult, AppUpdateState, AppVersion, Channel};
use std::{fs, path::Path};

impl AppUpdates {
    /// Downloads and stages `found` beside the installed `bundle` for a check of `channel`. A
    /// failure is logged and discarded, and the next check tries again.
    pub(super) fn download(
        &self,
        channel: Channel,
        check: LastCheck,
        found: Found,
        bundle: &Path,
    ) -> Ended {
        let version = found.offer.version;
        let staged = self.fetch_and_stage(&found, bundle);
        let Ok(mut shared) = self.shared.lock() else {
            return Ended::Failed;
        };
        let staged = match staged {
            Ok(staged) => staged,
            Err(why) => {
                self.log
                    .record(format!("App update {version} was discarded: {why}."));
                shared.state = shared.resting();
                return Ended::Checked { check, ready: None };
            }
        };
        if shared.channel != channel {
            stage::remove(&staged);
            shared.state = shared.resting();
            return Ended::Superseded;
        }
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

    fn fetch_and_stage(&self, found: &Found, bundle: &Path) -> Result<std::path::PathBuf, String> {
        let source = self.source.as_ref().ok_or("updates are off")?;
        let folder = bundle.parent().ok_or("the app has no folder")?;
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
        let Location::Installed(bundle) = &shared.location else {
            return Ok(false);
        };
        let Some(folder) = bundle.parent() else {
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
    /// launch.
    pub(crate) fn finish(&self) -> Option<Relaunch> {
        let mut shared = self.shared.lock().ok()?;
        if let Some(relaunch) = shared.relaunch.take() {
            return Some(relaunch);
        }
        let Location::Installed(bundle) = shared.location.clone() else {
            return None;
        };
        if let Some(staged) = shared.staged.take() {
            let old = bundle.with_file_name(format!("{OLD}{}.app", self.current));
            let version = staged.offer.version;
            match swap(&staged.bundle, &bundle, &old) {
                Ok(()) => self.log.record(format!("Installed SkillBinder {version}.")),
                Err(error) => self
                    .log
                    .record(format!("Couldn't install SkillBinder {version}: {error}")),
            }
            self.log.flush();
        }
        shared.restart.then(|| Relaunch::reopen(bundle))
    }
}
