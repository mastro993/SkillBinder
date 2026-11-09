//! Page state derived from the app snapshot: where each library skill is on, whether it is
//! Active, and which skills Library shows for its folder tab, search, and filters. Render files
//! keep only layout. The active preset is an argument, since the view prefers a preset just
//! clicked over the snapshot's.

use super::library_copies::{AgentRow, Reader, agent_rows};
use super::workspace_targets::dir_rows;
use super::{library_targets, workspace_targets};
use skillbinder_proto::{
    AppSnapshot, FolderId, LibrarySnapshot, PresetId, Skill, SkillId, Targets, Workspace,
    WorkspaceTargets,
};

/// Whether skill `id`'s source has a newer version. Drift and a failed check are not updates.
pub(super) fn has_update(snapshot: &AppSnapshot, id: &SkillId) -> bool {
    snapshot
        .updates
        .get(id)
        .is_some_and(|update| update.available())
}

/// One toggle: a skill directory and every agent that reads it. Agents that read the same
/// directory are one toggle, since the directory is what holds the copy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Entry {
    /// The first row reading the directory, whose reader, name, and logo the toggle shows.
    pub row: AgentRow,
    /// What the toggle changes: agent ids for your agents, directories for a workspace.
    pub keys: Vec<String>,
    /// The name of every agent that reads the directory.
    pub readers: Vec<String>,
}

impl Entry {
    /// Groups rows, each with its keys, by the folder they resolve to, keeping the first row of
    /// each and their order.
    fn grouped(rows: impl IntoIterator<Item = (AgentRow, Vec<String>)>) -> Vec<Self> {
        let mut entries: Vec<Self> = Vec::new();
        for (row, keys) in rows {
            let readers = match &row.reader {
                Reader::Universal(names) => names.clone(),
                Reader::Agent(_) => vec![row.name.clone()],
            };
            match entries
                .iter_mut()
                .find(|entry| entry.row.resolved == row.resolved)
            {
                Some(entry) => {
                    entry.keys.extend(keys);
                    entry.readers.extend(readers);
                }
                None => entries.push(Self { row, keys, readers }),
            }
        }
        entries
    }

    /// The tooltip's second line: the agents that read the directory, when the name does not
    /// already say it.
    pub(super) fn detail(&self) -> Option<String> {
        let universal = matches!(self.row.reader, Reader::Universal(_));
        (universal || self.readers.len() > 1).then(|| self.readers.join(", "))
    }
}

/// A workspace and the directories its line offers.
pub(super) struct WorkspaceEntries<'a> {
    pub workspace: &'a Workspace,
    pub entries: Vec<Entry>,
}

/// Every place a skill can be on: your agents' directories, then each workspace's, in sidebar
/// order. Built once per render.
pub(super) struct Places<'a> {
    pub agents: Vec<Entry>,
    pub workspaces: Vec<WorkspaceEntries<'a>>,
}

impl<'a> Places<'a> {
    pub(super) fn new(snapshot: &'a AppSnapshot) -> Self {
        let agents = Entry::grouped(
            agent_rows(&snapshot.agents, &snapshot.home)
                .into_iter()
                .map(|row| {
                    let ids = row.ids.clone();
                    (row, ids)
                }),
        );
        let workspaces = snapshot
            .workspaces
            .iter()
            .map(|workspace| WorkspaceEntries {
                workspace,
                entries: Entry::grouped(
                    dir_rows(&snapshot.agents, workspace, &snapshot.home)
                        .into_iter()
                        // A workspace with agents offers only those; one without offers the
                        // detected agents' directories.
                        .filter(|row| {
                            workspace.dirs.is_empty() || workspace.dirs.contains(&row.dir)
                        })
                        .map(|row| (row.row, vec![row.dir])),
                ),
            })
            .collect();
        Self { agents, workspaces }
    }

    /// Every listed agent's id: the agents a fixed list keeps when All agents is switched off.
    pub(super) fn every(&self) -> Vec<String> {
        self.agents
            .iter()
            .flat_map(|entry| entry.keys.iter().cloned())
            .collect()
    }

