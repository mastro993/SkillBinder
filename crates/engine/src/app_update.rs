//! App updates (ADR 0017, ADR 0018): reads the channel's signed manifest from the catalog's site
//! on a thread of its own, never the worker. Where the app can replace itself, a newer version is
//! downloaded, verified, and staged beside it, and installs at quit; elsewhere it is offered as
//! download-only. A build without the release stamp never checks. The channel, the
//! automatic-check switch, the last check, and the staged version are device settings; this
//! module holds them in memory for snapshots. It also finds where the app sits, and on macOS
//! moves it into Applications (ADR 0019). On Windows the update is the setup program, run
//! silently at quit. On Linux it is the AppImage, and the app offers once to add itself to the
//! applications menu.

mod check;
mod desktop_entry;
mod download;
mod install;
mod location;
mod manifest;
mod ready;
mod relocate;
mod settings;
mod stage;
#[cfg(test)]
mod tests;

use crate::{catalog::CatalogSource, logging::Logger, storage};
pub(crate) use check::Ended;
use check::Known;
pub use location::AppPlacement;
use location::Location;
pub(crate) use location::bundle_of;
use manifest::Offer;
pub use relocate::Relaunch;
use skillbinder_proto::{
    AppError, AppResult, AppUpdateSnapshot, AppUpdateState, AppVersion, Channel, ErrorCategory,
};
pub use stage::UpdatePayload;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};

/// The public key release manifests are signed with, `skillbinder-update.pub` (docs/release.md),
/// key id 41973E438186A81C.
pub(crate) const RELEASE_KEY: &str = "RWQcqIaBQz6XQfXzvvT6bIWtRbew0/txY9K20zqsnVXm3aodwyH+fIws";
/// Whether `build.yml` stamped this build as a release; only a stamped build checks.
pub(crate) const STAMPED: bool = match option_env!("SKILLBINDER_RELEASE") {
    Some(stamp) => !stamp.is_empty(),
    None => false,
};
/// The Apple team whose Developer ID signs releases; `build.yml` sets it for stamped builds.
pub(crate) const TEAM: &str = match option_env!("SKILLBINDER_TEAM_ID") {
    Some(team) => team,
    None => "",
};
const _: () = assert!(
    !STAMPED || !cfg!(target_os = "macos") || !TEAM.is_empty(),
    "a stamped macOS build needs SKILLBINDER_TEAM_ID to verify its app updates"
);

/// Where a stamped build reads its manifests, the key that signs them, and what its archives
/// hold.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateSource {
    /// The site laid out like the catalog's, with `updates/v1/<channel>.json` and `.minisig`.
    pub site: CatalogSource,
    /// The minisign public key, in base64.
    pub public_key: String,
    /// What the archives hold.
    pub payload: UpdatePayload,
}

/// Why the app is quitting, which decides whether a Windows app update installs now.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuitCause {
    /// The user quit, or Restart or Move to Applications asked to.
    User,
    /// The user is logging off or Windows is shutting down, and could end a setup program
    /// halfway (spec rule 12).
    SessionEnd,
}

/// A verified update staged beside the installed app, or the Windows setup program staged in
/// application data.
#[derive(Clone, Debug)]
struct Staged {
    offer: Offer,
    bundle: PathBuf,
}

struct Shared {
    channel: Channel,
    auto_check: bool,
    state: AppUpdateState,
    last_checked: Option<u64>,
    known: Option<Known>,
    /// A check, or the download that follows it, is running.
    running: bool,
    /// The update that installs at quit.
    staged: Option<Staged>,
    /// Restart asked to open the app again once it quits.
    restart: bool,
    /// The release page of this version, at its first launch after an older one.
    updated: Option<String>,
    /// Where the running app sits, found once at launch.
    location: Location,
    /// What to start at quit once Move to Applications has moved the app.
    relaunch: Option<Relaunch>,
    /// Linux: the app runs as an AppImage and hasn't asked yet to add itself to the applications
    /// menu.
    offer_menu: bool,
}

