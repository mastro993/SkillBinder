//! Installed agents and their installed skills: the skill folders Global's last scan found in an
//! agent's global skill directory, which agents sharing that directory share.

use std::path::Path;

use skillbinder_proto::{
    AgentOption, CopyChange, Deployment, DeploymentTarget, Importable, SkillId, ValidationStatus,
};

/// What one installed skill folder holds relative to the library.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum InstalledStatus {
    /// A deployed copy that still matches its deployment.
    InLibrary,
    /// A deployed copy whose bytes changed.
    Changed,
    /// A deployed copy that is gone.
    Missing,
    /// A skill the library does not hold.
    Importable,
    /// A skill the library does not hold whose payload fails validation.
    Invalid,
    /// A skill the library does not hold whose payload cannot safely be copied.
    Blocked,
}

impl InstalledStatus {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::InLibrary => "In library",
            Self::Changed => "Changed",
            Self::Missing => "Missing",
            Self::Importable => "Importable",
            Self::Invalid => "Invalid",
            Self::Blocked => "Blocked",
        }
    }
}

/// One skill folder in an agent's global skill directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct InstalledSkill {
    pub slug: String,
    pub status: InstalledStatus,
    /// The library skill a deployed copy belongs to.
    pub skill: Option<SkillId>,
}

/// Global's agents, alphabetically: the snapshot lists agents by lowercase display name.
pub(super) fn installed_agents<'a>(
    agents: &'a [AgentOption],
    global_agents: &[String],
) -> Vec<&'a AgentOption> {
    agents
        .iter()
        .filter(|agent| global_agents.contains(&agent.id))
        .collect()
}

/// Skill folders under `dir`, by slug, from Global's deployments and importables.
pub(super) fn installed_skills(
    dir: &Path,
    deployments: &[Deployment],
    importables: &[Importable],
    slug: impl Fn(&SkillId) -> String,
) -> Vec<InstalledSkill> {
    let inside = |path: &String| Path::new(path).starts_with(dir);
    let deployed = deployments
        .iter()
        .filter(|deployment| deployment.target == DeploymentTarget::Global)
        .flat_map(|deployment| {
            deployment
                .paths
                .iter()
                .filter(|path| inside(path))
                .map(|path| InstalledSkill {
                    slug: slug(&deployment.skill_id),
                    status: match deployment.changes.get(path) {
                        None => InstalledStatus::InLibrary,
                        Some(CopyChange::Changed) => InstalledStatus::Changed,
                        Some(CopyChange::Missing) => InstalledStatus::Missing,
                    },
                    skill: Some(deployment.skill_id.clone()),
                })
        });
    let unmatched = importables
        .iter()
        .filter(|importable| importable.target == DeploymentTarget::Global)
        .flat_map(|importable| {
            let status = match importable.validation {
                ValidationStatus::Invalid => InstalledStatus::Invalid,
                ValidationStatus::Blocked => InstalledStatus::Blocked,
                ValidationStatus::Valid | ValidationStatus::Warning => InstalledStatus::Importable,
            };
            importable
                .paths
                .iter()
                .filter(|path| inside(path))
                .map(move |_| InstalledSkill {
                    slug: importable.slug.clone(),
                    status,
                    skill: None,
                })
        });
    let mut skills: Vec<_> = deployed.chain(unmatched).collect();
    skills.sort_by(|a, b| (&a.slug, a.status).cmp(&(&b.slug, b.status)));
    skills
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn agent(id: &str) -> AgentOption {
        AgentOption {
            id: id.to_owned(),
            display_name: id.to_owned(),
            workspace_skills_dir: ".agents/skills".to_owned(),
            global_skills_dir: Some("/home/.agents/skills".to_owned()),
        }
    }

    #[test]
    fn installed_agents_are_globals_agents_in_snapshot_order() {
        let agents = [agent("cline"), agent("cursor"), agent("zed")];
        let global = ["zed".to_owned(), "cline".to_owned()];
        let ids: Vec<_> = installed_agents(&agents, &global)
            .into_iter()
            .map(|agent| agent.id.as_str())
            .collect();
        assert_eq!(ids, ["cline", "zed"]);
    }

    #[test]
    fn installed_skills_list_every_global_folder_in_the_directory() {
        let (kept, changed) = (SkillId::new(), SkillId::new());
        let deployment = |skill: &SkillId, path: &str, change: Option<CopyChange>| Deployment {
            target: DeploymentTarget::Global,
            skill_id: skill.clone(),
            digest: "digest".to_owned(),
            paths: vec![path.to_owned(), "/home/.cursor/skills/other".to_owned()],
            changes: change
                .map(|change| BTreeMap::from([(path.to_owned(), change)]))
                .unwrap_or_default(),
        };
        let importable = |target, slug: &str, validation| Importable {
            target,
            slug: slug.to_owned(),
            digest: "digest".to_owned(),
            validation,
            paths: vec![format!("/home/.claude/skills/{slug}")],
        };
        let deployments = [
            deployment(&kept, "/home/.claude/skills/kept", None),
            deployment(
                &changed,
                "/home/.claude/skills/changed",
                Some(CopyChange::Changed),
            ),
        ];
        let workspace = DeploymentTarget::Workspace(skillbinder_proto::WorkspaceId::new());
        let importables = [
            importable(
                DeploymentTarget::Global,
                "broken",
                ValidationStatus::Invalid,
            ),
            importable(DeploymentTarget::Global, "found", ValidationStatus::Warning),
            importable(workspace, "elsewhere", ValidationStatus::Valid),
        ];
        let slug = |skill: &SkillId| if skill == &kept { "kept" } else { "changed" }.to_owned();
        let skills = installed_skills(
            Path::new("/home/.claude/skills"),
            &deployments,
            &importables,
            slug,
        );
        let listed: Vec<_> = skills
            .iter()
            .map(|skill| (skill.slug.as_str(), skill.status))
            .collect();
        assert_eq!(
            listed,
            [
                ("broken", InstalledStatus::Invalid),
                ("changed", InstalledStatus::Changed),
                ("found", InstalledStatus::Importable),
                ("kept", InstalledStatus::InLibrary),
            ]
        );
        assert_eq!(skills[3].skill, Some(kept));
    }

    #[test]
    fn a_sibling_directory_sharing_a_prefix_is_not_inside() {
        let importable = Importable {
            target: DeploymentTarget::Global,
            slug: "skill".to_owned(),
            digest: "digest".to_owned(),
            validation: ValidationStatus::Valid,
            paths: vec!["/home/.claude/skills-old/skill".to_owned()],
        };
        let skills = installed_skills(
            Path::new("/home/.claude/skills"),
            &[],
            &[importable],
            |skill| skill.to_string(),
        );
        assert!(skills.is_empty());
    }
}
