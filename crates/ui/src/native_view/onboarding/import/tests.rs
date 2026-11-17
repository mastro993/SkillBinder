use super::super::super::library_copies::{AgentRow, Reader};
use super::summary::summary;
use skillbinder_proto::*;

/// A found version of `slug` in the global skill directory the agents `agents` read.
fn found_in(slug: &str, agents: &[&str]) -> Candidate {
    Candidate {
        id: CandidateId(format!("{slug}@{}", agents.join("+"))),
        key: slug.to_owned(),
        preferred: true,
        matches_source: false,
        modified: None,
        skill_id: None,
        held: None,
        slug: slug.to_owned(),
        description: String::new(),
        locations: vec![CandidateLocation {
            display_path: format!("~/{slug}"),
            readers: Vec::new(),
            agents: Vec::new(),
            global_agents: agents.iter().map(|agent| (*agent).to_owned()).collect(),
            workspaces: Vec::new(),
        }],
        validation: ValidationStatus::Valid,
        messages: Vec::new(),
        total_bytes: 1,
        file_count: 1,
    }
}

fn row(name: &str, ids: &[&str]) -> AgentRow {
    AgentRow {
        reader: Reader::Agent(ids[0].to_owned()),
        name: name.to_owned(),
        ids: ids.iter().map(|id| (*id).to_owned()).collect(),
        dir: format!("~/{name}"),
        resolved: format!("/h/{name}"),
    }
}

fn scan(candidates: Vec<Candidate>) -> ScanSnapshot {
    ScanSnapshot {
        candidates,
        error: None,
    }
}

#[test]
fn the_summary_counts_each_found_skill_once_and_names_its_agents_in_section_order() {
    let rows = [
        row("Universal", &["codex", "amp"]),
        row("Claude Code", &["claude-code"]),
        row("Cursor", &["cursor"]),
    ];
    let mut held = found_in("held", &["cursor"]);
    held.held = Some(SkillId("held".to_owned()));
    let found = scan(vec![
        found_in("review", &["claude-code"]),
        // Another version of the same skill, read by the universal agents.
        found_in("review", &["amp"]),
        found_in("pdf", &["claude-code"]),
        // Already in library, so not found for import.
        held,
    ]);

    assert_eq!(
        summary(&found, &rows).text(),
        "2 skills found in Universal, Claude Code"
    );
    let cursor = scan(vec![found_in("pdf", &["cursor"])]);
    assert_eq!(summary(&cursor, &rows).text(), "1 skill found in Cursor");
    // Found only where no Agents row reads, such as through a link.
    let elsewhere = scan(vec![found_in("linked", &[])]);
    assert_eq!(
        summary(&elsewhere, &rows).text(),
        "1 skill found in your agents"
    );
}
