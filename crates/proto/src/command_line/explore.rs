//! What `scan`, `search`, and `repos` answer: the Import skills dialog's list, Explore's results,
//! and your repositories.

use crate::ValidationStatus;
use serde::{Deserialize, Serialize};

/// A found skill not in library, as the Import skills dialog lists it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FoundRow {
    /// The skill's slug.
    pub slug: String,
    /// Its description.
    pub description: String,
    /// The dialog's group: `Your agents`, or the workspace it was found in.
    pub place: String,
    /// `import --all` imports it, as the dialog ticks it.
    pub ticked: bool,
    /// Its versions, in the order the scan lists them.
    pub versions: Vec<FoundVersion>,
}

/// One version of a found skill.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FoundVersion {
    /// Its short digest, which `import <slug>@<version>` names.
    pub version: String,
    /// It is the version an import takes unless another is named.
    pub planned: bool,
    /// Unix seconds of its newest file's modification, when known.
    pub modified: Option<u64>,
    /// Its validation; a blocked version can't be imported.
    pub validation: ValidationStatus,
    /// Where it was found: each skill directory, with the agents or workspace reading it.
    pub locations: Vec<String>,
    /// The places whose copy importing it replaces.
    pub replaces: Vec<String>,
}

/// What `search` found.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SearchOutput {
    /// The catalog's and your repositories' matches, then skills.sh's not already listed.
    pub results: Vec<SearchRow>,
    /// How skills.sh answered.
    pub skills_sh: SkillsShAnswer,
}

/// A skill `search` found.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SearchRow {
    /// What `install` takes for it: `<owner>/<repo>/<slug>`, or skills.sh's
    /// `<owner>/<repo>@<skill>`.
    pub reference: String,
    /// Its name.
    pub name: String,
    /// Its description; empty for a skills.sh result.
    pub description: String,
    /// Who publishes it: the catalog's creator, the repository's `<owner>/<repo>`, or
    /// `skills.sh`.
    pub source: String,
    /// Its installs as Explore shows them, such as `+900 this week`.
    pub installs: Option<String>,
    /// The library holds it from this source.
    pub in_library: bool,
    /// The library holds another skill by its name, so it can't be installed.
    pub collides: bool,
}

/// How skills.sh answered a search.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillsShAnswer {
    /// Its results are listed.
    Answered,
    /// Not asked: the search is shorter than two characters.
    NotAsked,
    /// skills.sh could not be reached.
    Offline,
    /// skills.sh answered with an error or something the app can't read.
    Unavailable,
}

/// One of your repositories, as Explore lists it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RepositoryRow {
    /// Its `<owner>/<repo>`.
    pub name: String,
    /// Its Git URL.
    pub url: String,
    /// The skills it lists.
    pub skills: usize,
    /// The skills `repo install-all` installs: those whose slug the library does not hold.
    pub installable: usize,
}
