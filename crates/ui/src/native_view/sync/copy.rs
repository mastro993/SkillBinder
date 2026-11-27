//! The Sync page's words: each state's glyph, title, line, and main action (Y2), what a running
//! action's button reads, the toast an action ends with (Y7), each waiting skill's kind and why
//! (Y5, Y8), what Discard and Restore ask before they write (Y7), and what Resolve… and a merge
//! in History say of the skills changed on both sides (Y5, Y6, Y9).

use super::super::library_dates::{from_unix, relative};
use chrono::{DateTime, Datelike, Utc};
use skillbinder_proto::{
    ChangeKind, ChangedSkill, ConflictSide, Incoming, Reason, Side, SyncAction, SyncSnapshot,
    SyncState,
};
use std::collections::BTreeSet;

/// The tint of the status card's glyph tile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Tint {
    Success,
    Accent,
    Warning,
    Danger,
}

/// The status card's one next action: a Git operation, or Resolve…, which asks for a choice for
/// each skill changed on both sides.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Next {
    Run(SyncAction),
    Resolve,
}

/// What the status card shows for the state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Status {
    pub(super) glyph: &'static str,
    pub(super) tint: Tint,
    pub(super) title: String,
    pub(super) line: String,
    /// The one next action, if the state has one.
    pub(super) main: Option<Next>,
}

/// The status card for `sync` at `now`, once every skill changed on both sides has a choice if
/// `chosen`; a library with no remote shows the setup column instead.
pub(super) fn status(sync: &SyncSnapshot, chosen: bool, now: DateTime<Utc>) -> Option<Status> {
    let checked = sync
        .refreshed_at
        .and_then(from_unix)
        .map(|at| relative(at, now));
    let card = |glyph, tint, title: String, line: String, main: Option<SyncAction>| Status {
        glyph,
        tint,
        title,
        line,
        main: main.map(Next::Run),
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
            format!("{here} here, {there} from {from} · none on the same skill"),
            Some(SyncAction::Sync),
        ),
        SyncState::NeedsResolve => Status {
            glyph: "git-merge-conflict",
            tint: Tint::Warning,
            title: format!("{} changed on both sides", skills(sync.conflicts.len())),
            line: if chosen {
                "Choices made · Sync to finish"
            } else {
                "Choose which version to keep"
            }
            .to_owned(),
            main: Some(if chosen {
                Next::Run(SyncAction::Sync)
            } else {
                Next::Resolve
            }),
        },
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
pub(super) fn main_label(next: Next) -> &'static str {
    match next {
        Next::Run(SyncAction::Push) => "Push",
        Next::Run(SyncAction::Pull) => "Pull",
        Next::Run(SyncAction::Sync) => "Sync",
        Next::Run(SyncAction::Refresh) => "Try again",
        Next::Run(SyncAction::Disconnect) => "Disconnect",
        Next::Resolve => "Resolve…",
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
    why_changed(skill.kind, skill.reason.as_ref())
}

/// Why a skill changed `kind`, from `reason`.
fn why_changed(kind: ChangeKind, reason: Option<&Reason>) -> Option<String> {
    Some(match reason? {
        Reason::Imported { from: Some(from) } => format!("Imported from {from}"),
        Reason::Imported { from: None } => "Imported".to_owned(),
        Reason::Installed => "Installed from Explore".to_owned(),
        Reason::Updated => "Updated from its source".to_owned(),
        Reason::Reinstalled => "Reinstalled".to_owned(),
        Reason::Restored { at } => format!(
            "Restored to the version of {}",
            version_day(*at, &chrono::Local::now())
        ),
        Reason::Record => "Record added".to_owned(),
        Reason::Outside if kind == ChangeKind::Removed => "Removed outside SkillBinder".to_owned(),
        Reason::Outside => "Changed outside SkillBinder".to_owned(),
    })
}

/// What a row's Discard does to a change of `kind`, as its tooltip says.
pub(super) fn discard_tip(kind: ChangeKind) -> &'static str {
    match kind {
        ChangeKind::Added => "Remove it from the library; your agents keep their copies",
        ChangeKind::Changed => "Go back to the last synced version",
        ChangeKind::Removed => "Bring it back",
    }
}