    /// Where `skill` is on: each entry held or not, through the `active` preset and each
    /// workspace.
    pub(super) fn of(&self, skill: &'a Skill, active: &PresetId) -> SkillPlaces<'_> {
        let targets = skill.presets.get(active);
        let agents = self
            .agents
            .iter()
            .map(|entry| Held {
                held: library_targets::held(targets, &entry.keys),
                entry,
            })
            .collect();
        let workspaces = self
            .workspaces
            .iter()
            .map(|place| {
                let targets = place.workspace.skills.get(&skill.id);
                WorkspacePlace {
                    workspace: place.workspace,
                    entries: place
                        .entries
                        .iter()
                        .map(|entry| Held {
                            held: workspace_targets::held(
                                targets,
                                &entry.keys,
                                &place.workspace.dirs,
                            ),
                            entry,
                        })
                        .collect(),
                    targets,
                }
            })
            .collect();
        SkillPlaces {
            skill: &skill.id,
            agents,
            targets,
            every: self.every(),
            workspaces,
        }
    }
}

/// An entry and whether the skill is held there.
pub(super) struct Held<'a> {
    pub entry: &'a Entry,
    pub held: bool,
}

/// One workspace's line for a skill.
pub(super) struct WorkspacePlace<'a> {
    pub workspace: &'a Workspace,
    pub entries: Vec<Held<'a>>,
    /// What the workspace holds the skill for, while it holds it.
    pub targets: Option<&'a WorkspaceTargets>,
}

impl WorkspacePlace<'_> {
    /// The workspace holds the skill, even for no directory it uses.
    pub(super) fn holds(&self) -> bool {
        self.targets.is_some()
    }

    /// The workspace's folder is missing, so its line is inert.
    pub(super) fn missing(&self) -> bool {
        self.workspace.missing
    }

    /// The workspace uses no agent yet, so its line offers the detected agents' directories.
    pub(super) fn uses_none(&self) -> bool {
        self.workspace.dirs.is_empty()
    }

    /// The names of the directories that hold the skill, for the workspace chip's tooltip.
    pub(super) fn held_names(&self) -> String {
        self.entries
            .iter()
            .filter(|held| held.held)
            .map(|held| held.entry.row.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The workspace's targets after toggling `entry`; `None` deactivates the skill there.
    pub(super) fn toggled(&self, entry: &Entry) -> Option<WorkspaceTargets> {
        workspace_targets::toggled(self.targets, &entry.keys, &self.workspace.dirs)
    }
}

/// Where one skill is on: your agents through the active preset, and each workspace.
pub(super) struct SkillPlaces<'a> {
    pub skill: &'a SkillId,
    pub agents: Vec<Held<'a>>,
    /// What the active preset holds the skill for, while it holds it.
    pub targets: Option<&'a Targets>,
    /// Every listed agent's id.
    every: Vec<String>,
    pub workspaces: Vec<WorkspacePlace<'a>>,
}

impl<'a> SkillPlaces<'a> {
    /// The active preset holds the skill for all agents, including ones detected later.
    pub(super) fn all_agents(&self) -> bool {
        self.targets == Some(&Targets::AllAgents)
    }

    /// Your agents' entries that hold the skill, in Agents section order.
    pub(super) fn held_agents(&self) -> impl Iterator<Item = &'a Entry> + '_ {
        self.agents
            .iter()
            .filter(|held| held.held)
            .map(|held| held.entry)
    }

    /// The workspaces that hold the skill, in sidebar order.
    pub(super) fn holding_workspaces(&self) -> impl Iterator<Item = &WorkspacePlace<'a>> {
        self.workspaces.iter().filter(|place| place.holds())
    }

    /// On right now: held for one of your agents, or by a workspace.
    pub(super) fn is_active(&self) -> bool {
        self.agents.iter().any(|held| held.held) || self.workspaces.iter().any(|w| w.holds())
    }

    /// The active preset's targets after the All agents switch: all agents, or exactly the
    /// agents listed now.
    pub(super) fn switched(&self, on: bool) -> Targets {
        if on {
            Targets::AllAgents
        } else {
            Targets::Agents(self.every.iter().cloned().collect())
        }
    }

    /// The active preset's targets after toggling `entry`. An empty list removes the skill from
    /// the preset.
    pub(super) fn toggled(&self, entry: &Entry) -> Targets {
        library_targets::toggled(self.targets, &entry.keys, &self.every)
    }
}

/// Whether `skill` is on right now: the `active` preset holds it for one of your agents, or a
/// workspace holds it, even one whose folder is missing.
pub(super) fn is_active(places: &Places<'_>, skill: &Skill, active: &PresetId) -> bool {
    let held = skill.presets.get(active).is_some_and(|targets| {
        places
            .agents
            .iter()
            .any(|entry| library_targets::held(Some(targets), &entry.keys))
    });
    held || places
        .workspaces
        .iter()
        .any(|place| place.workspace.skills.contains_key(&skill.id))
}

