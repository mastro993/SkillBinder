use crate::SkillId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

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

/// A library skill active in one of your agents' global skill directories, or a copy SkillBinder
/// still manages there, and how that directory stands.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ManagedCopy {
    /// The skill's slug, which is also the copy's directory name.
    pub skill: SkillId,
    /// The global skill directory, as [`crate::AgentOption::resolved_skills_dir`] shows it.
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

/// How a directory stands against what the active preset holds there.
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
}
