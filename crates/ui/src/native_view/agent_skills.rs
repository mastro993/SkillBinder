//! Installed agents and the skills found in skill directories: an installed agent's installed
//! skills are the skill folders the last scan found in its global skill directory, which agents
//! sharing that directory share. The universal agents, which all read `~/.agents/skills`, are
//! listed together as Universal.

use std::path::{Path, PathBuf};

use skillbinder_proto::{AgentOption, Candidate, SkillId, ValidationStatus};

/// The name of the universal agents together.
pub(super) const UNIVERSAL: &str = "Universal";

/// What one found skill folder holds relative to the library.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum InstalledStatus {
    /// Its content matches a library skill.
    InLibrary,
    /// A skill the library does not hold.
    NotInLibrary,
    /// A skill the library does not hold whose payload fails validation.
    Invalid,
    /// A skill the library does not hold whose payload cannot safely be copied.
    Blocked,
}

impl InstalledStatus {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::InLibrary => "In library",
            Self::NotInLibrary => "Not in library",
            Self::Invalid => "Invalid",
            Self::Blocked => "Blocked",
        }
    }
}

/// One skill folder found in a skill directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct InstalledSkill {
    pub slug: String,
    pub status: InstalledStatus,
    /// The library skill with the same content.
    pub skill: Option<SkillId>,
}

/// Agents whose global skill directory exists, alphabetically: the snapshot lists agents by
/// lowercase display name.
pub(super) fn installed_agents(agents: &[AgentOption]) -> Vec<&AgentOption> {
    agents.iter().filter(|agent| agent.installed).collect()
}

/// The global skill directory the universal agents share: `~/.agents/skills`.
pub(super) fn universal_dir(home: &str) -> PathBuf {
    Path::new(home).join(".agents").join("skills")
}

/// Splits installed agents into the universal agents and the rest, keeping their order.
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

/// Skill folders the scan found inside any of `dirs`, by slug.
pub(super) fn found_in(dirs: &[PathBuf], candidates: &[Candidate]) -> Vec<InstalledSkill> {
    let mut skills: Vec<_> = candidates
        .iter()
        .flat_map(|candidate| {
            let status = match (&candidate.skill_id, candidate.validation) {
                (Some(_), _) => InstalledStatus::InLibrary,
                (None, ValidationStatus::Invalid) => InstalledStatus::Invalid,
                (None, ValidationStatus::Blocked) => InstalledStatus::Blocked,
                (None, ValidationStatus::Valid | ValidationStatus::Warning) => {
                    InstalledStatus::NotInLibrary
                }
            };
            candidate
                .locations
                .iter()
                .filter(|location| {
                    let path = Path::new(&location.display_path);
                    dirs.iter().any(|dir| path.starts_with(dir))
                })
                .map(move |_| InstalledSkill {
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
    use skillbinder_proto::{CandidateId, CandidateLocation};

    fn agent(id: &str) -> AgentOption {
        AgentOption {
            id: id.to_owned(),
            display_name: id.to_owned(),
            workspace_skills_dir: ".agents/skills".to_owned(),
            global_skills_dir: Some("/home/.agents/skills".to_owned()),
            installed: true,
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
            }],
            validation,
            messages: Vec::new(),
            total_bytes: 0,
            file_count: 0,
        }
    }

    #[test]
    fn installed_agents_are_the_agents_with_a_global_directory_in_snapshot_order() {
        let mut missing = agent("cursor");
        missing.installed = false;
        let agents = [agent("cline"), missing, agent("zed")];
        let ids: Vec<_> = installed_agents(&agents)
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
    fn found_skills_list_every_folder_in_the_directories() {
        let kept = SkillId::new();
        let mut elsewhere = candidate("elsewhere", None, ValidationStatus::Valid);
        elsewhere.locations[0].display_path = "/work/.claude/skills/elsewhere".to_owned();
        let candidates = [
            candidate("kept", Some(kept.clone()), ValidationStatus::Valid),
            candidate("broken", None, ValidationStatus::Invalid),
            candidate("found", None, ValidationStatus::Warning),
            elsewhere,
        ];
        let skills = found_in(&[PathBuf::from("/home/.claude/skills")], &candidates);
        let listed: Vec<_> = skills
            .iter()
            .map(|skill| (skill.slug.as_str(), skill.status))
            .collect();
        assert_eq!(
            listed,
            [
                ("broken", InstalledStatus::Invalid),
                ("found", InstalledStatus::NotInLibrary),
                ("kept", InstalledStatus::InLibrary),
            ]
        );
        assert_eq!(skills[2].skill, Some(kept));
    }

    #[test]
    fn a_sibling_directory_sharing_a_prefix_is_not_inside() {
        let mut skill = candidate("skill", None, ValidationStatus::Valid);
        skill.locations[0].display_path = "/home/.claude/skills-old/skill".to_owned();
        assert!(found_in(&[PathBuf::from("/home/.claude/skills")], &[skill]).is_empty());
    }
}
