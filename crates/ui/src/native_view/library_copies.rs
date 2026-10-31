//! Where library skills are applied: the rows whose skill directory holds a skill's managed copy,
//! the Agents rows for your agents or a workspace's directory rows, and the copies that hold a
//! skill back. Universal stands for the universal agents together, which share one directory.

use std::collections::BTreeMap;
use std::path::Path;

use super::Route;
use super::agent_skills::{UNIVERSAL, agent_name, detected_agents, split_universal};
use skillbinder_proto::{AgentOption, CopyState, ManagedCopy, SkillId};

/// Which Agents row a directory belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Reader {
    /// The universal agents together, by display name.
    Universal(Vec<String>),
    /// One other detected agent, by registry id.
    Agent(String),
}

/// An Agents row and the global skill directory its agents read, or a workspace skill directory
/// and the agents that read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AgentRow {
    pub reader: Reader,
    /// The name the Agents section shows.
    pub name: String,
    /// The registry identities of the row's agents.
    pub ids: Vec<String>,
    /// The skill directory as the app shows it.
    pub dir: String,
    /// The same directory with links resolved, as copies name it.
    pub resolved: String,
}

impl AgentRow {
    /// The page the row opens: the agent's, or Universal's.
    pub(super) fn route(&self) -> Route {
        match &self.reader {
            Reader::Universal(_) => Route::Universal,
            Reader::Agent(id) => Route::Agent(id.clone()),
        }
    }
}

/// The Agents rows in section order: Universal first, then the other detected agents
/// alphabetically.
pub(super) fn agent_rows(agents: &[AgentOption], home: &str) -> Vec<AgentRow> {
    let (universal, others) = split_universal(detected_agents(agents), home);
    let directory = |agent: &AgentOption| {
        let dir = agent.global_skills_dir.clone()?;
        let resolved = agent
            .resolved_skills_dir
            .clone()
            .unwrap_or_else(|| dir.clone());
        Some((dir, resolved))
    };
    let mut rows = Vec::new();
    if let Some((dir, resolved)) = universal.first().and_then(|agent| directory(agent)) {
        rows.push(AgentRow {
            reader: Reader::Universal(universal.iter().map(|a| a.display_name.clone()).collect()),
            name: UNIVERSAL.to_owned(),
            ids: universal.iter().map(|agent| agent.id.clone()).collect(),
            dir,
            resolved,
        });
    }
    rows.extend(others.into_iter().filter_map(|agent| {
        let (dir, resolved) = directory(agent)?;
        Some(AgentRow {
            reader: Reader::Agent(agent.id.clone()),
            name: agent_name(agent, home),
            ids: vec![agent.id.clone()],
            dir,
            resolved,
        })
    }));
    rows
}

/// How one library skill's copies stand.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Placement {
    /// The Agents rows whose directory holds the skill's applied copy, in section order.
    pub applied: Vec<AgentRow>,
    /// The skill's copies that are not applied, or ask the user, or were not removed.
    pub unsettled: Vec<ManagedCopy>,
}

/// Each library skill's placement, for the skills with any copy. A copy counts for every row
/// whose agents read its directory, such as one linked into several agents' directories.
pub(super) fn placements(
    copies: &[ManagedCopy],
    rows: &[AgentRow],
) -> BTreeMap<SkillId, Placement> {
    let mut placements = BTreeMap::<SkillId, Placement>::new();
    for copy in copies {
        let placement = placements.entry(copy.skill.clone()).or_default();
        if copy.state != CopyState::Applied {
            placement.unsettled.push(copy.clone());
            continue;
        }
        placement.applied.extend(
            rows.iter()
                .filter(|row| row.resolved == copy.directory)
                .filter(|row| !placement.applied.contains(row))
                .cloned()
                .collect::<Vec<_>>(),
        );
    }
    for placement in placements.values_mut() {
        placement
            .applied
            .sort_by_key(|row| rows.iter().position(|other| other == row));
    }
    placements
}

/// A directory as the user reads it, with the home folder shortened to `~`.
pub(super) fn shown_dir(dir: &str, home: &str) -> String {
    Path::new(dir)
        .strip_prefix(home)
        .map_or_else(|_| dir.to_owned(), |inner| format!("~/{}", inner.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(id: &str, name: &str, global: &str) -> AgentOption {
        AgentOption {
            id: id.to_owned(),
            display_name: name.to_owned(),
            workspace_skills_dir: ".agents/skills".to_owned(),
            global_skills_dir: Some(global.to_owned()),
            resolved_skills_dir: Some(global.to_owned()),
            detected: true,
        }
    }

    fn copy(skill: &SkillId, directory: &str, state: CopyState) -> ManagedCopy {
        ManagedCopy {
            skill: skill.clone(),
            directory: directory.to_owned(),
            managed: state == CopyState::Applied,
            state,
        }
    }

    #[test]
    fn rows_list_universal_first_and_only_detected_agents() {
        let mut missing = agent("cursor", "Cursor", "/home/.cursor/skills");
        missing.detected = false;
        let agents = [
            agent("amp", "Amp", "/home/.agents/skills"),
            agent("claude-code", "Claude Code", "/home/.claude/skills"),
            agent("codex", "Codex", "/home/.agents/skills"),
            missing,
        ];
        let rows = agent_rows(&agents, "/home");
        let names: Vec<_> = rows.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names, ["Universal", "Claude Code"]);
        assert_eq!(
            rows[0].reader,
            Reader::Universal(vec!["Amp".to_owned(), "Codex".to_owned()])
        );
        assert_eq!(rows[0].ids, ["amp", "codex"]);
        assert_eq!(rows[0].route(), Route::Universal);
        assert_eq!(rows[1].route(), Route::Agent("claude-code".to_owned()));
    }

    #[test]
    fn applied_copies_name_their_rows_and_the_rest_are_unsettled() {
        let mut claude = agent("claude-code", "Claude Code", "/home/.claude/skills");
        // Claude Code's directory links to the universal one, so one copy serves both rows.
        claude.resolved_skills_dir = Some("/home/.agents/skills".to_owned());
        let agents = [
            agent("codex", "Codex", "/home/.agents/skills"),
            claude,
            agent("zed", "Zed", "/home/.zed/skills"),
        ];
        let rows = agent_rows(&agents, "/home");
        let review = SkillId("review".into());
        let failed = CopyState::NotApplied("No access.".into());
        let copies = [
            copy(&review, "/home/.zed/skills", failed.clone()),
            copy(&review, "/home/.agents/skills", CopyState::Applied),
        ];
        let placement = &placements(&copies, &rows)[&review];
        let names: Vec<_> = placement
            .applied
            .iter()
            .map(|row| row.name.as_str())
            .collect();
        assert_eq!(names, ["Universal", "Claude Code"]);
        assert_eq!(
            placement.unsettled,
            [copy(&review, "/home/.zed/skills", failed)]
        );
        assert_eq!(shown_dir("/home/.zed/skills", "/home"), "~/.zed/skills");
    }
}
