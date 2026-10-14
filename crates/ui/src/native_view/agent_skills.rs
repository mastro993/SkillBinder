//! Detected agents and the skills found in skill directories: a detected agent's found skills
//! are the skill folders the last scan found in its global skill directory, which agents sharing
//! that directory share. The universal agents, which all read `~/.agents/skills`, are
//! listed together as Universal.

use std::path::{Path, PathBuf};

use skillbinder_proto::{AgentOption, Candidate, CandidateLocation, SkillId, ValidationStatus};

/// The name of the universal agents together.
pub(super) const UNIVERSAL: &str = "Universal";

/// What one found skill folder holds relative to the library.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum FoundStatus {
    /// Its content matches a library skill, so it is installed.
    Installed,
    /// A skill the library does not hold.
    NotInstalled,
    /// A skill the library does not hold whose payload fails validation.
    Invalid,
    /// A skill the library does not hold whose payload cannot safely be copied.
    Blocked,
}

impl FoundStatus {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Installed => "Installed",
            Self::NotInstalled => "Not installed",
            Self::Invalid => "Invalid",
            Self::Blocked => "Blocked",
        }
    }
}

/// One skill folder found in a skill directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FoundSkill {
    pub slug: String,
    pub status: FoundStatus,
    /// The library skill with the same content.
    pub skill: Option<SkillId>,
}

/// Agents whose global skill directory exists, alphabetically: the snapshot lists agents by
/// lowercase display name.
pub(super) fn detected_agents(agents: &[AgentOption]) -> Vec<&AgentOption> {
    agents.iter().filter(|agent| agent.detected).collect()
}

/// The global skill directory the universal agents share: `~/.agents/skills`.
pub(super) fn universal_dir(home: &str) -> PathBuf {
    Path::new(home).join(".agents").join("skills")
}

/// Whether one of `agents` reads `location` as its global skill directory, through any link.
pub(super) fn read_globally(location: &CandidateLocation, agents: &[&AgentOption]) -> bool {
    agents
        .iter()
        .any(|agent| location.global_agents.contains(&agent.id))
}

/// Splits detected agents into the universal agents and the rest, keeping their order.
pub(super) fn split_universal<'a>(
    agents: Vec<&'a AgentOption>,
    home: &str,
) -> (Vec<&'a AgentOption>, Vec<&'a AgentOption>) {
    let dir = universal_dir(home);
    agents.into_iter().partition(|agent| {
        agent
            .global_skills_dir
            .as_deref()
            .is_some_and(|own| Path::new(own) == dir)
    })
}

/// An agent's name in the Agents section. The registry's own Universal agent reads another
/// directory, so its name adds that directory to tell it apart from the Universal row.
pub(super) fn agent_name(agent: &AgentOption, home: &str) -> String {
    match &agent.global_skills_dir {
        Some(dir) if agent.display_name == UNIVERSAL => {
            let shown = Path::new(dir)
                .strip_prefix(home)
                .map_or_else(|_| dir.clone(), |inner| format!("~/{}", inner.display()));
            format!("{UNIVERSAL} ({shown})")
        }
        _ => agent.display_name.clone(),
    }
}

/// Skill folders the scan found at the locations `holds` accepts, by slug.
pub(super) fn found_where(
    candidates: &[Candidate],
    holds: impl Fn(&CandidateLocation) -> bool,
) -> Vec<FoundSkill> {
    let mut skills: Vec<_> = candidates
        .iter()
        .flat_map(|candidate| {
            let status = match (&candidate.skill_id, candidate.validation) {
                (Some(_), _) => FoundStatus::Installed,
                (None, ValidationStatus::Invalid) => FoundStatus::Invalid,
                (None, ValidationStatus::Blocked) => FoundStatus::Blocked,
                (None, ValidationStatus::Valid | ValidationStatus::Warning) => {
                    FoundStatus::NotInstalled
                }
            };
            candidate
                .locations
                .iter()
                .filter(|location| holds(location))
                .map(move |_| FoundSkill {
                    slug: candidate.slug.clone(),
                    status,
                    skill: candidate.skill_id.clone(),
                })
        })
        .collect();
    skills.sort_by(|a, b| (&a.slug, a.status).cmp(&(&b.slug, b.status)));
    skills
}

