//! Which of a workspace's agents it holds a library skill for. A workspace's page lists the
//! folders its directories resolve to in each row's agents menu; Library's popover and the Add
//! skill picker also offer detected agents' directories it does not use yet. Each directory is
//! one row, naming the agents that read it, since every one of them loads what it holds.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use super::agent_skills::{UNIVERSAL, universal_dir};
use super::library_copies::{AgentRow, Reader, shown_dir};
use super::library_targets::held_label;
use super::snapshot_query::Entry;
use super::{Effect, NativeView};
use gpui_kit::Context;
use skillbinder_proto::{AgentOption, DirRole, SkillId, Workspace, WorkspaceId, WorkspaceTargets};

/// The workspace skill directory the universal agents, and many others, read.
const UNIVERSAL_DIR: &str = ".agents/skills";

/// One workspace skill directory as the agents menu, Library's popover, and the picker show it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DirRow {
    /// The directory relative to the workspace folder, such as `.claude/skills`.
    pub dir: String,
    /// How the row reads: Universal or its agents, and where the directory is.
    pub row: AgentRow,
}

/// The directories the workspace uses, then those of detected agents it does not use yet, each
/// once: `.agents/skills` first, then by name. A detected agent offers its registry directory and
/// the directories the reviewed overlay names for it alone, such as `.kimi-code/skills`; a
/// directory only other agents read is offered once the workspace uses it. A row names the
/// detected agents that read it, or every registry agent that does when none is detected;
/// `.agents/skills` is Universal while a universal agent reads it, so its avatar opens Universal's
/// page.
pub(super) fn dir_rows<'a>(
    agents: &'a [AgentOption],
    workspace: &'a Workspace,
    home: &str,
) -> Vec<DirRow> {
    let detected: Vec<&'a AgentOption> = agents.iter().filter(|agent| agent.detected).collect();
    let dirs: BTreeSet<&'a str> = workspace
        .dirs
        .iter()
        .map(String::as_str)
        .chain(detected.iter().flat_map(|agent| offered(agent)))
        .collect();
    let mut rows: Vec<DirRow> = dirs
        .into_iter()
        .filter_map(|dir| {
            let readers: Vec<&AgentOption> =
                agents.iter().filter(|agent| reads(agent, dir)).collect();
            let shown: Vec<&AgentOption> = if readers.iter().any(|agent| agent.detected) {
                readers.into_iter().filter(|agent| agent.detected).collect()
            } else {
                readers
            };
            let names: Vec<String> = shown.iter().map(|a| a.display_name.clone()).collect();
            let universal = shown.iter().any(|agent| {
                agent
                    .global_skills_dir
                    .as_deref()
                    .is_some_and(|own| Path::new(own) == universal_dir(home))
            });
            let (reader, name) = if dir == UNIVERSAL_DIR && universal {
                (Reader::Universal(names), UNIVERSAL.to_owned())
            } else {
                (Reader::Agent(shown.first()?.id.clone()), names.join(", "))
            };
            // Copies name a directory the workspace uses by its resolved path.
            let path = workspace
                .paths
                .get(dir)
                .cloned()
                .unwrap_or_else(|| joined(&workspace.path, dir));
            Some(DirRow {
                dir: dir.to_owned(),
                row: AgentRow {
                    reader,
                    name,
                    ids: shown.iter().map(|agent| agent.id.clone()).collect(),
                    dir: shown_dir(&path, home),
                    resolved: path,
                },
            })
        })
        .collect();
    // An agent with two directories of its own, such as Cline's `.cline/skills` and legacy
    // `.clinerules/skills`, would give two rows the same name, so those name their directory.
    let mut names = BTreeSet::new();
    let duplicated: BTreeSet<String> = rows
        .iter()
        .filter(|row| !names.insert(row.row.name.as_str()))
        .map(|row| row.row.name.clone())
        .collect();
    for row in &mut rows {
        // The registry's directory keeps the plain name.
        let registry = agents
            .iter()
            .any(|agent| agent.workspace_skills_dir == row.dir);
        if duplicated.contains(&row.row.name) && !registry {
            row.row.name = format!("{} ({})", row.row.name, row.dir);
        }
    }
    rows.sort_by_cached_key(|row| (row.dir != UNIVERSAL_DIR, row.row.name.to_lowercase()));
    rows
}

/// The directories a row offers for a detected agent: the registry's, then the ones the overlay
/// records as named for it alone.
fn offered(agent: &AgentOption) -> impl Iterator<Item = &str> {
    std::iter::once(agent.workspace_skills_dir.as_str()).chain(
        agent
            .other_workspace_dirs
            .iter()
            .filter(|dir| dir.role == DirRole::Own)
            .map(|dir| dir.path.as_str()),
    )
}

