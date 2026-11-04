//! Where each skill is held: the global skill directories of the detected agents the active
//! preset's entries name, and the workspace skill directories each workspace holds it in, each
//! directory counted once however many agents share it.

use super::store::{Holder, Pair};
use skillbinder_proto::{AgentOption, Library, Overlap, SameSlug, SkillId, Workspace, WorkspaceId};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

/// Where skills are held, and the workspace folders whose directories those are.
#[derive(Default)]
pub(super) struct Places {
    /// Every directory and slug where a skill is held.
    pub desired: BTreeSet<Pair>,
    /// Who holds each pair only workspaces hold, the first workspace and directory when several
    /// do, missing or not; a pair the active preset holds is your agents'.
    pub holders: BTreeMap<Pair, Holder>,
    /// Where each workspace's skill directories lead now, by workspace and directory.
    pub reached: BTreeMap<(WorkspaceId, String), String>,
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
    pub fn owner(&self, pair: &Pair) -> Option<Holder> {
        self.holders.get(pair).cloned()
    }

    /// Whether `holder`'s directory still leads to `directory`.
    pub fn leads(&self, holder: &Holder, directory: &str) -> bool {
        self.reached
            .get(&(holder.workspace.clone(), holder.dir.clone()))
            .is_some_and(|path| path == directory)
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
            places
                .reached
                .insert((workspace.id.clone(), dir.clone()), path.clone());
            for (skill, targets) in &workspace.skills {
                if targets.includes(dir) {
                    // A missing workspace still holds its skills, but nothing is written for it.
                    let pair = (path.clone(), skill.0.clone());
                    places
                        .holders
                        .entry(pair.clone())
                        .or_insert_with(|| Holder {
                            workspace: workspace.id.clone(),
                            dir: dir.clone(),
                        });
                    if !workspace.missing {
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

/// The skills `workspace` holds that a detected agent also holds in a global skill directory it
/// reads, and how that agent treats the two copies.
pub(crate) fn overlaps(
    workspace: &Workspace,
    global: &BTreeSet<Pair>,
    agents: &[AgentOption],
) -> BTreeMap<SkillId, Overlap> {
    let mut overlaps: BTreeMap<SkillId, Overlap> = BTreeMap::new();
    for agent in agents.iter().filter(|agent| agent.detected) {
        let dirs: Vec<&str> = std::iter::once(agent.workspace_skills_dir.as_str())
            .chain(
                agent
                    .other_workspace_dirs
                    .iter()
                    .map(|dir| dir.path.as_str()),
            )
            .filter(|dir| workspace.dirs.contains(*dir))
            .collect();
        let globals: Vec<&str> = agent
            .resolved_skills_dir
            .iter()
            .chain(&agent.other_global_dirs)
            .map(String::as_str)
            .collect();
        let overlap = match agent.same_slug {
            SameSlug::Precedence | SameSlug::One => Overlap::Hidden,
            SameSlug::Both | SameSlug::Unknown => Overlap::Twice,
        };
        for (skill, targets) in &workspace.skills {
            if !dirs.iter().any(|dir| targets.includes(dir)) {
                continue;
            }
            if !globals
                .iter()
                .any(|dir| global.contains(&((*dir).to_owned(), skill.0.clone())))
            {
                continue;
            }
            // One agent that keeps both copies is enough to say so.
            overlaps
                .entry(skill.clone())
                .and_modify(|seen| {
                    if overlap == Overlap::Twice {
                        *seen = Overlap::Twice;
                    }
                })
                .or_insert(overlap);
        }
    }
    overlaps
}

#[cfg(test)]
mod tests {
    use super::*;
    use skillbinder_proto::{
        AgentDir, DirRole, PresetId, Skill, SkillId, Targets, WorkspaceTargets,
    };
    fn agent(id: &str, directory: &str, detected: bool) -> AgentOption {
        AgentOption {
            id: id.to_owned(),
            display_name: id.to_owned(),
            workspace_skills_dir: ".agents/skills".to_owned(),
            other_workspace_dirs: Vec::new(),
            global_skills_dir: Some(directory.to_owned()),
            resolved_skills_dir: Some(directory.to_owned()),
            other_global_dirs: Vec::new(),
            same_slug: SameSlug::Unknown,
            detected,
        }
    }

    /// The workspace holding `slug` in `dir`, used by that directory alone.
    fn workspace(dir: &str, slug: &str) -> Workspace {
        Workspace {
            id: WorkspaceId("w".to_owned()),
            name: "api".to_owned(),
            path: "/home/api".to_owned(),
            position: 0,
            missing: false,
            color: None,
            icon: None,
            image: None,
            dirs: [dir.to_owned()].into(),
            paths: [(dir.to_owned(), format!("/home/api/{dir}"))].into(),
            skills: [(
                SkillId(slug.to_owned()),
                WorkspaceTargets::Dirs([dir.to_owned()].into()),
            )]
            .into(),
            overlaps: BTreeMap::new(),
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

    #[test]
    fn overlaps_through_a_compatibility_directory_may_load_the_skill_twice() {
        // Cursor reads `.claude/skills` in a workspace and `~/.agents/skills` globally, both as
        // directories named for other agents, and keeps both copies.
        let mut cursor = agent("cursor", "/h/.cursor/skills", true);
        cursor.other_workspace_dirs = [AgentDir {
            path: ".claude/skills".to_owned(),
            role: DirRole::Compatibility,
        }]
        .into();
        cursor.other_global_dirs = ["/h/.agents/skills".to_owned()].into();
        cursor.same_slug = SameSlug::Both;
        let global = [("/h/.agents/skills".to_owned(), "review".to_owned())].into();
        let overlaps = overlaps(&workspace(".claude/skills", "review"), &global, &[cursor]);
        assert_eq!(
            overlaps.get(&SkillId("review".to_owned())),
            Some(&Overlap::Twice)
        );
    }

    #[test]
    fn a_copy_an_agent_hides_is_marked_hidden_and_one_kept_twice_wins() {
        let hidden = {
            let mut gemini = agent("gemini-cli", "/h/.gemini/skills", true);
            gemini.workspace_skills_dir = ".gemini/skills".to_owned();
            gemini.same_slug = SameSlug::Precedence;
            gemini
        };
        let twice = {
            let mut codex = agent("codex", "/h/.agents/skills", true);
            codex.other_workspace_dirs = [AgentDir {
                path: ".gemini/skills".to_owned(),
                role: DirRole::Compatibility,
            }]
            .into();
            codex.same_slug = SameSlug::Both;
            codex
        };
        let global = [
            ("/h/.gemini/skills".to_owned(), "lint".to_owned()),
            ("/h/.agents/skills".to_owned(), "lint".to_owned()),
        ]
        .into();
        let workspace = workspace(".gemini/skills", "lint");
        assert_eq!(
            overlaps(&workspace, &global, std::slice::from_ref(&hidden))
                .get(&SkillId("lint".to_owned())),
            Some(&Overlap::Hidden)
        );
        assert_eq!(
            overlaps(&workspace, &global, &[hidden, twice]).get(&SkillId("lint".to_owned())),
            Some(&Overlap::Twice)
        );
    }
}
