//! Skills changed on both sides (ADR 0039): which version each side holds, and the choice Sync
//! needs for each before it merges.

use super::{ChangeKind, Reason};
use serde::{Deserialize, Serialize};

/// Whose version of a skill changed on both sides a merge keeps.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// This device's version.
    Local,
    /// The remote's version.
    Remote,
}

impl Side {
    /// The word a merge's `SkillBinder-Choice` trailer stores it as.
    pub fn word(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Remote => "remote",
        }
    }

    /// The side a `SkillBinder-Choice` trailer's word names.
    pub fn stored(word: &str) -> Option<Self> {
        match word {
            "local" => Some(Self::Local),
            "remote" => Some(Self::Remote),
            _ => None,
        }
    }
}

/// A skill both sides changed, since the commit they share, to different versions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Conflict {
    /// The skill's slug.
    pub slug: String,
    /// How and why this device changed it.
    pub local: ConflictSide,
    /// How and why the remote changed it, and which device did, when.
    pub remote: ConflictSide,
}

/// One side's change to a skill changed on both.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConflictSide {
    /// How it changed.
    pub kind: ChangeKind,
    /// Why it changed; unknown for a commit made before device names.
    pub reason: Option<Reason>,
    /// The device whose commit made the remote's version; none on this device's side, or for a
    /// commit made before device names.
    pub device: Option<String>,
    /// When the commit that made the remote's version was made, as Unix seconds; none on this
    /// device's side, whose change is not pushed yet.
    pub at: Option<u64>,
}

/// One choice a merge made for a skill changed on both sides, as History lists it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    /// The skill's slug.
    pub slug: String,
    /// Whose version the merge kept.
    pub side: Side,
    /// The device whose version it kept, if named.
    pub device: Option<String>,
    /// Where the remote's version won, the commit holding the merging device's version, which
    /// Restore reads; none where that device had removed the skill.
    pub unpicked: Option<String>,
}
