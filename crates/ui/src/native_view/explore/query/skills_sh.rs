//! What Explore shows of skills.sh during a search, derived from the snapshot alone: where its
//! search stands, its results without the skills already shown, and "In library" for a result
//! whose folder is not resolved yet.

use super::{Local, held_from};
use skillbinder_proto::{
    AppSnapshot, CatalogSkill, CatalogState, Resolution, SkillsShResult, SkillsShState,
    normalized_url, result_key,
};

/// A skills.sh result as its row shows it: the skill its folder resolved to, or the result
/// itself until then.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in super::super) enum Found<'a> {
    Skill(&'a CatalogSkill),
    Result(&'a SkillsShResult),
}

impl Found<'_> {
    /// Its repository URL and slug key, which a result already shown shares.
    fn identity(&self) -> (String, &str) {
        match self {
            Self::Skill(skill) => (normalized_url(&skill.source.url), &skill.key),
            Self::Result(result) => (normalized_url(&result.url), &result.key),
        }
    }
}

/// Where skills.sh stands for the search shown.
#[derive(Debug, PartialEq)]
pub(in super::super) enum Remote<'a> {
    /// Nothing is asked: the search is shorter than two characters.
    Idle,
    /// Its answer is on its way: the pause before asking, or the request.
    Searching,
    /// Its results in its order, without the skills the local results show.
    Results(Vec<Found<'a>>),
    /// The catalog is offline, or skills.sh could not be reached.
    NeedsConnection,
    /// skills.sh answered with an error or something the app cannot read.
    Unavailable,
}

impl Remote<'_> {
    /// The count its chip shows, once it answered.
    pub(in super::super) fn count(&self) -> Option<usize> {
        match self {
            Self::Results(found) => Some(found.len()),
            Self::Idle | Self::Searching | Self::NeedsConnection | Self::Unavailable => None,
        }
    }
}

/// skills.sh for `search`, already trimmed and lowercased, beside the matches of the catalog and
/// your repositories.
pub(in super::super) fn remote<'a>(
    snapshot: &'a AppSnapshot,
    local: Local<'_>,
    search: &str,
) -> Remote<'a> {
    if search.chars().count() < 2 {
        return Remote::Idle;
    }
    if snapshot.explore.state == CatalogState::Offline {
        return Remote::NeedsConnection;
    }
    let skills_sh = &snapshot.explore.skills_sh;
    if skills_sh.query != search {
        return Remote::Searching;
    }
    match &skills_sh.state {
        SkillsShState::Idle | SkillsShState::Searching => Remote::Searching,
        SkillsShState::NeedsConnection => Remote::NeedsConnection,
        SkillsShState::Unavailable => Remote::Unavailable,
        SkillsShState::Results(results) => {
            let shown: Vec<_> = local
                .results(search, None)
                .into_iter()
                .map(|skill| (normalized_url(&skill.source.url), skill.key.as_str()))
                .collect();
            Remote::Results(
                results
                    .iter()
                    .map(|result| resolved(snapshot, result))
                    .filter(|found| !shown.contains(&found.identity()))
                    .collect(),
            )
        }
    }
}

/// `result` as its row shows it.
fn resolved<'a>(snapshot: &'a AppSnapshot, result: &'a SkillsShResult) -> Found<'a> {
    match snapshot.explore.resolutions.get(&result_key(result)) {
        Some(Resolution::Resolved(skill)) => Found::Skill(skill.as_ref()),
        Some(Resolution::Resolving | Resolution::NotFound | Resolution::Failed) | None => {
            Found::Result(result)
        }
    }
}

/// Whether the library holds `result`'s `skillId` from its repository, before its folder is
/// known.
pub(crate) fn result_in_library(snapshot: &AppSnapshot, result: &SkillsShResult) -> bool {
    let url = normalized_url(&result.url);
    held_from(snapshot, &result.key, |source| {
        normalized_url(&source.url) == url
    })
    .is_some()
}

/// Whether the library holds `result`'s `skillId` from another repository or from none.
pub(crate) fn result_collides(snapshot: &AppSnapshot, result: &SkillsShResult) -> bool {
    snapshot
        .library
        .as_ref()
        .is_some_and(|library| library.keys.contains_key(&result.key))
        && !result_in_library(snapshot, result)
}
