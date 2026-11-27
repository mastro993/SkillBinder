//! The Sync page's changes as the command asks for them, and what `sync status` and
//! `sync history` answer: the skills not synced yet on each side, the skills changed on both,
//! and History, each skill with its kind and why in the page's words.

use crate::{ChangeKind, Side, SyncState};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A change the Sync page makes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "sync", rename_all = "snake_case")]
pub enum SyncCommand {
    /// `sync`, `push`, or `pull`.
    Run {
        /// Which of Sync's Git operations.
        action: SyncKind,
    },
    /// `remote set`: connects the library to a Git remote, replacing the one set.
    SetRemote {
        /// The remote's Git URL.
        url: String,
        /// Its branch.
        branch: String,
    },
    /// `remote remove`: disconnects the remote; the library stays as it is.
    RemoveRemote,
    /// `sync restore`: puts a skill back to its version in a History change, as History's
    /// Restore this version and Bring back do.
    Restore {
        /// The skill, by slug.
        skill: String,
        /// The change's commit, as `sync history` lists it.
        to: String,
    },
    /// `sync discard`: undoes this device's change to a skill not pushed yet, as Not synced
    /// yet's Discard does.
    Discard {
        /// The skill, by slug.
        skill: String,
    },
    /// `sync resolve`: Syncs keeping the chosen side of each skill changed on both sides, as
    /// Resolve… does.
    Resolve {
        /// The side kept for each such skill, by slug.
        choices: BTreeMap<String, Side>,
    },
    /// `device rename`: renames this device for the commits Push and Sync make from now on.
    RenameDevice {
        /// The new name.
        name: String,
    },
}

/// One of Sync's Git operations on the library.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncKind {
    /// Pulls what can fast-forward, then pushes the library's changes.
    Sync,
    /// Commits the library's changes and pushes them.
    Push,
    /// Fast-forwards to the remote's changes.
    Pull,
}

/// Where the library stands with its remote, and what waits on each side.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SyncStatusOutput {
    /// Where the library stands with its sync remote.
    pub state: SyncState,
    /// The remote's Git URL, when one is set.
    pub remote: Option<String>,
    /// This device's name, which each commit Push or Sync makes carries.
    pub device: String,
    /// The skills this device changed that a Push sends.
    pub local: Vec<SyncSkillRow>,
    /// The skills the remote changed that a Pull brings.
    pub incoming: Vec<SyncSkillRow>,
    /// The devices whose commits changed what the remote holds, newest first.
    pub devices: Vec<String>,
    /// The skills both sides changed to different versions; Sync needs a choice for each.
    pub conflicts: Vec<ConflictRow>,
}

/// A skill that changed, how, and why.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SyncSkillRow {
    /// The skill's slug.
    pub slug: String,
    /// How it changed.
    pub kind: ChangeKind,
    /// Why it changed, as the Sync page says it; none for a commit made before device names.
    pub why: Option<String>,
}

/// A skill both sides changed to different versions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ConflictRow {
    /// The skill's slug.
    pub slug: String,
    /// This device's version, which `mine` keeps.
    pub mine: ConflictVersion,
    /// The remote's version, which `theirs` keeps.
    pub theirs: ConflictVersion,
}

/// One side's version of a skill changed on both.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ConflictVersion {
    /// How it changed.
    pub kind: ChangeKind,
    /// Why it changed, as the Sync page says it.
    pub why: Option<String>,
    /// The device whose commit made the remote's version; none for this device's.
    pub device: Option<String>,
}

/// A page of History, newest first.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HistoryOutput {
    /// The library's changes, newest first.
    pub changes: Vec<HistoryRow>,
    /// The commit `--before` takes for the next page, while older changes remain.
    pub next: Option<String>,
}

/// One change in History.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HistoryRow {
    /// The commit's id, which `sync restore --to` takes.
    pub commit: String,
    /// When it was made, as Unix seconds.
    pub at: u64,
    /// When it was made, as History says it, such as `Today, 09:15`.
    pub when: String,
    /// The device that made it; none for a commit made before device names.
    pub device: Option<String>,
    /// The skills it changed.
    pub skills: Vec<SyncSkillRow>,
    /// For a merge, each skill changed on both sides and whose version it kept.
    pub choices: Vec<ChoiceRow>,
}

/// One choice a merge made for a skill changed on both sides.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ChoiceRow {
    /// The skill's slug.
    pub slug: String,
    /// Whose version the merge kept: the merging device's (`local`) or the remote's.
    pub side: Side,
    /// What History says of it, such as `Kept Mac Studio's pdf`.
    pub kept: String,
    /// Where the remote's version won, the commit holding the merging device's version, which
    /// `sync restore --to` takes.
    pub unpicked: Option<String>,
}
