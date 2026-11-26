//! The found skills a workspace's, an agent's, or Universal's page lists (ADR 0036): those the
//! latest completed scan found in that place that are not in library, each once by key, the
//! ignored ones apart. A workspace's place is every skill directory the scan walks in its folder;
//! an agent's or Universal's is its own global skill directory.

use super::super::import_dialog::versions::{found_skills, skill_directory};
use super::super::import_review::place_name;
use skillbinder_proto::{AppSnapshot, Candidate, CandidateLocation, Workspace, WorkspaceId};
use std::path::Path;

/// Whose page lists the found skills.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Here<'a> {
    Workspace(&'a WorkspaceId),
    /// Your agents reading one global skill directory, by id.
    Agents(&'a [String]),
}

impl Here<'_> {
    /// Whether the scan found `location` in this place.
    fn holds(self, location: &CandidateLocation) -> bool {
        match self {
            Self::Workspace(id) => location.workspaces.contains(id),
            Self::Agents(ids) => location
                .global_agents
                .iter()
                .any(|agent| ids.contains(agent)),
        }
    }
}

/// One found skill a page lists.
#[derive(Debug)]
pub(crate) struct FoundRow<'a> {
    /// The slug compared under folding, which an ignore names.
    pub key: &'a str,
    /// The version the row reads: the preferred one, else the first.
    pub shown: &'a Candidate,
    /// Where any version was found in the page's place, in scan order.
    pub here: Vec<&'a CandidateLocation>,
    /// Where any version was found anywhere else, in scan order.
    pub elsewhere: Vec<&'a CandidateLocation>,
}

impl FoundRow<'_> {
    /// Whether `search`, lowercase and trimmed, matches the slug or the description.
    pub(crate) fn matches(&self, search: &str) -> bool {
        search.is_empty()
            || self.shown.slug.to_lowercase().contains(search)
            || self.shown.description.to_lowercase().contains(search)
    }

    /// The directories holding it in `workspace`, relative to its folder, such as
    /// `.claude/skills`, each once.
    pub(crate) fn dirs_in(&self, workspace: &Workspace) -> Vec<String> {
        let mut dirs: Vec<String> = Vec::new();
        for location in &self.here {
            let directory = skill_directory(location);
            let dir = Path::new(&directory)
                .strip_prefix(&workspace.path)
                .map_or(directory.clone(), |inner| inner.display().to_string());
            if !dirs.contains(&dir) {
                dirs.push(dir);
            }
        }
        dirs
    }

    /// The other places it was found in, as the app names them, each once.
    pub(crate) fn also_in(&self, snapshot: &AppSnapshot) -> Vec<String> {
        let mut places: Vec<String> = Vec::new();
        for location in &self.elsewhere {
            let place = place_name(
                &skill_directory(location),
                &snapshot.agents,
                &snapshot.workspaces,
                &snapshot.home,
            );
            if !places.contains(&place) {
                places.push(place);
            }
        }
        places
    }
}

/// A page's found skills that are not in library, by key.
#[derive(Debug, Default)]
pub(crate) struct FoundHere<'a> {
    pub not_in_library: Vec<FoundRow<'a>>,
    pub ignored: Vec<FoundRow<'a>>,
}

impl FoundHere<'_> {
    pub(crate) fn is_empty(&self) -> bool {
        self.not_in_library.is_empty() && self.ignored.is_empty()
    }
}

/// The found skills of `here`'s page: each skill the latest completed scan found there that is
/// not in library, under Ignored when its key is ignored. None before a scan completes.
pub(crate) fn found_here<'a>(snapshot: &'a AppSnapshot, here: Here<'_>) -> FoundHere<'a> {
    let listed: Vec<&Candidate> = snapshot
        .scan
        .iter()
        .flat_map(|scan| &scan.candidates)
        .filter(|candidate| candidate.in_library().is_none())
        .collect();
    let mut rows: Vec<FoundRow<'a>> = found_skills(&listed)
        .into_iter()
        .filter_map(|skill| {
            let shown = skill
                .versions
                .iter()
                .copied()
                .find(|version| version.preferred)
                .or_else(|| skill.versions.first().copied())?;
            let (here, elsewhere) = skill
                .versions
                .iter()
                .flat_map(|version| &version.locations)
                .partition::<Vec<_>, _>(|location| here.holds(location));
            (!here.is_empty()).then_some(FoundRow {
                key: skill.key,
                shown,
                here,
                elsewhere,
            })
        })
        .collect();
    rows.sort_by_key(|row| row.key);
    let (ignored, not_in_library) = rows
        .into_iter()
        .partition(|row| snapshot.ignored.contains_key(row.key));
    FoundHere {
        not_in_library,
        ignored,
    }
}

#[cfg(test)]
mod tests;
