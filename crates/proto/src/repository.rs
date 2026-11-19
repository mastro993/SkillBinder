//! Your repositories: Git repositories the user added to Explore to browse and install their
//! skills. They are kept on this device only and never sync.

use crate::{AppError, CatalogSkill, InstallFailure, RepositoryId, SkillId};
use serde::{Deserialize, Serialize};

/// A repository the user added, with its last listing.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Repository {
    /// Durable identity on this device.
    pub id: RepositoryId,
    /// The Git URL as the user entered it, which Git fetches from.
    pub url: String,
    /// Its skills as last listed, each with this repository as its origin.
    pub skills: Vec<CatalogSkill>,
    /// Unix seconds of the last listing that reached the repository.
    pub listed_at: u64,
    /// Unix seconds of when it was added.
    pub added_at: u64,
}

/// A repository's skills, listed before it is added.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryListing {
    /// The slugs found, in the order they were found.
    pub slugs: Vec<String>,
}

/// Why a repository cannot be listed or added; nothing is saved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepositoryProblem {
    /// The URL is not an HTTPS, SSH, or SCP-style Git URL without credentials.
    NotGitUrl,
    /// Git could not fetch it: the host is unreachable, or it refused access.
    Unreachable,
    /// It holds no skill.
    NoSkills,
    /// A repository with the same normalized URL is added already.
    AlreadyAdded,
    /// Git is missing.
    NeedsGit,
    /// Anything else, such as storage that cannot be written.
    Failed(AppError),
}

impl From<AppError> for RepositoryProblem {
    fn from(error: AppError) -> Self {
        Self::Failed(error)
    }
}

/// How far a running Install all has got.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InstallAllProgress {
    /// Installs that ended, installed or failed.
    pub done: usize,
    /// Skills it installs in all.
    pub total: usize,
}

/// How an Install all ended.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InstallAllOutcome {
    /// The skills it put in the library, in the order they landed.
    pub installed: Vec<SkillId>,
    /// Each skill that failed, by slug, with why.
    pub failed: Vec<(String, InstallFailure)>,
}
