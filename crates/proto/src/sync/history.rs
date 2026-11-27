//! History (ADR 0038): the library's published changes, newest first, a page at a time.

use super::ChangedSkill;
use serde::{Deserialize, Serialize};

/// One commit on the library's first-parent chain: who made it, when, and the skills it changed.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SyncChange {
    /// The commit's id, which [`SyncHistory::next`] and later Restore name it by.
    pub commit: String,
    /// When it was made, as Unix seconds.
    pub at: u64,
    /// The device that made it; none for a commit made before device names.
    pub device: Option<String>,
    /// Each skill it changed against its first parent, added first, then changed, then removed,
    /// each by slug.
    pub skills: Vec<ChangedSkill>,
}

/// A page of History.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SyncHistory {
    /// The changes, newest first.
    pub changes: Vec<SyncChange>,
    /// The commit the next page starts after, while older changes remain.
    pub next: Option<String>,
}
