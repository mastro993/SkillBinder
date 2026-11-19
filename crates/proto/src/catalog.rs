//! Explore's catalog: the curated list of creators and their skills that SkillBinder publishes,
//! as the engine translated and cached it.

use crate::{
    InstallAllProgress, InstallState, Preview, Repository, RepositoryId, Resolution, SkillSource,
    SkillsShSearch,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

/// A creator: a GitHub owner whose repositories the catalog lists, such as anthropics.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Creator {
    /// Stable identity the catalog's skills name.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Absolute path to the creator's cached avatar, when one was fetched; initials show without
    /// it.
    pub avatar: Option<String>,
}

/// Where a catalog skill comes from.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CatalogOrigin {
    /// The curated catalog, published by one of its creators.
    Curated {
        /// The creator's [`Creator::id`].
        creator: String,
    },
    /// One of your repositories.
    Repository {
        /// The repository's [`Repository::id`].
        id: RepositoryId,
    },
    /// A skills.sh search result whose folder was resolved.
    SkillsSh {
        /// All-time installs on skills.sh.
        installs: u64,
    },
}

/// One skill of the catalog.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CatalogSkill {
    /// The skill's slug: its SKILL.md name, or its folder's name without one.
    pub slug: String,
    /// The slug compared under NFKC and full case folding, as [`LibrarySnapshot::keys`] holds
    /// library skills' slugs.
    ///
    /// [`LibrarySnapshot::keys`]: crate::LibrarySnapshot::keys
    pub key: String,
    /// Display name.
    pub name: String,
    /// Description, at most 1,024 characters.
    pub description: String,
    /// Who publishes it, or that skills.sh found it.
    pub origin: CatalogOrigin,
    /// The repository and folder it would be installed from, with no pinned ref.
    pub source: SkillSource,
    /// Installs gained over the last 7 days; 0 for a repository's skill, which has no count, and
    /// for a skills.sh result, whose origin holds its all-time installs instead.
    pub installs_week: u64,
}

/// The catalog's creators and skills; skills are ordered by installs this week, highest first.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Catalog {
    /// Creators with at least one skill.
    pub creators: Vec<Creator>,
    /// Skills, each from a creator in `creators`.
    pub skills: Vec<CatalogSkill>,
}

/// How the last refresh of the catalog went.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum CatalogState {
    /// A refresh is running, or none has run yet.
    #[default]
    Loading,
    /// The catalog is current: the source answered with it or said it was unchanged.
    Ready,
    /// The source could not be reached.
    Offline,
    /// The source answered with an error or a file the app cannot read.
    Unavailable,
    /// The source no longer serves the format this app reads.
    Outdated,
}

/// Explore's projection: the catalog state, the cached catalog, if any, your repositories, the
/// installs that run or failed this session, the open detail dialog's preview, and the skills.sh
/// search with the resolutions of its results.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExploreSnapshot {
    /// How the last refresh went.
    pub state: CatalogState,
    /// The cached catalog; `None` before any refresh succeeded on this device.
    pub catalog: Option<Arc<Catalog>>,
    /// Each install that is not idle, by [`install_key`](crate::install_key).
    pub installs: BTreeMap<String, InstallState>,
    /// The preview of the skill whose detail dialog is open, if one is.
    pub preview: Option<Preview>,
    /// Your repositories, in the order they were added, each with its last listing.
    pub repositories: Arc<Vec<Repository>>,
    /// Each running Install all, by repository.
    pub install_all: BTreeMap<RepositoryId, InstallAllProgress>,
    /// The skills.sh search for the last query asked.
    pub skills_sh: SkillsShSearch,
    /// Each skills.sh result's folder something asked for this session, by
    /// [`result_key`](crate::result_key).
    pub resolutions: BTreeMap<String, Resolution>,
}

/// `url` normalized so that two spellings of one repository compare equal: the scheme and host
/// lowercased, and for `github.com` the owner and repository too, which GitHub treats
/// case-insensitively; a trailing `/` and `.git` dropped; and an SCP-style `user@host:path`
/// mapped to `ssh://user@host/path`. The HTTPS and SSH URLs of one repository stay different.
pub fn normalized_url(url: &str) -> String {
    let url = url.trim();
    let (scheme, rest) = match url.split_once("://") {
        Some((scheme, rest)) => (scheme.to_ascii_lowercase(), rest.to_owned()),
        None => match url.split_once(':') {
            Some((host, path)) => ("ssh".to_owned(), format!("{host}/{path}")),
            None => return url.to_owned(),
        },
    };
    let (authority, path) = rest.split_once('/').unwrap_or((&rest, ""));
    let (user, host) = match authority.rsplit_once('@') {
        Some((user, host)) => (format!("{user}@"), host.to_ascii_lowercase()),
        None => (String::new(), authority.to_ascii_lowercase()),
    };
    let path = path.trim_matches('/');
    let path = path
        .strip_suffix(".git")
        .unwrap_or(path)
        .trim_end_matches('/');
    let path = if host == "github.com" {
        path.to_ascii_lowercase()
    } else {
        path.to_owned()
    };
    format!("{scheme}://{user}{host}/{path}")
}

/// Whether two sources name the same folder of the same repository, comparing normalized URLs
/// and ignoring any pinned ref.
pub fn same_source(a: &SkillSource, b: &SkillSource) -> bool {
    a.path.trim_matches('/') == b.path.trim_matches('/')
        && normalized_url(&a.url) == normalized_url(&b.url)
}

/// The repository's `owner/repo`: the normalized URL's last two path segments.
pub fn owner_repo(url: &str) -> String {
    let normalized = normalized_url(url);
    let path = normalized
        .split_once("://")
        .map_or(normalized.as_str(), |(_, rest)| rest);
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).skip(1).collect();
    segments[segments.len().saturating_sub(2)..].join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(url: &str, path: &str) -> SkillSource {
        SkillSource {
            url: url.to_owned(),
            reference: None,
            path: path.to_owned(),
        }
    }

    #[test]
    fn spellings_of_one_repository_normalize_alike() {
        let expected = "https://github.com/mattpocock/skills";
        for url in [
            "https://github.com/mattpocock/skills.git",
            "https://GitHub.com/MattPocock/Skills/",
            "HTTPS://github.com/mattpocock/skills.git/",
        ] {
            assert_eq!(normalized_url(url), expected, "{url}");
        }
        assert_eq!(
            normalized_url("git@GitHub.com:Owner/Repo.git"),
            "ssh://git@github.com/owner/repo"
        );
        // Outside github.com, paths keep their case.
        assert_eq!(
            normalized_url("https://Example.com/Team/Repo.git"),
            "https://example.com/Team/Repo"
        );
    }

    #[test]
    fn https_and_ssh_are_different_sources() {
        let https = source("https://github.com/o/r.git", "skills/pdf");
        assert!(same_source(
            &https,
            &source("https://github.com/O/R", "skills/pdf/")
        ));
        assert!(!same_source(
            &https,
            &source("git@github.com:o/r.git", "skills/pdf")
        ));
        assert!(!same_source(
            &https,
            &source("https://github.com/o/r.git", "skills/docx")
        ));
    }

    #[test]
    fn owner_repo_is_the_last_two_segments() {
        assert_eq!(
            owner_repo("https://github.com/Anthropics/Skills.git"),
            "anthropics/skills"
        );
        assert_eq!(owner_repo("git@gitlab.com:group/sub/repo.git"), "sub/repo");
        assert_eq!(owner_repo("https://example.com/repo"), "repo");
    }
}
