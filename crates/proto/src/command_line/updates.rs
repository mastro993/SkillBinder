//! What `updates` answers: Library's Updates filter, and the sources the last check couldn't
//! reach.

use super::SkillRow;
use serde::{Deserialize, Serialize};

/// The skills with an update, as of the last check.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UpdatesOutput {
    /// The skills whose source has a newer version, as Library lists them.
    pub skills: Vec<SkillRow>,
    /// The skills whose source the last check couldn't read.
    pub unchecked: Vec<Unchecked>,
}

/// A skill whose source couldn't be checked.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Unchecked {
    /// The skill's slug.
    pub slug: String,
    /// Why, as the skill's page says it.
    pub reason: String,
}
