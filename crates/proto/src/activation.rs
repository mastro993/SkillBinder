use crate::{SkillId, WorkspaceId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Which of your agents a preset holds a skill for. The target of activation is a global skill
/// directory, so agents that share one, such as the universal agents, count once.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum Targets {
    /// Every detected agent's global skill directory, including agents detected later.
    #[default]
    AllAgents,
    /// Exactly these agents, by registry identity; the list never gains agents. An empty list
    /// holds the skill for no agent, so the engine removes the skill from the preset instead.
    Agents(BTreeSet<String>),
}
impl Targets {
    /// Whether the skill is held for `agent`.
    pub fn includes(&self, agent: &str) -> bool {
        match self {
            Self::AllAgents => true,
            Self::Agents(agents) => agents.contains(agent),
        }
    }
}

/// Which of a workspace's agents it holds a skill for. A workspace's agents are the workspace skill
/// directories it uses, so the target is a directory, relative to the workspace folder, such as
/// `.claude/skills`, and every agent that reads it counts once.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkspaceTargets {
    /// Every skill directory the workspace uses, including ones added later.
    AllAgents,
    /// Exactly these directories; the list never gains directories.
    Dirs(BTreeSet<String>),
}
impl WorkspaceTargets {
    /// Whether the skill is held in workspace skill directory `dir`, one the workspace uses.
    pub fn includes(&self, dir: &str) -> bool {
        match self {
            Self::AllAgents => true,
            Self::Dirs(dirs) => dirs.contains(dir),
        }
    }
}

/// Library skills added to one place at once, each for the agents chosen for it. A skill the place
/// already holds keeps what it holds and gains them; nothing is removed.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AddSkills {
    /// To the active preset, for your agents: each skill's agents by registry identity.
    YourAgents {
        /// Each skill and the agents it gains, by registry identity.
        skills: BTreeMap<SkillId, BTreeSet<String>>,
    },
    /// To workspace `id`: each skill's targets, after `dirs`, the detected agents' workspace skill
    /// directories it does not use yet, join its agents.
    Workspace {
        /// The workspace the skills join.
        id: WorkspaceId,
        /// Directories the workspace's agents gain, each a detected agent's.
        dirs: BTreeSet<String>,
        /// Each skill and the targets it gains there.
        skills: BTreeMap<SkillId, WorkspaceTargets>,
    },
}

/// A library skill active in a global or workspace skill directory, or a copy SkillBinder still
/// manages there, and how that directory stands.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ManagedCopy {
    /// The skill's slug, which is also the copy's directory name.
    pub skill: SkillId,
    /// The skill directory with links resolved: a global one as
    /// [`crate::AgentOption::resolved_skills_dir`] shows it, or a workspace's as
    /// [`crate::Workspace::paths`] does.
    pub directory: String,
    /// Whether the copy is applied, and what holds it back when it is not.
    pub state: CopyState,
    /// Whether the directory holds a copy SkillBinder manages, such as an applied or edited one,
    /// or one it could not rewrite or remove yet.
    pub managed: bool,
}
impl ManagedCopy {
    /// Whether the user decides what happens to the directory's copy before anything changes.
    pub fn asks(&self) -> bool {
        matches!(self.state, CopyState::Conflict | CopyState::Edited { .. })
    }
}

/// How a directory stands against what the active preset or a workspace holds there.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CopyState {
    /// The skill is active there, and the directory holds its managed copy.
    Applied,
    /// The skill is active there, but its copy could not be written; a retry or the next scan
    /// tries again. A safe sentence explaining why.
    NotApplied(String),
    /// The skill is active there, but the directory holds a copy of the slug with other content,
    /// which is replaced only once the user confirms.
    Conflict,
    /// Like [`Self::Conflict`], but the user chose to keep their copy, so it is not asked again.
    Skipped,
    /// The managed copy was edited in place, so it is never overwritten or removed without
    /// asking. `active` tells whether the skill is still active there, so Replace overwrites it,
    /// or no longer is, so Replace removes it.
    Edited {
        /// The skill is still active in the directory.
        active: bool,
    },
    /// The skill is no longer active there, but its managed copy could not be removed; a retry or
    /// the next scan tries again. A safe sentence explaining why.
    NotRemoved(String),
}

impl CopyState {
    /// Whether the copy needs the user once a change has run: one that asks, one that could not
    /// be written, and one that could not be removed. An applied copy and one the user chose to
    /// keep do not.
    pub fn needs_attention(&self) -> bool {
        match self {
            Self::Conflict | Self::Edited { .. } | Self::NotApplied(_) | Self::NotRemoved(_) => {
                true
            }
            Self::Applied | Self::Skipped => false,
        }
    }
}

