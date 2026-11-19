//! skills.sh search: the long tail Explore's search asks skills.sh for while the user types
//! (ADR 0014). A result names a repository and a `skillId`, not the skill's folder, which the
//! engine resolves before the skill can be previewed or installed.

use crate::{CatalogSkill, normalized_url};
use serde::{Deserialize, Serialize};

/// One skills.sh search result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillsShResult {
    /// skills.sh's `skillId`, the result's slug until its folder is resolved.
    pub skill_id: String,
    /// `skill_id` compared under NFKC and full case folding, as
    /// [`CatalogSkill::key`] compares slugs.
    pub key: String,
    /// Display name.
    pub name: String,
    /// The repository: `https://github.com/<owner>/<repo>.git`.
    pub url: String,
    /// All-time installs on skills.sh.
    pub installs: u64,
}

/// Where the skills.sh search for the current query stands.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum SkillsShState {
    /// No search: the query is shorter than two characters.
    #[default]
    Idle,
    /// The request for the query is running.
    Searching,
    /// skills.sh's results, in its order.
    Results(Vec<SkillsShResult>),
    /// skills.sh could not be reached.
    NeedsConnection,
    /// skills.sh answered with an error, or with something the app cannot read.
    Unavailable,
}

/// The skills.sh search for the last query asked.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillsShSearch {
    /// The query, trimmed, as it was asked.
    pub query: String,
    /// Where its search stands; an answer to an older query never shows here.
    pub state: SkillsShState,
}

/// Where the folder of a skills.sh result stands, once something asked for it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Resolution {
    /// Its repository is being fetched and searched.
    Resolving,
    /// The skill found: its slug, description, and source with the folder.
    Resolved(CatalogSkill),
    /// No folder's SKILL.md name, or folder name, is the result's `skillId`.
    NotFound,
    /// The repository could not be fetched or read; resolving again retries.
    Failed,
}

/// What identifies a skills.sh result, its resolution, and an install of it before resolution:
/// its normalized repository URL and its `skillId`.
pub fn result_key(result: &SkillsShResult) -> String {
    format!(
        "{}#skills.sh:{}",
        normalized_url(&result.url),
        result.skill_id
    )
}
