//! The Sync page's words: each state's glyph, title, line, and main action (Y2), what a running
//! action's button reads, and the toast an action ends with (Y7).

use super::super::library_dates::{from_unix, relative};
use chrono::{DateTime, Utc};
use skillbinder_proto::{SyncAction, SyncSnapshot, SyncState};

/// The tint of the status card's glyph tile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Tint {
    Success,
    Accent,
    Danger,
}

/// What the status card shows for the state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Status {
    pub(super) glyph: &'static str,
    pub(super) tint: Tint,
    pub(super) title: String,
    pub(super) line: String,
    /// The one next action, if the state has one.
    pub(super) main: Option<SyncAction>,
}

/// The status card for `sync` at `now`; a library with no remote shows the setup column instead.
pub(super) fn status(sync: &SyncSnapshot, now: DateTime<Utc>) -> Option<Status> {
    let checked = sync
        .refreshed_at
        .and_then(from_unix)
        .map(|at| relative(at, now));
    let card = |glyph, tint, title: String, line: String, main| Status {
        glyph,
        tint,
        title,
        line,
        main,
    };
    let here = sync.local_skills;
    let there = sync.incoming_skills;
    Some(match sync.state {
        SyncState::NotConfigured => return None,
        SyncState::Synced => card(
            "check-circle",
            Tint::Success,
            "Up to date".to_owned(),
            checked.map_or_else(
                || "Not checked yet".to_owned(),
                |at| format!("Checked {at}"),
            ),
            None,
        ),
        SyncState::NeedsPush => card(
            "upload",
            Tint::Accent,
            format!("{} changed on this device", skills(here)),
            "Not pushed yet".to_owned(),
            Some(SyncAction::Push),
        ),
        SyncState::NeedsPull => card(
            "download",
            Tint::Accent,
            format!("{} changed on the remote", skills(there)),
            "Not pulled yet".to_owned(),
            Some(SyncAction::Pull),
        ),
        SyncState::NeedsSync => card(
            "git-merge",
            Tint::Accent,
            "Changes on both sides".to_owned(),
            format!("{here} here, {there} from the remote"),
            Some(SyncAction::Sync),
        ),
        SyncState::Unreachable => {
            let error = sync
                .last_error
                .as_ref()
                .map_or("The remote did not answer.", |error| error.message.as_str());
            card(
                "alert-circle",
                Tint::Danger,
                "Can't reach the remote".to_owned(),
                match checked {
                    Some(at) => format!("Last checked {at} · {error}"),
                    None => error.to_owned(),
                },
                Some(SyncAction::Refresh),
            )
        }
    })
}

/// The label of the main action's button.
pub(super) fn main_label(action: SyncAction) -> &'static str {
    match action {
        SyncAction::Push => "Push",
        SyncAction::Pull => "Pull",
        SyncAction::Sync => "Sync",
        SyncAction::Refresh => "Try again",
        SyncAction::Disconnect => "Disconnect",
    }
}

/// What the button of `action` reads while it runs.
pub(super) fn running(action: SyncAction) -> &'static str {
    match action {
        SyncAction::Push => "Pushing…",
        SyncAction::Pull => "Pulling…",
        SyncAction::Sync => "Syncing…",
        SyncAction::Refresh => "Checking…",
        SyncAction::Disconnect => "Disconnecting…",
    }
}

/// The toast `action` ends with, naming the remote by `address`.
pub(super) fn done(action: SyncAction, address: &str) -> Option<String> {
    match action {
        SyncAction::Push => Some(format!("Pushed to {address}")),
        SyncAction::Pull => Some(format!("Pulled from {address}")),
        SyncAction::Sync => Some(format!("Synced with {address}")),
        SyncAction::Refresh | SyncAction::Disconnect => None,
    }
}

fn skills(count: u64) -> String {
    match count {
        1 => "1 skill".to_owned(),
        count => format!("{count} skills"),
    }
}

#[cfg(test)]
mod tests;
