//! Installing a skill from Explore: getting it from its source into the library (ADR 0015).

use crate::{SkillSource, normalized_url};
use serde::{Deserialize, Serialize};

/// Where an install of one source's skill stands. A source with none is idle: it was never
/// installed this session, or its install succeeded or was refused because the library came to
/// hold its slug.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum InstallState {
    /// Its network phase or its commit is running, or it waits for a free Git thread.
    Installing,
    /// The last install failed; Install stays offered.
    Failed(InstallFailure),
}

/// Why an install put nothing in the library.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum InstallFailure {
    /// Git could not read the repository: it is private to someone else, or gone.
    Access,
    /// The repository's host could not be reached.
    Offline,
    /// The fetched commit has no folder at the skill's path.
    NotFound,
    /// The skill's folder is blocked.
    Blocked,
    /// The skill's SKILL.md metadata is invalid.
    Invalid,
    /// The skill's folder has no files.
    Empty,
    /// Anything else, with its user-safe message.
    Other(String),
}

/// What identifies an install: its source's normalized repository URL and folder, as
/// [`same_source`](crate::same_source) compares them.
pub fn install_key(source: &SkillSource) -> String {
    format!(
        "{}#{}",
        normalized_url(&source.url),
        source.path.trim_matches('/')
    )
}