/// Which skills the Active segment keeps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum ActiveFilter {
    #[default]
    All,
    Active,
    Inactive,
}

/// Library's filters, which last the session: All with Updates off at launch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct LibraryFilter {
    pub active: ActiveFilter,
    /// Only skills whose source has a newer version.
    pub updates: bool,
}

impl LibraryFilter {
    /// A segment or Updates narrows the list.
    pub(super) fn narrows(self) -> bool {
        self.active != ActiveFilter::All || self.updates
    }
}

/// What Library lists: the folder tab, the search, and the filters.
#[derive(Clone, Copy)]
pub(super) struct Query<'a> {
    pub folder: Option<&'a FolderId>,
    /// The search, lowercase and trimmed.
    pub search: &'a str,
    pub filter: LibraryFilter,
    /// A skill listed even when the filters would leave it out: the one whose popover is open.
    pub keep: Option<&'a SkillId>,
}

/// The skills Library shows, by name.
pub(super) fn library_skills<'a>(
    snapshot: &'a AppSnapshot,
    places: &Places<'_>,
    active: &PresetId,
    query: Query<'_>,
) -> Vec<&'a Skill> {
    let Some(library) = &snapshot.library else {
        return Vec::new();
    };
    let mut shown: Vec<&Skill> = library
        .library
        .skills
        .iter()
        .filter(|skill| {
            scoped(skill, library, query)
                && (query.keep == Some(&skill.id)
                    || filtered(snapshot, places, active, skill, query.filter))
        })
        .collect();
    shown.sort_by_cached_key(|skill| name_order(skill));
    shown
}

/// How many skills each filter would show, with the folder tab, the search, and the other
/// filters applied: a segment's count applies Updates, and Updates' count applies the segment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct FilterCounts {
    pub all: usize,
    pub active: usize,
    pub inactive: usize,
    pub updates: usize,
}

pub(super) fn filter_counts(
    snapshot: &AppSnapshot,
    places: &Places<'_>,
    active: &PresetId,
    query: Query<'_>,
) -> FilterCounts {
    let Some(library) = &snapshot.library else {
        return FilterCounts::default();
    };
    let count = |filter: LibraryFilter| {
        library
            .library
            .skills
            .iter()
            .filter(|skill| {
                scoped(skill, library, query) && filtered(snapshot, places, active, skill, filter)
            })
            .count()
    };
    let updates = query.filter.updates;
    let segment = |active| LibraryFilter { active, updates };
    FilterCounts {
        all: count(segment(ActiveFilter::All)),
        active: count(segment(ActiveFilter::Active)),
        inactive: count(segment(ActiveFilter::Inactive)),
        updates: count(LibraryFilter {
            updates: true,
            ..query.filter
        }),
    }
}

/// The folder tab and the search keep `skill`.
fn scoped(skill: &Skill, library: &LibrarySnapshot, query: Query<'_>) -> bool {
    query
        .folder
        .is_none_or(|id| skill.folder_id.as_ref() == Some(id))
        && matches(skill, library, query.search)
}

/// The segment and Updates keep `skill`.
fn filtered(
    snapshot: &AppSnapshot,
    places: &Places<'_>,
    active: &PresetId,
    skill: &Skill,
    filter: LibraryFilter,
) -> bool {
    let segment = match filter.active {
        ActiveFilter::All => true,
        ActiveFilter::Active => is_active(places, skill, active),
        ActiveFilter::Inactive => !is_active(places, skill, active),
    };
    segment && (!filter.updates || has_update(snapshot, &skill.id))
}

/// Alphabetical by slug ignoring case, then by slug so equal names keep a stable order.
pub(super) fn name_order(skill: &Skill) -> (String, String) {
    let slug = skill.id.as_str();
    (slug.to_lowercase(), slug.to_owned())
}

/// Search matches the slug or description, ignoring case; `query` is lowercase.
pub(super) fn matches(skill: &Skill, library: &LibrarySnapshot, query: &str) -> bool {
    query.is_empty()
        || skill.id.as_str().to_lowercase().contains(query)
        || library
            .details
            .get(&skill.id)
            .is_some_and(|details| details.description.to_lowercase().contains(query))
}

#[cfg(test)]
pub(super) mod fixture;
#[cfg(test)]
mod tests;
