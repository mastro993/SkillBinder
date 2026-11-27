//! The Sync page's words: each state's glyph, title, line, and main action (Y2), what a running
//! action's button reads, the toast an action ends with (Y7), and each waiting skill's kind and
//! why (Y5, Y8).

use super::super::library_dates::{from_unix, relative};
use chrono::{DateTime, Utc};
use skillbinder_proto::{
    ChangeKind, ChangedSkill, Incoming, Reason, SyncAction, SyncSnapshot, SyncState,
};

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
    let here = sync.local.len();
    let incoming = sync.incoming.as_ref();
    let there = incoming.map_or(0, |incoming| incoming.skills.len());
    let from = incoming
        .and_then(devices)
        .unwrap_or_else(|| "the remote".to_owned());
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
            format!("{} changed on {from}", skills(there)),
            "Not pulled yet".to_owned(),
            Some(SyncAction::Pull),
        ),
        SyncState::NeedsSync => card(
            "git-merge",
            Tint::Accent,
            "Changes on both sides".to_owned(),
            format!("{here} here, {there} from {from}"),
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

/// The devices that changed what the remote holds: the newest, then "and <k> more" with several;
/// none for commits made before device names.
pub(super) fn devices(incoming: &Incoming) -> Option<String> {
    let (newest, rest) = incoming.devices.split_first()?;
    Some(match rest.len() {
        0 => newest.clone(),
        more => format!("{newest} and {more} more"),
    })
}

/// The end of the remote's group head (Y5): when its newest commit was made, if known, then
/// "Not pulled".
pub(super) fn not_pulled(incoming: &Incoming, now: DateTime<Utc>) -> String {
    let Some(at) = incoming.at.and_then(from_unix) else {
        return "Not pulled".to_owned();
    };
    let at = relative(at, now);
    let mut letters = at.chars();
    let first = letters.next().map(|c| c.to_uppercase().to_string());
    format!(
        "{}{} · Not pulled",
        first.unwrap_or_default(),
        letters.as_str()
    )
}

/// A kind's mark, its tooltip, and its tint (Y5).
pub(super) fn kind(kind: ChangeKind) -> (&'static str, &'static str, Tint) {
    match kind {
        ChangeKind::Added => ("+", "Added", Tint::Success),
        ChangeKind::Changed => ("~", "Changed", Tint::Accent),
        ChangeKind::Removed => ("−", "Removed", Tint::Danger),
    }
}

/// Why a skill changed (Y8); unknown for a commit made before device names.
pub(super) fn why(skill: &ChangedSkill) -> Option<String> {
    Some(match skill.reason.as_ref()? {
        Reason::Imported { from: Some(from) } => format!("Imported from {from}"),
        Reason::Imported { from: None } => "Imported".to_owned(),
        Reason::Installed => "Installed from Explore".to_owned(),
        Reason::Updated => "Updated from its source".to_owned(),
        Reason::Reinstalled => "Reinstalled".to_owned(),
        Reason::Record => "Record added".to_owned(),
        Reason::Outside if skill.kind == ChangeKind::Removed => {
            "Removed outside SkillBinder".to_owned()
        }
        Reason::Outside => "Changed outside SkillBinder".to_owned(),
    })
}

fn skills(count: usize) -> String {
    match count {
        1 => "1 skill".to_owned(),
        count => format!("{count} skills"),
    }
}

#[cfg(test)]
mod tests;
