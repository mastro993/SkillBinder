//! Reviewing one skill's update before it lands (ADR 0037): what the update would change, and
//! what Update did with the reviewed version.

use crate::{SkillId, SkillSource, UpdateOutcome};
use serde::{Deserialize, Serialize};

/// What updating a library skill from its source would change, from its current library content
/// to the fetched content.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UpdateReview {
    /// The skill reviewed.
    pub id: SkillId,
    /// The source fetched.
    pub source: SkillSource,
    /// The manifest digest of the skill's library content the changes start from.
    pub from: String,
    /// The Git tree id of the fetched content the changes end at.
    pub to: String,
    /// Each file the update adds, changes, or removes, sorted by path.
    pub files: Vec<ReviewedFile>,
    /// The changes as one Git-style unified patch, in the order of `files`, each file with its
    /// whole text as context.
    pub patch: String,
}

impl UpdateReview {
    /// The lines the update adds and removes across its files.
    pub fn totals(&self) -> (u64, u64) {
        self.files.iter().fold((0, 0), |(added, removed), file| {
            (added + file.additions, removed + file.deletions)
        })
    }
}

/// One file an update changes, its path relative to the skill's folder with `/` separators.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReviewedFile {
    /// Where the file is in the skill's folder.
    pub path: String,
    /// How the update changes it.
    pub change: FileChange,
    /// Lines added; none for a binary file.
    pub additions: u64,
    /// Lines removed; none for a binary file.
    pub deletions: u64,
}

/// How an update changes a file.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum FileChange {
    /// Only the fetched content has it.
    Added,
    /// Its content or its executable bit changes.
    Changed,
    /// Only the library content has it.
    Removed,
}

/// The version a review showed, which Update lands only while both ends still hold.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReviewedUpdate {
    /// The skill reviewed.
    pub id: SkillId,
    /// The reviewed library content's manifest digest.
    pub from: String,
    /// The reviewed fetched content's Git tree id.
    pub to: String,
}

impl From<&UpdateReview> for ReviewedUpdate {
    fn from(review: &UpdateReview) -> Self {
        Self {
            id: review.id.clone(),
            from: review.from.clone(),
            to: review.to.clone(),
        }
    }
}

/// What Update did with a reviewed version.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ReviewedOutcome {
    /// It landed, or failed as Update does, its state recording why.
    Updated(UpdateOutcome),
    /// The source or the library content moved since the review, so nothing was written; the
    /// newest changes are to be reviewed instead.
    Moved {
        /// Which end moved.
        moved: Moved,
        /// The changes now.
        review: Box<UpdateReview>,
    },
}

/// Which end of a review moved before Update.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Moved {
    /// The source's content changed; checked first.
    Source,
    /// The skill's library content changed, such as by a Pull.
    Library,
}
