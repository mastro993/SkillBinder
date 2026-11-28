//! What `updates` answers: Library's Updates filter, and the sources the last check couldn't
//! reach; and what `update --diff` answers: each named skill's review.

use super::SkillRow;
use crate::UpdateReview;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The skills with an update, as of the last check.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct UpdatesOutput {
    /// The skills whose source has a newer version, as Library lists them.
    pub skills: Vec<SkillRow>,
    /// The skills whose source the last check couldn't read.
    pub unchecked: Vec<Unchecked>,
}

/// A skill whose source couldn't be checked.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Unchecked {
    /// The skill's slug.
    pub slug: String,
    /// Why, as the skill's page says it.
    pub reason: String,
}

/// What Update would change in the named skills, as their review dialogs show it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ReviewsOutput {
    /// Each named skill with an update, its changes from its library content to its source's.
    pub reviews: Vec<UpdateReview>,
    /// The named skills without an update, by slug.
    pub current: Vec<String>,
}
