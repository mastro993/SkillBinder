use super::super::super::library_copies::agent_rows;
use super::super::super::library_targets::sharing;
use super::super::fixture::{Fixture, agents, dirs, preset_id};
use super::*;
use skillbinder_proto::{Targets, WorkspaceTargets};

fn slugs(skills: &[&Skill]) -> Vec<String> {
    skills.iter().map(|skill| skill.id.to_string()).collect()
}

/// The ids of the agents reading the directory of the Agents row named `name`.
fn reading(snapshot: &AppSnapshot, name: &str) -> Vec<String> {
    let rows = agent_rows(&snapshot.agents, &snapshot.home);
    let row = rows
        .iter()
        .find(|row| row.name == name)
        .unwrap_or_else(|| unreachable!("no row {name}"));
    sharing(row, &rows)
}

#[test]
fn a_workspace_lists_only_the_library_skills_it_holds_by_slug() {
    let snapshot = Fixture::new()
        .workspace("acme-web", &[".claude/skills"])
        .skill("zeta", "", None)
        .skill("Alpha", "", None)
        .skill("off", "", None)
        .ws_hold("acme-web", "zeta", WorkspaceTargets::AllAgents)
        .ws_hold("acme-web", "Alpha", dirs(&[".claude/skills"]))
        .ws_hold("acme-web", "gone", WorkspaceTargets::AllAgents)
        .build();
    let workspace = &snapshot.workspaces[0];
    assert_eq!(
        slugs(&workspace_skills(&snapshot, workspace)),
        ["Alpha", "zeta"]
    );
}

#[test]
fn an_agent_lists_the_skills_the_preset_holds_for_its_directory() {
    let snapshot = Fixture::new()
        .agent("amp", "Amp", ".agents/skills", ".agents/skills", true)
        .agent(
            "claude-code",
            "Claude Code",
            ".claude/skills",
            ".claude/skills",
            true,
        )
        .agent("cursor", "Cursor", ".agents/skills", ".cursor/skills", true)
        .agent("zed", "Zed", ".zed/skills", ".zed/skills", false)
        .preset("frontend", "Frontend")
        .skill("all", "", None)
        .skill("claude", "", None)
        .skill("amp", "", None)
        .skill("other-preset", "", None)
        .skill("zed", "", None)
        .hold("all", "default", Targets::AllAgents)
        .hold("claude", "default", agents(&["claude-code"]))
        .hold("amp", "default", agents(&["amp"]))
        .hold("other-preset", "frontend", Targets::AllAgents)
        .hold("zed", "default", agents(&["zed"]))
        .build();
    let listed = |name: &str| {
        let ids = reading(&snapshot, name);
        slugs(&agent_skills(&snapshot, &preset_id("default"), &ids))
    };
    assert_eq!(listed("Universal"), ["all", "amp"]);
    assert_eq!(listed("Claude Code"), ["all", "claude"]);
    assert_eq!(listed("Cursor"), ["all"]);
}

#[test]
fn an_agent_linked_to_the_universal_directory_shares_its_skills() {
    let snapshot = Fixture::new()
        .agent("amp", "Amp", ".agents/skills", ".agents/skills", true)
        .agent(
            "claude-code",
            "Claude Code",
            ".claude/skills",
            ".claude/skills",
            true,
        )
        .linked(".agents/skills")
        .skill("amp", "", None)
        .hold("amp", "default", agents(&["amp"]))
        .build();
    let listed = |name: &str| {
        let ids = reading(&snapshot, name);
        slugs(&agent_skills(&snapshot, &preset_id("default"), &ids))
    };
    assert_eq!(listed("Universal"), ["amp"]);
    assert_eq!(listed("Claude Code"), ["amp"]);
}

#[test]
fn the_picker_lists_the_folder_and_search_by_name() {
    let snapshot = Fixture::new()
        .folder("web", "Web")
        .skill("pdf", "Read PDFs.", Some("web"))
        .skill("debug", "Find bugs.", None)
        .skill("Charts", "Plot PDF data.", Some("web"))
        .build();
    let web = Some(FolderId("web".into()));
    let listed = |folder: Option<&FolderId>, search| slugs(&searched(&snapshot, folder, search));
    assert_eq!(listed(None, ""), ["Charts", "debug", "pdf"]);
    assert_eq!(listed(web.as_ref(), ""), ["Charts", "pdf"]);
    assert_eq!(listed(None, "pdf"), ["Charts", "pdf"]);
    assert_eq!(listed(None, "bug"), ["debug"]);
}