#[cfg(test)]
mod tests {
    use super::*;
    use skillbinder_proto::CandidateId;

    fn agent(id: &str) -> AgentOption {
        AgentOption {
            id: id.to_owned(),
            display_name: id.to_owned(),
            workspace_skills_dir: ".agents/skills".to_owned(),
            global_skills_dir: Some("/home/.agents/skills".to_owned()),
            resolved_skills_dir: Some("/home/.agents/skills".to_owned()),
            detected: true,
        }
    }

    fn candidate(slug: &str, skill: Option<SkillId>, validation: ValidationStatus) -> Candidate {
        Candidate {
            id: CandidateId::new(),
            skill_id: skill,
            slug: slug.to_owned(),
            description: String::new(),
            locations: vec![CandidateLocation {
                display_path: format!("/home/.claude/skills/{slug}"),
                readers: Vec::new(),
                agents: Vec::new(),
                global_agents: vec!["claude-code".to_owned()],
                workspaces: Vec::new(),
            }],
            validation,
            messages: Vec::new(),
            total_bytes: 0,
            file_count: 0,
        }
    }

    #[test]
    fn detected_agents_are_the_agents_with_a_global_directory_in_snapshot_order() {
        let mut missing = agent("cursor");
        missing.detected = false;
        let agents = [agent("cline"), missing, agent("zed")];
        let ids: Vec<_> = detected_agents(&agents)
            .into_iter()
            .map(|agent| agent.id.as_str())
            .collect();
        assert_eq!(ids, ["cline", "zed"]);
    }

    #[test]
    fn universal_agents_are_the_agents_reading_the_home_agents_directory() {
        let mut own = agent("claude-code");
        own.global_skills_dir = Some("/home/.claude/skills".to_owned());
        let mut registry_universal = agent("universal");
        registry_universal.display_name = UNIVERSAL.to_owned();
        registry_universal.global_skills_dir = Some("/home/.config/agents/skills".to_owned());
        let agents = [agent("cline"), own, agent("zed"), registry_universal];
        let (universal, rest) = split_universal(agents.iter().collect(), "/home");
        let ids = |agents: &[&AgentOption]| agents.iter().map(|a| a.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(&universal), ["cline", "zed"]);
        assert_eq!(ids(&rest), ["claude-code", "universal"]);
        assert_eq!(agent_name(rest[0], "/home"), "claude-code");
        assert_eq!(
            agent_name(rest[1], "/home"),
            "Universal (~/.config/agents/skills)"
        );
    }

    #[test]
    fn found_skills_list_every_folder_an_agent_reads_globally() {
        let kept = SkillId::new();
        let mut elsewhere = candidate("elsewhere", None, ValidationStatus::Valid);
        elsewhere.locations[0].global_agents.clear();
        let mut linked = candidate("linked", None, ValidationStatus::Valid);
        linked.locations[0].global_agents = vec!["codex".to_owned()];
        let candidates = [
            candidate("kept", Some(kept.clone()), ValidationStatus::Valid),
            candidate("broken", None, ValidationStatus::Invalid),
            candidate("found", None, ValidationStatus::Warning),
            elsewhere,
            linked,
        ];
        let mut claude = agent("claude-code");
        claude.global_skills_dir = Some("/home/.claude/skills".to_owned());
        let skills = found_where(&candidates, |location| read_globally(location, &[&claude]));
        let listed: Vec<_> = skills
            .iter()
            .map(|skill| (skill.slug.as_str(), skill.status))
            .collect();
        assert_eq!(
            listed,
            [
                ("broken", FoundStatus::Invalid),
                ("found", FoundStatus::NotInstalled),
                ("kept", FoundStatus::Installed),
            ]
        );
        assert_eq!(skills[2].skill, Some(kept));
    }
}