/// How far a change that writes and removes copies has got: of the (directory, skill) pairs it
/// brings in line, how many it has done.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActivationProgress {
    /// Pairs brought in line so far.
    pub done: usize,
    /// Pairs the change brings in line, as far as it has planned them.
    pub total: usize,
    /// The pair the change is on while it runs; `None` once it is over.
    pub step: Option<ActivationStep>,
}

/// The (directory, skill) pair a change is bringing in line.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActivationStep {
    /// The skill's slug.
    pub skill: SkillId,
    /// The skill directory with links resolved, as [`ManagedCopy::directory`] names it.
    pub directory: String,
    /// The skill is no longer active there, so its copy goes.
    pub removing: bool,
}
impl ActivationProgress {
    /// The share done, rounded to the nearest percent and capped at 100; nothing planned is 0.
    pub fn percent(&self) -> u8 {
        if self.total == 0 {
            return 0;
        }
        let done = self.done.min(self.total) as u128;
        let total = self.total as u128;
        // Round half up, as the progress strip's design rounds.
        u8::try_from((done * 200 + total) / (total * 2)).unwrap_or(100)
    }
}

/// The user's answer for a copy that [`ManagedCopy::asks`], or for a skipped one.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CopyChoice {
    /// Write the library's content over a conflicting or edited copy, or remove an edited copy
    /// of a skill no longer active there. The copy is managed afterwards.
    Replace,
    /// Leave the directory's copy as it is. SkillBinder stops managing it, and a skill still
    /// active there stays not applied until the copy is gone or matches the library.
    Keep,
}

/// The skills that switching the active preset adds to your agents and removes from them.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PresetSwitch {
    /// Skills the new active preset holds that the old one does not, by slug.
    pub adds: Vec<SkillId>,
    /// Skills the old active preset holds that the new one does not, by slug.
    pub removes: Vec<SkillId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_round_trip_and_include_their_agents() -> serde_json::Result<()> {
        let specific = Targets::Agents(["claude-code".to_owned()].into());
        let stored = serde_json::to_string(&[Targets::AllAgents, specific.clone()])?;
        assert_eq!(stored, r#"["AllAgents",{"Agents":["claude-code"]}]"#);
        assert_eq!(
            serde_json::from_str::<Vec<Targets>>(&stored)?,
            [Targets::AllAgents, specific.clone()]
        );
        assert!(Targets::AllAgents.includes("codex"));
        assert!(specific.includes("claude-code"));
        assert!(!specific.includes("codex"));
        Ok(())
    }

    #[test]
    fn additions_round_trip() -> serde_json::Result<()> {
        let id = WorkspaceId("acme".to_owned());
        let changes = [
            AddSkills::YourAgents {
                skills: [(SkillId("pdf".into()), ["claude-code".to_owned()].into())].into(),
            },
            AddSkills::Workspace {
                id,
                dirs: [".agents/skills".to_owned()].into(),
                skills: [
                    (SkillId("pdf".into()), WorkspaceTargets::AllAgents),
                    (
                        SkillId("review".into()),
                        WorkspaceTargets::Dirs([".claude/skills".to_owned()].into()),
                    ),
                ]
                .into(),
            },
        ];
        let stored = serde_json::to_string(&changes)?;
        assert_eq!(serde_json::from_str::<Vec<AddSkills>>(&stored)?, changes);
        Ok(())
    }

    #[test]
    fn percent_rounds_to_the_nearest_and_never_passes_100() {
        let progress = |done, total| ActivationProgress {
            done,
            total,
            ..ActivationProgress::default()
        };
        assert_eq!(progress(0, 0).percent(), 0);
        assert_eq!(progress(1, 3).percent(), 33);
        assert_eq!(progress(2, 3).percent(), 67);
        assert_eq!(progress(3, 3).percent(), 100);
        assert_eq!(progress(9, 20).percent(), 45);
        assert_eq!(progress(1, 8).percent(), 13);
        assert_eq!(progress(5, 3).percent(), 100);
    }

    #[test]
    fn only_copies_left_for_the_user_need_attention() {
        let reason = || "No access.".to_owned();
        let states = [
            (CopyState::Applied, false),
            (CopyState::Skipped, false),
            (CopyState::Conflict, true),
            (CopyState::Edited { active: true }, true),
            (CopyState::Edited { active: false }, true),
            (CopyState::NotApplied(reason()), true),
            (CopyState::NotRemoved(reason()), true),
        ];
        for (state, attention) in states {
            assert_eq!(state.needs_attention(), attention, "{state:?}");
        }
    }
}
