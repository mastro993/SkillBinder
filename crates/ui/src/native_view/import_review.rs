//! How an import's review names places: where a copy it replaces is, and who reads a skill
//! directory.

use super::library_copies::{agent_rows, shown_dir};
use super::workspace_targets::dir_rows;
use skillbinder_proto::{AgentOption, Workspace};

/// How the review names skill directory `directory`, whose copy an import replaces: the Agents
/// rows whose agents read it, or a workspace's name with the agents that read that workspace
/// directory, or else the directory with home shortened to `~`.
pub(crate) fn replaced_in(
    directory: &str,
    agents: &[AgentOption],
    workspaces: &[Workspace],
    home: &str,
) -> String {
    match readers(directory, agents, workspaces, home) {
        Readers::Global(names) | Readers::Unknown(names) => names,
        Readers::Workspace(workspace, names) => format!("{workspace} ({names})"),
    }
}

/// How the app names skill directory `directory` as a place: the Agents rows whose agents read
/// it, such as `Claude Code`, or a workspace's name and the agents that read that workspace
/// directory, such as `acme-web · Universal`, or else the directory with home shortened to `~`.
pub(super) fn place_name(
    directory: &str,
    agents: &[AgentOption],
    workspaces: &[Workspace],
    home: &str,
) -> String {
    match readers(directory, agents, workspaces, home) {
        Readers::Global(names) | Readers::Unknown(names) => names,
        Readers::Workspace(workspace, names) => format!("{workspace} · {names}"),
    }
}

/// Who reads a skill directory.
enum Readers {
    /// The Agents rows whose agents read it, by name.
    Global(String),
    /// A workspace's name, and the names of its directory rows that read it.
    Workspace(String, String),
    /// No one the app knows: the directory with home shortened to `~`.
    Unknown(String),
}

fn readers(
    directory: &str,
    agents: &[AgentOption],
    workspaces: &[Workspace],
    home: &str,
) -> Readers {
    let names = |rows: Vec<String>| (!rows.is_empty()).then(|| rows.join(", "));
    let global = agent_rows(agents, home)
        .into_iter()
        .filter(|row| row.resolved == directory)
        .map(|row| row.name)
        .collect();
    if let Some(global) = names(global) {
        return Readers::Global(global);
    }
    workspaces
        .iter()
        .find_map(|workspace| {
            let rows = dir_rows(agents, workspace, home)
                .into_iter()
                .filter(|row| row.row.resolved == directory)
                .map(|row| row.row.name)
                .collect();
            Some(Readers::Workspace(workspace.name.clone(), names(rows)?))
        })
        .unwrap_or_else(|| Readers::Unknown(shown_dir(directory, home)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use skillbinder_proto::{SameSlug, WorkspaceId};
    use std::collections::{BTreeMap, BTreeSet};

    fn agent(id: &str, name: &str, global: &str, workspace: &str) -> AgentOption {
        AgentOption {
            id: id.to_owned(),
            display_name: name.to_owned(),
            workspace_skills_dir: workspace.to_owned(),
            other_workspace_dirs: Vec::new(),
            global_skills_dir: Some(global.to_owned()),
            resolved_skills_dir: Some(global.to_owned()),
            other_global_dirs: Vec::new(),
            same_slug: SameSlug::Unknown,
            detected: true,
            chosen: true,
            asked: true,
        }
    }

    #[test]
    fn replacements_name_the_agents_or_the_workspace_that_read_the_directory() {
        let agents = [
            agent("codex", "Codex", "/h/.agents/skills", ".agents/skills"),
            agent("cursor", "Cursor", "/h/.cursor/skills", ".cursor/skills"),
            agent(
                "claude-code",
                "Claude Code",
                "/h/.claude/skills",
                ".claude/skills",
            ),
        ];
        let workspace = Workspace {
            id: WorkspaceId("api".to_owned()),
            name: "api".to_owned(),
            path: "/w/api".to_owned(),
            position: 0,
            missing: false,
            color: None,
            icon: None,
            image: None,
            dirs: BTreeSet::from([".claude/skills".to_owned()]),
            paths: BTreeMap::from([(
                ".claude/skills".to_owned(),
                "/w/api/.claude/skills".to_owned(),
            )]),
            skills: BTreeMap::new(),
            overlaps: BTreeMap::new(),
        };
        let workspaces = [workspace];
        let named = |directory| replaced_in(directory, &agents, &workspaces, "/h");
        assert_eq!(named("/h/.cursor/skills"), "Cursor");
        assert_eq!(named("/h/.agents/skills"), "Universal");
        assert_eq!(named("/w/api/.claude/skills"), "api (Claude Code)");
        assert_eq!(named("/h/.other/skills"), "~/.other/skills");
        let place = |directory| place_name(directory, &agents, &workspaces, "/h");
        assert_eq!(place("/h/.cursor/skills"), "Cursor");
        assert_eq!(place("/w/api/.claude/skills"), "api · Claude Code");
    }
}
