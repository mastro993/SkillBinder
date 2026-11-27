//! What waits to sync on each side (Design 4): each skill's kind of change and why it changed.

use super::Reason;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How a skill changed, by whether its directory is there before and after.
#[derive(
    Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    /// Its directory is new.
    Added,
    /// Its directory or record changed.
    Changed,
    /// Its directory is gone.
    Removed,
}

/// One skill that changed, how, and why.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChangedSkill {
    /// The skill's slug.
    pub slug: String,
    /// How it changed.
    pub kind: ChangeKind,
    /// Why it changed; unknown for a commit made before device names.
    pub reason: Option<Reason>,
}

/// What the remote holds that this device has not pulled yet.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Incoming {
    /// The devices whose commits changed it, newest first; empty for commits made before device
    /// names.
    pub devices: Vec<String>,
    /// When the remote's newest commit was made, as Unix seconds.
    pub at: Option<u64>,
    /// Each skill that changed, added first, then changed, then removed, each by slug.
    pub skills: Vec<ChangedSkill>,
}
