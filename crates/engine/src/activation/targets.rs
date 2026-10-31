//! Where the active preset holds each skill: the global skill directories of the detected agents
//! its entries name, each counted once however many agents share it.

use super::store::Pair;
use skillbinder_proto::{AgentOption, Library};
use std::collections::BTreeSet;

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