/// What Discard asks before it writes (Y7): its title, and what discarding `skill`'s change does.
pub(super) fn discard_question(skill: &ChangedSkill) -> (String, String) {
    let slug = &skill.slug;
    let body = match skill.kind {
        ChangeKind::Added => format!(
            "{slug} was added on this device and never pushed. Discarding removes it from the \
             library. Copies your agents have stay where they are."
        ),
        ChangeKind::Changed => {
            format!("{slug} goes back to its last synced version. Your agents' copies follow.")
        }
        ChangeKind::Removed => format!(
            "{slug} comes back to the library as it was at the last sync. It is in no preset \
             until you turn it on."
        ),
    };
    (format!("Discard your change to {slug}?"), body)
}

/// What Restore asks before it writes (Y7): its title, its body, and its button, Bring back for a
/// skill the change on `day` at `time` removed.
pub(super) fn restore_question(
    slug: &str,
    removed: bool,
    day: &str,
    time: &str,
) -> (String, String, &'static str) {
    let after = "It becomes a change on this device, so your other computers get it at the next \
                 Sync. Your agents' copies follow.";
    if removed {
        (
            format!("Bring back {slug}?"),
            format!("{slug} comes back as it was before it was removed. {after}"),
            "Bring back",
        )
    } else {
        (
            format!("Restore {slug} to {day}, {time}?"),
            format!("Its content and record go back to this version. {after}"),
            "Restore",
        )
    }
}

/// The day a restored version was made, in `now`'s time zone: "Nov 18", or "Nov 18, 2024" in
/// another year.
pub(super) fn version_day<Tz: chrono::TimeZone>(at: u64, now: &DateTime<Tz>) -> String
where
    Tz::Offset: std::fmt::Display,
{
    let day = from_unix(at)
        .unwrap_or_default()
        .with_timezone(&now.timezone());
    if day.year() == now.year() {
        day.format("%b %-d").to_string()
    } else {
        day.format("%b %-d, %Y").to_string()
    }
}

/// What the Resolve dialog opens with (Y9), naming the remote's `device`.
pub(super) fn resolve_intro(device: &str, count: usize) -> String {
    let which = if count == 1 {
        "this skill"
    } else {
        "these skills"
    };
    format!(
        "This device and {device} both changed {which}. Choose the version to keep. The other is \
         still in History."
    )
}

/// One side's card in the Resolve dialog (Y9): whose version it is, and its why and time, or
/// "Not pushed" for this device's.
pub(super) fn side_card(
    side: Side,
    version: &ConflictSide,
    device: &str,
    now: DateTime<Utc>,
) -> (String, String) {
    let name = match side {
        Side::Local => format!("This device ({device})"),
        Side::Remote => version
            .device
            .clone()
            .unwrap_or_else(|| "The remote".to_owned()),
    };
    let when = match side {
        Side::Local => Some("Not pushed".to_owned()),
        Side::Remote => version.at.and_then(from_unix).map(|at| relative(at, now)),
    };
    let detail = [why_changed(version.kind, version.reason.as_ref()), when]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
    (name, detail)
}

/// How many skills changed on only one side, which merge without a choice: a skill both sides
/// changed, to different versions or the same, is not one of them.
pub(super) fn one_sided(sync: &SyncSnapshot) -> usize {
    let here: BTreeSet<&str> = sync.local.iter().map(|skill| skill.slug.as_str()).collect();
    let there: BTreeSet<&str> = (sync.incoming.iter())
        .flat_map(|incoming| &incoming.skills)
        .map(|skill| skill.slug.as_str())
        .collect();
    here.symmetric_difference(&there).count()
}

/// Under the Resolve dialog's choices (Y9): how many skills changed on one side only.
pub(super) fn merge_on_their_own(count: usize) -> String {
    match count {
        1 => "1 other skill changed on only one side and merges on its own.".to_owned(),
        count => format!("{count} other skills changed on only one side and merge on their own."),
    }
}

/// What an incoming row that changed on both sides ends with once chosen (Y5).
pub(super) fn chosen(side: Side) -> &'static str {
    match side {
        Side::Local => "Keeping this device's",
        Side::Remote => "Taking this one",
    }
}

/// A merge's choice in History (Y6): "Kept <device>'s <slug>".
pub(super) fn kept(device: Option<&str>, slug: &str) -> String {
    match device {
        Some(device) => format!("Kept {device}'s {slug}"),
        None => format!("Kept the remote's {slug}"),
    }
}

/// "1 skill" or "<n> skills".
pub(super) fn skills(count: usize) -> String {
    match count {
        1 => "1 skill".to_owned(),
        count => format!("{count} skills"),
    }
}

#[cfg(test)]
mod tests;
