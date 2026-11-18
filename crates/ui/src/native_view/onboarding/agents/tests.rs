use super::super::super::library_copies::found_rows;
use super::{choice, unticked};
use skillbinder_proto::{AgentOption, SameSlug};
use std::collections::BTreeSet;

/// A found agent reading `dir` under `/home`, chosen when `chosen`.
fn agent(id: &str, dir: &str, chosen: bool) -> AgentOption {
    let dir = format!("/home/{dir}");
    AgentOption {
        id: id.to_owned(),
        display_name: id.to_owned(),
        workspace_skills_dir: ".agents/skills".to_owned(),
        other_workspace_dirs: Vec::new(),
        global_skills_dir: Some(dir.clone()),
        resolved_skills_dir: Some(dir),
        other_global_dirs: Vec::new(),
        same_slug: SameSlug::Unknown,
        detected: true,
        chosen,
        asked: true,
    }
}

fn dirs(dirs: &[&str]) -> BTreeSet<String> {
    dirs.iter().map(|dir| format!("/home/{dir}")).collect()
}

#[test]
fn every_row_starts_ticked_until_the_choice_is_saved_then_shows_it() {
    let agents = [
        agent("amp", ".agents/skills", true),
        agent("codex", ".agents/skills", true),
        agent("cursor", ".cursor/skills", false),
    ];
    let rows = found_rows(&agents, "/home");
    assert!(unticked(&rows, &agents, false).is_empty());
    assert_eq!(unticked(&rows, &agents, true), dirs(&[".cursor/skills"]));
}

#[test]
fn continue_saves_every_row_shown_and_the_ticked_ones_universal_as_one() {
    let agents = [
        agent("amp", ".agents/skills", false),
        agent("codex", ".agents/skills", false),
        agent("claude-code", ".claude/skills", false),
        agent("cursor", ".cursor/skills", false),
    ];
    let rows = found_rows(&agents, "/home");
    assert_eq!(rows.len(), 3, "Universal is one row");
    let (shown, chosen) = choice(&rows, &dirs(&[".cursor/skills"]));
    assert_eq!(
        shown,
        dirs(&[".agents/skills", ".claude/skills", ".cursor/skills"])
    );
    assert_eq!(chosen, dirs(&[".agents/skills", ".claude/skills"]));
    let (_, none) = choice(&rows, &shown);
    assert!(none.is_empty(), "zero ticked is allowed");
}
