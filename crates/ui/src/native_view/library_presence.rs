//! Library skill presence: the workspaces where the last scan found each library skill, and the
//! agents reading each copy.

use super::agent_skills::{installed_agents, split_universal};
use super::sidebar::SYSTEM;
use skillbinder_proto::{AgentOption, ScanSnapshot, SkillId, Workspace, WorkspaceRef};
use std::collections::{BTreeMap, BTreeSet};

/// An agent, or the universal agents together, reading a workspace's copy of a skill.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Reader {
    /// Two or more universal agents, by display name.
    Universal(Vec<String>),
    /// One agent by registry id.
    Agent { id: String, name: String },
}

/// One workspace holding a library skill.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Place {
    pub workspace: WorkspaceRef,
    /// "System" or the workspace's name.
    pub name: String,
    /// The folder the workspace reads under.
    pub path: String,
    /// Universal first, then agents by display name.
    pub readers: Vec<Reader>,
}

/// Where the last scan found each library skill, System first and then workspaces in sidebar
/// order. Workspaces removed since the scan are left out.
pub(super) fn presence(
    scan: &ScanSnapshot,
    agents: &[AgentOption],
    workspaces: &[Workspace],
    home: &str,
) -> BTreeMap<SkillId, Vec<Place>> {
    let mut found: BTreeMap<&SkillId, BTreeMap<&WorkspaceRef, BTreeSet<&str>>> = BTreeMap::new();
    for candidate in &scan.candidates {
        let Some(skill) = &candidate.skill_id else {
            continue;
        };
        let places = found.entry(skill).or_default();
        for location in &candidate.locations {
            for workspace in &location.workspaces {
                places
                    .entry(workspace)
                    .or_default()
                    .extend(location.agents.iter().map(String::as_str));
            }
        }
    }
    // System's universal agents share `~/.agents/skills`; a workspace's share its `.agents/skills`.
    let (system, _) = split_universal(installed_agents(agents), home);
    let system: BTreeSet<&str> = system.iter().map(|agent| agent.id.as_str()).collect();
    let shared: BTreeSet<&str> = agents
        .iter()
        .filter(|agent| agent.workspace_skills_dir == UNIVERSAL_WORKSPACE_DIR)
        .map(|agent| agent.id.as_str())
        .collect();
    let name = |id: &str| {
        agents
            .iter()
            .find(|agent| agent.id == id)
            .map_or_else(|| id.to_owned(), |agent| agent.display_name.clone())
    };
    found
        .into_iter()
        .map(|(skill, places)| {
            let places = places
                .into_iter()
                .filter_map(|(workspace, ids)| {
                    let (name_of, path) = describe(workspace, workspaces, home)?;
                    Some(Place {
                        workspace: workspace.clone(),
                        name: name_of,
                        path,
                        readers: readers(&ids, universal(workspace, &system, &shared), &name),
                    })
                })
                .collect::<Vec<_>>();
            (skill.clone(), sorted(places, workspaces))
        })
        .collect()
}

/// The directory, relative to an added workspace, that its universal agents share.
const UNIVERSAL_WORKSPACE_DIR: &str = ".agents/skills";

fn universal<'a>(
    workspace: &WorkspaceRef,
    system: &'a BTreeSet<&'a str>,
    shared: &'a BTreeSet<&'a str>,
) -> &'a BTreeSet<&'a str> {
    match workspace {
        WorkspaceRef::System => system,
        WorkspaceRef::Added(_) => shared,
    }
}

/// A workspace's name and folder, or `None` when it was removed since the scan.
fn describe(
    workspace: &WorkspaceRef,
    workspaces: &[Workspace],
    home: &str,
) -> Option<(String, String)> {
    match workspace {
        WorkspaceRef::System => Some((SYSTEM.to_owned(), home.to_owned())),
        WorkspaceRef::Added(id) => workspaces
            .iter()
            .find(|workspace| &workspace.id == id)
            .map(|workspace| (workspace.name.clone(), workspace.path.clone())),
    }
}

/// System first, then workspaces by their sidebar position.
fn sorted(mut places: Vec<Place>, workspaces: &[Workspace]) -> Vec<Place> {
    places.sort_by_key(|place| match &place.workspace {
        WorkspaceRef::System => None,
        WorkspaceRef::Added(id) => workspaces
            .iter()
            .find(|workspace| &workspace.id == id)
            .map(|workspace| workspace.position),
    });
    places
}

