use super::{found_line, pending, question};
use skillbinder_proto::{AgentOption, SameSlug};
use std::collections::BTreeSet;

/// A found agent reading `dir` under `/home`, with its choice and whether it was asked about.
fn agent(id: &str, dir: &str, chosen: bool, asked: bool) -> AgentOption {
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
        asked,
    }
}

#[test]
fn only_found_agents_never_asked_about_wait_for_the_notice() {
    let mut gone = agent("hermes", ".hermes/skills", false, false);
    gone.detected = false;
    let agents = [
        agent("claude-code", ".claude/skills", true, true),
        // Unticked in Choose your agents, or turned off in Settings.
        agent("cursor", ".cursor/skills", false, true),
        agent("kiro-cli", ".kiro/skills", false, false),
        agent("windsurf", ".windsurf/skills", false, false),
        gone,
    ];
    let names = |answered: &BTreeSet<String>| -> Vec<String> {
        pending(&agents, "/home", answered)
            .into_iter()
            .map(|row| row.name)
            .collect()
    };
    assert_eq!(names(&BTreeSet::new()), ["kiro-cli", "windsurf"]);
    let answered = ["/home/.windsurf/skills".to_owned()].into();
    assert_eq!(
        names(&answered),
        ["kiro-cli"],
        "an answer counts before the snapshot shows it"
    );
}

#[test]
fn the_found_line_names_two_agents_then_counts_the_rest() {
    assert_eq!(found_line(&["Codex"]), "Found Codex.");
    assert_eq!(
        found_line(&["Codex", "Windsurf"]),
        "Found Codex and Windsurf."
    );
    assert_eq!(
        found_line(&["Codex", "Windsurf", "Kiro CLI", "Hermes Agent"]),
        "Found Codex, Windsurf, and 2 more."
    );
    assert_eq!(question(false), "Let SkillBinder manage its skills?");
    assert_eq!(question(true), "Let SkillBinder manage their skills?");
}
