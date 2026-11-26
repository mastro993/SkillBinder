//! The Import skills dialog's list (design D3 to D8, I5 to I8): each found skill not in library
//! once, the offered ones grouped under Your agents and each workspace and the ignored ones apart,
//! with the plan item that imports it, and the ticks over the offered ones by key.

use super::versions::{FoundSkill, found_skills};
use skillbinder_proto::{
    AppSnapshot, Candidate, CandidateId, FoundImport, ImportItem, MAX_IMPORT, ValidationStatus,
    Workspace, WorkspaceId,
};
use std::collections::{BTreeMap, BTreeSet};

/// Where a group's skills were found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Place {
    /// Your agents' global skill directories, and anywhere no workspace reads.
    Agents,
    Workspace(WorkspaceId),
}

impl Place {
    /// The group's name: `Your agents`, or the workspace's.
    pub(crate) fn name(&self, workspaces: &[Workspace]) -> String {
        match self {
            Self::Agents => "Your agents".to_owned(),
            Self::Workspace(id) => workspaces
                .iter()
                .find(|workspace| &workspace.id == id)
                .map_or_else(
                    || "Workspace".to_owned(),
                    |workspace| workspace.name.clone(),
                ),
        }
    }
}

/// One found skill and the plan's item for it.
pub(crate) struct Row<'a> {
    pub skill: FoundSkill<'a>,
    /// The item that imports it; none when no version can be imported, such as a blocked one.
    pub item: Option<&'a ImportItem>,
}

impl<'a> Row<'a> {
    pub(crate) fn key(&self) -> &'a str {
        self.skill.key
    }

    /// The version the row shows: the planned one, else the first.
    pub(crate) fn shown(&self) -> Option<&'a Candidate> {
        let planned = self.item.map(|item| &item.candidate_id);
        let versions = &self.skill.versions;
        versions
            .iter()
            .copied()
            .find(|version| Some(&version.id) == planned)
            .or_else(|| versions.first().copied())
    }

    pub(crate) fn tickable(&self) -> bool {
        self.item.is_some()
    }

    pub(crate) fn invalid(&self) -> bool {
        self.item
            .is_some_and(|item| item.validation == ValidationStatus::Invalid)
    }

    /// Whether `search`, lowercase and trimmed, matches the slug or the description.
    pub(super) fn matches(&self, search: &str) -> bool {
        search.is_empty()
            || self.shown().is_some_and(|version| {
                version.slug.to_lowercase().contains(search)
                    || version.description.to_lowercase().contains(search)
            })
    }
}

pub(crate) struct Group<'a> {
    pub place: Place,
    pub rows: Vec<Row<'a>>,
}

/// `found` holding only the found skills whose keys `only` names, when it names some, so every
/// list, tick, and count of the dialog sees just those (U6). The plan keeps its items.
pub(crate) fn narrowed(mut found: FoundImport, only: Option<&BTreeSet<String>>) -> FoundImport {
    if let Some(only) = only {
        found
            .scan
            .candidates
            .retain(|candidate| only.contains(&candidate.key));
    }
    found
}

/// Each found skill of `found` not in library with the plan's item for it, in scan order.
fn found_rows(found: &FoundImport) -> impl Iterator<Item = Row<'_>> {
    let listed: Vec<&Candidate> = found
        .scan
        .candidates
        .iter()
        .filter(|candidate| candidate.in_library().is_none())
        .collect();
    found_skills(&listed).into_iter().map(|skill| {
        let item = found.plan.items.iter().find(|item| {
            let planned = &item.candidate_id;
            skill.versions.iter().any(|version| &version.id == planned)
        });
        Row { skill, item }
    })
}

/// The rows of `found` offered for import, every one but the `ignored` keys, grouped by the place
/// of each skill's first location: Your agents first, then each workspace in sidebar order, each
/// in scan order.
pub(crate) fn groups<'a>(
    found: &'a FoundImport,
    workspaces: &[Workspace],
    ignored: &BTreeMap<String, String>,
) -> Vec<Group<'a>> {
    let mut groups: Vec<Group<'a>> = Vec::new();
    for row in found_rows(found).filter(|row| !ignored.contains_key(row.key())) {
        let place = place(&row.skill);
        match groups.iter_mut().find(|group| group.place == place) {
            Some(group) => group.rows.push(row),
            None => groups.push(Group {
                place,
                rows: vec![row],
            }),
        }
    }
    let position = |place: &Place| match place {
        Place::Agents => (0, 0),
        Place::Workspace(id) => (
            1,
            workspaces
                .iter()
                .find(|workspace| &workspace.id == id)
                .map_or(u32::MAX, |workspace| workspace.position),
        ),
    };
    groups.sort_by_key(|group| position(&group.place));
    groups
}

