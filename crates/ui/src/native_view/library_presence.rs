//! Library skill presence: the detected agents whose global skill directory held a copy of each
//! library skill at the last scan. Workspace copies are not part of presence.

use std::collections::BTreeMap;

use super::Route;
use super::agent_skills::{
    UNIVERSAL, agent_name, detected_agents, found_where, read_globally, split_universal,
    universal_dir,
};
use skillbinder_proto::{AgentOption, ScanSnapshot, SkillId};

/// Which Agents row holds a copy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Reader {
    /// The universal agents together, by display name.
    Universal(Vec<String>),
    /// One other detected agent, by registry id.
    Agent(String),
}

/// An Agents row whose global skill directory holds a copy of a library skill.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Holder {
    pub reader: Reader,
    /// The name the Agents section shows.
    pub name: String,
    /// The global skill directory holding the copy.
    pub dir: String,
}

impl Holder {
    /// The page the chip opens: the agent's, or Universal's.
    pub(super) fn route(&self) -> Route {
        match &self.reader {
            Reader::Universal(_) => Route::Universal,
            Reader::Agent(id) => Route::Agent(id.clone()),
        }
    }
}

/// The Agents rows holding a copy of each library skill, in Agents section order: Universal
/// first, then the other detected agents alphabetically. A copy counts as on the agent page:
/// the row's agents read its location as their global skill directory. Library skills no row
/// holds are left out.
pub(super) fn presence(
    scan: &ScanSnapshot,
    agents: &[AgentOption],
    home: &str,
) -> BTreeMap<SkillId, Vec<Holder>> {
    let (universal, others) = split_universal(detected_agents(agents), home);
    let shared = (!universal.is_empty()).then(|| {
        let names = universal.iter().map(|agent| agent.display_name.clone());
        let holder = Holder {
            reader: Reader::Universal(names.collect()),
            name: UNIVERSAL.to_owned(),
            dir: universal_dir(home).display().to_string(),
        };
        (holder, universal)
    });
    let rows = shared
        .into_iter()
        .chain(others.into_iter().filter_map(|agent| {
            let holder = Holder {
                reader: Reader::Agent(agent.id.clone()),
                name: agent_name(agent, home),
                dir: agent.global_skills_dir.clone()?,
            };
            Some((holder, vec![agent]))
        }));
    let mut found: BTreeMap<SkillId, Vec<Holder>> = BTreeMap::new();
    for (holder, readers) in rows {
        let held = found_where(&scan.candidates, |location| {
            read_globally(location, &readers)
        });
        for skill in held.into_iter().filter_map(|skill| skill.skill) {
            let holders = found.entry(skill).or_default();
            if holders.last() != Some(&holder) {
                holders.push(holder.clone());
            }
        }
    }
    found
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
            resolved_skills_dir: Some(global.to_owned()),
            detected: true,
        }
    }

    /// A location the scan found, read globally by `global` and by the `workspaces`.
    fn location(global: &[&str], workspaces: Vec<WorkspaceId>) -> CandidateLocation {
        CandidateLocation {
            display_path: String::new(),
            readers: Vec::new(),
            agents: Vec::new(),
            global_agents: global.iter().map(|id| (*id).to_owned()).collect(),
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

    fn names(holders: &[Holder]) -> Vec<&str> {
        holders.iter().map(|holder| holder.name.as_str()).collect()
    }

    #[test]
    fn presence_lists_the_agents_rows_whose_global_directory_holds_a_copy() {
        let mut missing = agent("cursor", "Cursor", "/home/.cursor/skills");
        missing.detected = false;
        let agents = [
            agent("amp", "Amp", "/home/.agents/skills"),
            agent("claude-code", "Claude Code", "/home/.claude/skills"),
            agent("codex", "Codex", "/home/.agents/skills"),
            missing,
            agent("zed", "Zed", "/home/.zed/skills"),
        ];
        let (both, workspace_only) = (SkillId::new(), SkillId::new());
        let scan = ScanSnapshot {
            candidates: vec![
                found(
                    Some(both.clone()),
                    vec![
                        location(&["claude-code"], Vec::new()),
                        location(&["amp", "codex"], Vec::new()),
                        location(&["cursor"], Vec::new()),
                        location(&[], vec![WorkspaceId::new()]),
                    ],
                ),
                found(
                    Some(workspace_only.clone()),
                    vec![location(&[], vec![WorkspaceId::new()])],
                ),
                found(None, vec![location(&["zed"], Vec::new())]),
            ],
            error: None,
        };
        let presence = presence(&scan, &agents, "/home");
        assert_eq!(
            presence.keys().collect::<Vec<_>>(),
            [&both],
            "skills outside the library or only in workspaces have none"
        );
        let holders = &presence[&both];
        assert_eq!(
            names(holders),
            ["Universal", "Claude Code"],
            "Universal first; undetected agents and workspace copies are left out"
        );
        assert_eq!(
            holders[0].reader,
            Reader::Universal(vec!["Amp".to_owned(), "Codex".to_owned()])
        );
        assert_eq!(holders[0].route(), Route::Universal);
        assert_eq!(holders[0].dir, "/home/.agents/skills");
        assert_eq!(holders[1].route(), Route::Agent("claude-code".to_owned()));
        assert_eq!(holders[1].dir, "/home/.claude/skills");
    }

    #[test]
    fn a_copy_linked_into_several_global_directories_shows_every_agent() {
        let agents = [
            agent("claude-code", "Claude Code", "/home/.claude/skills"),
            agent("codex", "Codex", "/home/.agents/skills"),
        ];
        let skill = SkillId::new();
        let linked = location(&["claude-code", "codex"], Vec::new());
        let scan = ScanSnapshot {
            candidates: vec![found(Some(skill.clone()), vec![linked])],
            error: None,
        };
        let holders = &presence(&scan, &agents, "/home")[&skill];
        assert_eq!(
            names(holders),
            ["Universal", "Claude Code"],
            "one universal agent is still the Universal row"
        );
        assert_eq!(
            holders[0].reader,
            Reader::Universal(vec!["Codex".to_owned()])
        );
    }
}