/// Two or more universal agents read the copy as one Universal reader.
fn readers(
    ids: &BTreeSet<&str>,
    universal: &BTreeSet<&str>,
    name: &impl Fn(&str) -> String,
) -> Vec<Reader> {
    let shared: Vec<&str> = ids.intersection(universal).copied().collect();
    let grouped = shared.len() > 1;
    let by_name = |ids: Vec<&str>| {
        let mut named: Vec<(String, String)> = ids
            .into_iter()
            .map(|id| (name(id), id.to_owned()))
            .collect();
        named.sort_by_key(|(name, _)| name.to_lowercase());
        named
    };
    let mut readers = Vec::new();
    if grouped {
        let names = by_name(shared).into_iter().map(|(name, _)| name).collect();
        readers.push(Reader::Universal(names));
    }
    let own = ids
        .iter()
        .copied()
        .filter(|id| !grouped || !universal.contains(id))
        .collect();
    readers.extend(
        by_name(own)
            .into_iter()
            .map(|(name, id)| Reader::Agent { id, name }),
    );
    readers
}

#[cfg(test)]
mod tests {
    use super::*;
    use skillbinder_proto::{
        Candidate, CandidateId, CandidateLocation, ValidationStatus, WorkspaceId,
    };

    fn agent(id: &str, name: &str, global: &str) -> AgentOption {
        AgentOption {
            id: id.to_owned(),
            display_name: name.to_owned(),
            workspace_skills_dir: ".agents/skills".to_owned(),
            global_skills_dir: Some(global.to_owned()),
            installed: true,
        }
    }

    fn location(agents: &[&str], workspaces: Vec<WorkspaceRef>) -> CandidateLocation {
        CandidateLocation {
            display_path: String::new(),
            readers: Vec::new(),
            agents: agents.iter().map(|id| (*id).to_owned()).collect(),
            workspaces,
        }
    }

    fn found(skill: Option<SkillId>, locations: Vec<CandidateLocation>) -> Candidate {
        Candidate {
            id: CandidateId::new(),
            skill_id: skill,
            slug: "pdf".to_owned(),
            description: String::new(),
            locations,
            validation: ValidationStatus::Valid,
            messages: Vec::new(),
            total_bytes: 0,
            file_count: 0,
        }
    }

    fn workspace(name: &str, position: u32) -> Workspace {
        Workspace {
            id: WorkspaceId::new(),
            name: name.to_owned(),
            path: format!("/work/{name}"),
            position,
            missing: false,
        }
    }

    #[test]
    fn presence_maps_each_workspace_to_its_readers_with_universal_grouped() {
        let mut claude = agent("claude-code", "Claude Code", "/home/.claude/skills");
        claude.workspace_skills_dir = ".claude/skills".to_owned();
        let agents = [
            claude,
            agent("codex", "Codex", "/home/.agents/skills"),
            agent("amp", "Amp", "/home/.agents/skills"),
            agent("cursor", "Cursor", "/home/.cursor/skills"),
        ];
        let (second, first, removed) = (
            workspace("web", 1),
            workspace("api", 0),
            workspace("old", 2),
        );
        let [system, api, web, gone] = [
            WorkspaceRef::System,
            WorkspaceRef::Added(first.id.clone()),
            WorkspaceRef::Added(second.id.clone()),
            WorkspaceRef::Added(removed.id.clone()),
        ];
        let skill = SkillId::new();
        let scan = ScanSnapshot {
            candidates: vec![
                found(
                    Some(skill.clone()),
                    vec![
                        location(&["amp", "codex"], vec![system.clone()]),
                        location(&["claude-code"], vec![system.clone(), web.clone()]),
                        location(&["codex"], vec![api.clone(), gone]),
                        location(&["codex", "cursor"], vec![web.clone()]),
                    ],
                ),
                found(None, vec![location(&["amp"], vec![system.clone()])]),
            ],
            error: None,
        };
        let presence = presence(&scan, &agents, &[second, first], "/home");
        assert_eq!(presence.len(), 1, "skills outside the library have none");
        let places = &presence[&skill];
        let names: Vec<_> = places.iter().map(|place| place.name.as_str()).collect();
        assert_eq!(names, ["System", "api", "web"]);
        assert_eq!(
            places[0].readers,
            [
                Reader::Universal(vec!["Amp".to_owned(), "Codex".to_owned()]),
                Reader::Agent {
                    id: "claude-code".to_owned(),
                    name: "Claude Code".to_owned()
                },
            ]
        );
        assert_eq!(
            places[1].readers,
            [Reader::Agent {
                id: "codex".to_owned(),
                name: "Codex".to_owned()
            }],
            "one universal agent keeps its own name"
        );
        assert_eq!(
            places[2].readers,
            [
                Reader::Universal(vec!["Codex".to_owned(), "Cursor".to_owned()]),
                Reader::Agent {
                    id: "claude-code".to_owned(),
                    name: "Claude Code".to_owned()
                },
            ],
            "a workspace's universal agents are the ones sharing its .agents/skills"
        );
        assert_eq!(places[0].path, "/home");
        assert_eq!(places[2].path, "/work/web");
    }
}