impl Shared {
    /// The state with no check running: the staged update, or up to date.
    fn resting(&self) -> AppUpdateState {
        match &self.staged {
            Some(staged) => AppUpdateState::Ready {
                version: staged.offer.version,
                notes_url: staged.offer.notes_url.clone(),
            },
            None => AppUpdateState::Idle { last_failure: None },
        }
    }
}

/// App updates, shared by the worker's snapshots, the client, and the check thread.
#[derive(Clone)]
pub(crate) struct AppUpdates {
    shared: Arc<Mutex<Shared>>,
    /// `None` in a build that never checks.
    source: Option<UpdateSource>,
    current: AppVersion,
    placement: AppPlacement,
    /// Where archives download to, `<data>/updates`.
    downloads: PathBuf,
    log: Logger,
}

impl AppUpdates {
    /// Off without a source; otherwise idle on the build's channel with automatic checks on,
    /// until [`crate::worker::State::restore_app_updates`] reads the saved settings. Finds where
    /// the app sits.
    pub(crate) fn new(
        source: Option<UpdateSource>,
        current: AppVersion,
        placement: AppPlacement,
        downloads: PathBuf,
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
            running: false,
            staged: None,
            restart: false,
            updated: None,
            location: location::locate(&placement, &downloads),
            relaunch: None,
            offer_menu: false,
        };
        Self {
            shared: Arc::new(Mutex::new(shared)),
            source,
            current,
            placement,
            downloads,
            log,
        }
    }

    fn lock(&self) -> AppResult<MutexGuard<'_, Shared>> {
        self.shared
            .lock()
            .map_err(storage::error("locking the app update state"))
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
            offer_applications_menu: shared.offer_menu,
            updated: shared.updated.clone(),
        })
    }

    /// Writes the applications-menu entry and icon pointing at the running AppImage (spec rule
    /// 23). A failure is logged.
    pub(crate) fn add_to_applications_menu(&self) -> AppResult<()> {
        let Some(appimage) = &self.placement.appimage else {
            return Err(AppError::validation(
                "SkillBinder isn't running as an AppImage.",
            ));
        };
        desktop_entry::write(appimage, &self.placement.data_home).map_err(|error| {
            AppError::storage()
                .because(format_args!("writing the applications menu entry: {error}"))
        })
    }

    /// Moves the app from outside Applications into it (spec rule 21); the move starts the new
    /// copy once the app quits, through [`Self::finish`]. A failure is logged.
    pub(crate) fn move_to_applications(&self) -> AppResult<()> {
        let Location::OutsideApplications(original) = self.lock()?.location.clone() else {
            return Err(AppError::new(
                ErrorCategory::Validation,
                "SkillBinder is already in Applications.",
                "Keep using it from there.",
            ));
        };
        let relaunch =
            relocate::move_to_applications(original, &self.placement, self.current, &self.log)
                .map_err(|error| AppError::storage().because(error))?;
        self.lock()?.relaunch = Some(relaunch);
        Ok(())
    }

    /// Follows `channel` from now on; what was found for the other channel, and when, no longer
    /// applies. Switching to Stable discards a staged beta (spec rule 17).
    pub(crate) fn set_channel(&self, channel: Channel) -> AppResult<()> {
        let mut shared = self.lock()?;
        if shared.channel == channel {
            return Ok(());
        }
        shared.channel = channel;
        shared.last_checked = None;
        let beta = |staged: &Staged| staged.offer.version.beta.is_some();
        let discarded = channel == Channel::Stable && shared.staged.as_ref().is_some_and(beta);
        if discarded && let Some(staged) = shared.staged.take() {
            stage::remove(&staged.bundle);
        }
        // A running check settles the state when it ends.
        if shared.state != AppUpdateState::Off && (discarded || !shared.running) {
            shared.state = if shared.running {
                AppUpdateState::Checking
            } else {
                shared.resting()
            };
        }
        Ok(())
    }

    pub(crate) fn set_auto_check(&self, on: bool) -> AppResult<()> {
        self.lock()?.auto_check = on;
        Ok(())
    }

    /// Opens the app again once it quits and installs the staged update (spec rule 10).
    pub(crate) fn request_restart(&self) -> AppResult<()> {
        self.lock()?.restart = true;
        Ok(())
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
