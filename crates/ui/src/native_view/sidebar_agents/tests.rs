use super::super::agent_page::page_skills;
use super::super::skill_picker::{Place, Resolved};
use super::super::snapshot_query::fixture::{Fixture, agents, preset_id};
use super::*;
use skillbinder_proto::{
    Candidate, CandidateId, CandidateLocation, ScanSnapshot, Targets, ValidationStatus,
};

/// A skill folder found in Claude Code's directory that is not in the library.
fn found(slug: &str) -> Candidate {
    Candidate {
        id: CandidateId::new(),
        key: slug.to_owned(),
        preferred: true,
        matches_source: false,
        modified: None,
        skill_id: None,
        held: None,
        slug: slug.to_owned(),
        description: String::new(),
        locations: vec![CandidateLocation {
            display_path: format!("/home/.claude/skills/{slug}"),
            readers: Vec::new(),
            agents: Vec::new(),
            global_agents: vec!["claude-code".to_owned()],
            workspaces: Vec::new(),
        }],
        validation: ValidationStatus::Valid,
        messages: Vec::new(),
        total_bytes: 0,
        file_count: 0,
    }
}

#[test]
fn each_count_is_the_length_of_its_pages_list() {
    let mut snapshot = Fixture::new()
        .agent("amp", "Amp", ".agents/skills", ".agents/skills", true)
        .agent(
            "claude-code",
            "Claude Code",
            ".claude/skills",
            ".claude/skills",
            true,
        )
        .agent("cursor", "Cursor", ".agents/skills", ".cursor/skills", true)
        .linked(".agents/skills")
        .agent("zed", "Zed", ".zed/skills", ".zed/skills", false)
        .preset("frontend", "Frontend")
        .skill("all", "", None)
        .skill("claude", "", None)
        .skill("amp", "", None)
        .skill("elsewhere", "", None)
        .hold("all", "default", Targets::AllAgents)
        .hold("claude", "default", agents(&["claude-code"]))
        .hold("amp", "default", agents(&["amp"]))
        .hold("elsewhere", "frontend", Targets::AllAgents)
        .build();
    snapshot.scan = Some(ScanSnapshot {
        candidates: vec![found("not-in-library")],
        error: None,
    });
    let preset = preset_id("default");
    let counts: Vec<(String, usize)> = counted_rows(&snapshot, Some(&preset))
        .into_iter()
        .map(|(row, count)| (row.name, count))
        .collect();
    assert_eq!(
        counts,
        [
            ("Universal".to_owned(), 2),
            ("Claude Code".to_owned(), 2),
            ("Cursor".to_owned(), 2),
        ],
        "a found skill not in library never counts"
    );
    for (row, count) in counted_rows(&snapshot, Some(&preset)) {
        let place = match row.reader {
            Reader::Universal(_) => Place::Universal,
            Reader::Agent(id) => Place::Agent(id),
        };
        let Some(Resolved::Agents { ids, .. }) = Resolved::of(&snapshot, &place) else {
            unreachable!("every row has a page");
        };
        assert_eq!(page_skills(&snapshot, Some(&preset), &ids).len(), count);
    }
}
