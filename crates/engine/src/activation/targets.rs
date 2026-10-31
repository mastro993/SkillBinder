//! Where each skill is held: the global skill directories of the detected agents the active
//! preset's entries name, and the workspace skill directories each workspace holds it in, each
//! directory counted once however many agents share it.

use super::store::Pair;
use skillbinder_proto::{AgentOption, Library, SkillId, Workspace, WorkspaceId};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

/// Where skills are held, and the workspace folders whose directories those are.
#[derive(Default)]
pub(super) struct Places {
    /// Every directory and slug where a skill is held.
    pub desired: BTreeSet<Pair>,
    /// The workspace holding each pair only workspaces hold, the first one when several do; a
    /// pair the active preset holds is your agents'.
    pub holders: BTreeMap<Pair, WorkspaceId>,
    /// Each workspace skill directory, resolved, and the folder of the workspace using it.
    pub folders: BTreeMap<String, PathBuf>,
    /// The folders of workspaces that are missing. Every directory inside one is left as it is, so
    /// a copy never recreates the folder, and no record there is forgotten: the path a directory
    /// resolved to through links inside the folder cannot be resolved again while it is gone.
    pub missing: BTreeSet<PathBuf>,
}
impl Places {
    /// Whether `directory` lies inside a missing workspace's folder.
    pub fn paused(&self, directory: &str) -> bool {
        Path::new(directory)
            .ancestors()
            .any(|ancestor| self.missing.contains(ancestor))
    }

    /// Who a held pair's record is held for: a workspace, or `None` for your agents.
    pub fn owner(&self, pair: &Pair) -> Option<WorkspaceId> {
        self.holders.get(pair).cloned()
    }

    /// The folder `directory`'s links are resolved inside: its workspace's, or else home.
    pub fn boundary<'a>(&'a self, directory: &str, home: &'a Path) -> &'a Path {
        self.folders.get(directory).map_or(home, PathBuf::as_path)
    }
}

/// Every global skill directory and slug the active preset holds a skill in. An agent detected
/// later is included as soon as it is detected; an agent no longer detected holds nothing.
pub(super) fn desired(library: &Library, agents: &[AgentOption]) -> BTreeSet<Pair> {
    let detected: Vec<_> = agents
        .iter()
        .filter(|agent| agent.detected)
        .filter_map(|agent| Some((agent.id.as_str(), agent.resolved_skills_dir.as_deref()?)))
        .collect();
    let mut pairs = BTreeSet::new();
    for skill in &library.skills {
        let Some(targets) = skill.presets.get(&library.active_preset) else {
            continue;
        };
        for (agent, directory) in &detected {
            if targets.includes(agent) {
                pairs.insert(((*directory).to_owned(), skill.id.as_str().to_owned()));
            }
        }
    }
    pairs
}

/// Every workspace skill directory `workspaces` use, resolved inside each workspace's folder, and
/// slug they hold there. A directory whose links lead out of its folder holds nothing, as the scan
/// skips it.
pub(super) fn workspace_places(workspaces: &[Workspace]) -> Places {
    let mut places = Places::default();
    for workspace in workspaces {
        if workspace.missing {
            places.missing.insert(PathBuf::from(&workspace.path));
        }
        for (dir, path) in &workspace.paths {
            if !workspace.missing {
                for (skill, targets) in &workspace.skills {
                    if targets.includes(dir) {
                        let pair = (path.clone(), skill.0.clone());
                        places
                            .holders
                            .entry(pair.clone())
                            .or_insert_with(|| workspace.id.clone());
                        places.desired.insert(pair);
                    }
                }
            }
            places
                .folders
                .entry(path.clone())
                .or_insert_with(|| PathBuf::from(&workspace.path));
        }
    }
    places
}

/// The skills `workspace` holds in the workspace skill directory of a detected agent whose global
/// skill directory `global` holds them too, so that agent may load them twice.
pub(crate) fn overlaps(
    workspace: &Workspace,
    global: &BTreeSet<Pair>,
    agents: &[AgentOption],
) -> BTreeSet<SkillId> {
    let reading: Vec<(&str, &str)> = agents
        .iter()
        .filter(|agent| agent.detected && workspace.dirs.contains(&agent.workspace_skills_dir))
        .filter_map(|agent| {
            let own = agent.resolved_skills_dir.as_deref()?;
            Some((agent.workspace_skills_dir.as_str(), own))
        })
        .collect();
    workspace
        .skills
        .iter()
        .filter(|(skill, targets)| {
            reading.iter().any(|(dir, own)| {
                targets.includes(dir) && global.contains(&((*own).to_owned(), skill.0.clone()))
            })
        })
        .map(|(skill, _)| skill.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use skillbinder_proto::{PresetId, Skill, SkillId, Targets};

    fn agent(id: &str, directory: &str, detected: bool) -> AgentOption {
        AgentOption {
            id: id.to_owned(),
            display_name: id.to_owned(),
            workspace_skills_dir: ".agents/skills".to_owned(),
            global_skills_dir: Some(directory.to_owned()),
            resolved_skills_dir: Some(directory.to_owned()),
            detected,
        }
    }

    fn skill(slug: &str, presets: &[(&str, Targets)]) -> Skill {
        Skill {
            id: SkillId(slug.to_owned()),
            presets: presets
                .iter()
                .map(|(id, targets)| (PresetId((*id).to_owned()), targets.clone()))
                .collect(),
            folder_id: None,
            digest: String::new(),
            file_count: 0,
            total_bytes: 0,
            imported_at: None,
            source: None,
            recorded: String::new(),
            drift: None,
        }
    }

    #[test]
    fn shared_directories_count_once_and_only_detected_agents_hold_skills() {
        let agents = [
            agent("codex", "/h/.agents/skills", true),
            agent("cursor", "/h/.agents/skills", true),
            agent("claude-code", "/h/.claude/skills", true),
            agent("gone", "/h/.gone/skills", false),
        ];
        let only_claude = Targets::Agents(["claude-code".to_owned(), "gone".to_owned()].into());
        let library = Library {
            presets: Vec::new(),
            active_preset: PresetId("work".to_owned()),
            folders: Vec::new(),
            skills: vec![
                skill("all", &[("work", Targets::AllAgents)]),
                skill("one", &[("work", only_claude)]),
                skill("elsewhere", &[("home", Targets::AllAgents)]),
            ],
        };
        let pair = |dir: &str, slug: &str| (dir.to_owned(), slug.to_owned());
        assert_eq!(
            desired(&library, &agents),
            [
                pair("/h/.agents/skills", "all"),
                pair("/h/.claude/skills", "all"),
                pair("/h/.claude/skills", "one"),
            ]
            .into()
        );
    }
}
