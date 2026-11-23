use super::super::super::library_copies::found_rows;
use super::{TURNED_OFF, detail};
use skillbinder_proto::{AgentOption, SameSlug};
use std::collections::BTreeSet;

/// A found agent reading `dir` under `/home`, chosen when `chosen`.
fn agent(id: &str, name: &str, dir: &str, chosen: bool) -> AgentOption {
    let dir = format!("/home/{dir}");
    AgentOption {
        id: id.to_owned(),
        display_name: name.to_owned(),
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

#[test]
fn rows_list_every_found_agent_and_a_turned_off_one_says_its_skills_stay() {
    let mut gone = agent("windsurf", "Windsurf", ".windsurf/skills", false);
    gone.detected = false;
    let agents = [
        agent("amp", "Amp", ".agents/skills", true),
        agent("codex", "Codex", ".agents/skills", true),
        agent("cursor", "Cursor", ".cursor/skills", false),
        agent("gemini-cli", "Gemini CLI", ".gemini/skills", false),
        gone,
    ];
    let rows = found_rows(&agents, "/home");
    let names: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(
        names,
        ["Universal", "Cursor", "Gemini CLI"],
        "only found agents"
    );
    let on: Vec<bool> = rows.iter().map(|row| row.chosen(&agents)).collect();
    assert_eq!(on, [true, false, false]);

    let turned_off: BTreeSet<String> = ["/home/.cursor/skills".to_owned()].into();
    let lines: Vec<String> = rows
        .iter()
        .zip(&on)
        .map(|(row, on)| detail(row, *on, &turned_off, "/home"))
        .collect();
    assert_eq!(
        lines,
        ["Amp, Codex", TURNED_OFF, "~/.gemini/skills"],
        "an agent never on keeps its folder line"
    );
    let cursor = &rows[1];
    assert_eq!(
        detail(cursor, true, &turned_off, "/home"),
        "~/.cursor/skills",
        "turned on again, it shows its folder"
    );
}