/// The rows of `found` whose keys are `ignored`, in scan order (I5).
pub(crate) fn ignored_rows<'a>(
    found: &'a FoundImport,
    ignored: &BTreeMap<String, String>,
) -> Vec<Row<'a>> {
    found_rows(found)
        .filter(|row| ignored.contains_key(row.key()))
        .collect()
}

fn place(skill: &FoundSkill<'_>) -> Place {
    let first = skill
        .versions
        .first()
        .and_then(|version| version.locations.first());
    match first {
        Some(location) if location.global_agents.is_empty() => location
            .workspaces
            .first()
            .map_or(Place::Agents, |id| Place::Workspace(id.clone())),
        _ => Place::Agents,
    }
}

/// Every row of `groups`, in display order.
pub(crate) fn rows<'g, 'a>(groups: &'g [Group<'a>]) -> impl Iterator<Item = &'g Row<'a>> {
    groups.iter().flat_map(|group| &group.rows)
}

/// The ticks a fresh list starts with: every importable valid skill, up to 500, in display order.
pub(crate) fn default_ticks(groups: &[Group<'_>]) -> BTreeSet<String> {
    rows(groups)
        .filter(|row| row.tickable() && !row.invalid())
        .take(MAX_IMPORT)
        .map(|row| row.key().to_owned())
        .collect()
}

/// The keys of the importable rows of `rows`, in order.
pub(super) fn tickable<'r, 'a: 'r>(rows: impl IntoIterator<Item = &'r Row<'a>>) -> Vec<String> {
    rows.into_iter()
        .filter(|row| row.tickable())
        .map(|row| row.key().to_owned())
        .collect()
}

/// Ticks the skills `keys` in order until `MAX_IMPORT` are ticked, or unticks them all.
pub(super) fn tick_all(ticked: &mut BTreeSet<String>, keys: &[String], on: bool) {
    for key in keys {
        if !on {
            ticked.remove(key);
        } else if ticked.len() < MAX_IMPORT {
            ticked.insert(key.clone());
        }
    }
}

/// How a set of rows reads against the ticks: how many importable ones there are and how many of
/// them are ticked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Count {
    pub ticked: usize,
    pub able: usize,
}

impl Count {
    pub(super) fn of<'r, 'a: 'r>(
        ticked: &BTreeSet<String>,
        rows: impl IntoIterator<Item = &'r Row<'a>>,
    ) -> Self {
        let able: Vec<_> = rows.into_iter().filter(|row| row.tickable()).collect();
        Self {
            ticked: able.iter().filter(|row| ticked.contains(row.key())).count(),
            able: able.len(),
        }
    }

    /// Whether a checkbox over these rows reads ticked: as many are ticked as one import takes.
    pub(super) fn full(self) -> bool {
        self.ticked > 0 && self.ticked >= self.able.min(MAX_IMPORT)
    }

    /// Whether the checkbox reads partly ticked: some are ticked, but not one import's worth.
    pub(super) fn partial(self) -> bool {
        self.ticked > 0 && !self.full()
    }
}

/// What Import takes: the ticked importable skills' planned versions, and whether one is invalid.
pub(crate) fn selection(groups: &[Group<'_>], ticked: &BTreeSet<String>) -> Selection {
    let chosen: Vec<&Row<'_>> = rows(groups)
        .filter(|row| ticked.contains(row.key()))
        .collect();
    Selection {
        ids: chosen
            .iter()
            .filter_map(|row| row.item.map(|item| item.candidate_id.clone()))
            .collect(),
        invalid: chosen.iter().any(|row| row.invalid()),
    }
}

pub(crate) struct Selection {
    pub ids: Vec<CandidateId>,
    pub invalid: bool,
}

impl Selection {
    pub(crate) fn over(&self) -> bool {
        self.ids.len() > MAX_IMPORT
    }

    /// Whether Import can run, with the invalid metadata acknowledged or not.
    pub(crate) fn ready(&self, acknowledged: bool) -> bool {
        !self.ids.is_empty() && !self.over() && (acknowledged || !self.invalid)
    }
}

/// The Import button's label for `count` ticked skills (D10, O4).
pub(in crate::native_view) fn import_label(count: usize) -> String {
    match count {
        0 => "Import skills".to_owned(),
        1 => "Import 1 skill".to_owned(),
        count => format!("Import {count} skills"),
    }
}

/// How many found skills the latest completed scan offers: not in library and not ignored,
/// counting a skill with several versions once (L2, I8).
pub(crate) fn found_count(snapshot: &AppSnapshot) -> usize {
    let offered: Vec<&Candidate> = snapshot.scan.as_ref().map_or_else(Vec::new, |scan| {
        scan.candidates
            .iter()
            .filter(|candidate| candidate.in_library().is_none())
            .filter(|candidate| !snapshot.ignored.contains_key(&candidate.key))
            .collect()
    });
    found_skills(&offered).len()
}