/// Whether `agent` reads `dir`: as the registry's directory, or as one the overlay adds, whether
/// named for it or another agent's.
fn reads(agent: &AgentOption, dir: &str) -> bool {
    agent.workspace_skills_dir == dir
        || agent
            .other_workspace_dirs
            .iter()
            .any(|other| other.path == dir)
}

/// `dir`, relative with slash separators, inside `folder`, as the platform spells paths.
fn joined(folder: &str, dir: &str) -> String {
    dir.split('/')
        .fold(PathBuf::from(folder), |path, part| path.join(part))
        .display()
        .to_string()
}

impl NativeView {
    /// Holds `skill` in workspace `id` for `targets`, or deactivates it there, saving at once.
    pub(super) fn set_workspace_skill(
        &mut self,
        id: WorkspaceId,
        skill: SkillId,
        targets: Option<WorkspaceTargets>,
        cx: &mut Context<Self>,
    ) {
        let client = self.client.clone();
        self.run(
            async move {
                client
                    .set_workspace_skill(id, skill, targets)
                    .await
                    .map(|()| Effect::None)
            },
            cx,
        );
    }
}

/// The folders a workspace's agents menu lists: the directories it uses, `.agents/skills` first,
/// then by name, those that resolve to one folder sharing an entry, which keeps the first one's
/// name and logo.
pub(super) fn folders(agents: &[AgentOption], workspace: &Workspace, home: &str) -> Vec<Entry> {
    Entry::grouped(
        dir_rows(agents, workspace, home)
            .into_iter()
            .filter(|row| workspace.dirs.contains(&row.dir))
            .map(|row| (row.row, vec![row.dir])),
    )
}

/// The `folders` that hold the skill for `targets`, among the directories the workspace `uses`.
pub(super) fn held_folders<'a>(
    targets: Option<&WorkspaceTargets>,
    folders: &'a [Entry],
    uses: &BTreeSet<String>,
) -> Vec<&'a Entry> {
    folders
        .iter()
        .filter(|folder| held(targets, &folder.keys, uses))
        .collect()
}

/// The targets after the All agents item: all agents from a specific list, or from all agents
/// exactly the directories held now, every one the workspace `uses`.
pub(super) fn all_agents_toggled(
    targets: Option<&WorkspaceTargets>,
    uses: &BTreeSet<String>,
) -> WorkspaceTargets {
    match targets {
        Some(WorkspaceTargets::AllAgents) => WorkspaceTargets::Dirs(uses.clone()),
        _ => WorkspaceTargets::AllAgents,
    }
}

/// The agents button's label: "All agents", the one `held` folder's name, how many it holds, or
/// "No agents".
pub(super) fn targets_label(targets: Option<&WorkspaceTargets>, held: &[&Entry]) -> String {
    if matches!(targets, Some(WorkspaceTargets::AllAgents)) {
        return "All agents".to_owned();
    }
    let named: Vec<_> = held.iter().map(|folder| folder.row.name.as_str()).collect();
    held_label(&named)
}

/// Whether `targets` hold the skill in any of `group`, among the directories the workspace
/// `uses`.
pub(super) fn held(
    targets: Option<&WorkspaceTargets>,
    group: &[String],
    uses: &BTreeSet<String>,
) -> bool {
    group.iter().any(|dir| match targets {
        None => false,
        Some(WorkspaceTargets::AllAgents) => uses.contains(dir),
        Some(WorkspaceTargets::Dirs(dirs)) => dirs.contains(dir),
    })
}

/// The targets after choosing a row reading `group`: its directories leave when the skill is held
/// in any of them and join otherwise. Leaving all agents keeps the workspace's other directories
/// as a specific list, and so does joining a directory the workspace does not use yet, which then
/// becomes one of its agents. No directory left deactivates the skill.
pub(super) fn toggled(
    targets: Option<&WorkspaceTargets>,
    group: &[String],
    uses: &BTreeSet<String>,
) -> Option<WorkspaceTargets> {
    let mut dirs = match targets {
        None => BTreeSet::new(),
        Some(WorkspaceTargets::AllAgents) => uses.clone(),
        Some(WorkspaceTargets::Dirs(dirs)) => dirs.clone(),
    };
    if held(targets, group, uses) {
        for dir in group {
            dirs.remove(dir);
        }
    } else {
        dirs.extend(group.iter().cloned());
    }
    (!dirs.is_empty()).then_some(WorkspaceTargets::Dirs(dirs))
}

#[cfg(test)]
mod tests;
