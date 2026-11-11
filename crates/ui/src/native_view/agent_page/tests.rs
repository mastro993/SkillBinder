use super::super::agent_skills::UNIVERSAL;
use super::super::snapshot_query::fixture::{Fixture, HOME, agents, id, preset_id};
use super::super::undo::Undo;
use super::rows::{RemovalPlan, plan_removal};
use super::*;
use skillbinder_proto::{SameSlug, Targets};

fn agent(id: &str, global: &str, detected: bool) -> AgentOption {
    AgentOption {
        id: id.to_owned(),
        display_name: id.to_owned(),
        workspace_skills_dir: ".agents/skills".to_owned(),
        other_workspace_dirs: Vec::new(),
        global_skills_dir: Some(global.to_owned()),
        resolved_skills_dir: Some(global.to_owned()),
        other_global_dirs: Vec::new(),
        same_slug: SameSlug::Unknown,
        detected,
    }
}

/// Amp and Codex read `~/.agents/skills`, Claude Code and Cursor their own directories.
fn fixture() -> Fixture {
    Fixture::new()
        .agent("amp", "Amp", ".agents/skills", ".agents/skills", true)
        .agent("codex", "Codex", ".agents/skills", ".agents/skills", true)
        .agent(
            "claude-code",
            "Claude Code",
            ".claude/skills",
            ".claude/skills",
            true,
        )
        .agent("cursor", "Cursor", ".agents/skills", ".cursor/skills", true)
}

fn page_ids(snapshot: &AppSnapshot, place: &Place) -> Vec<String> {
    match Resolved::of(snapshot, place) {
        Some(Resolved::Agents { ids, .. }) => ids,
        _ => unreachable!("no page for {place:?}"),
    }
}

fn remove(snapshot: &AppSnapshot, place: &Place, slug: &str) -> Option<RemovalPlan> {
    plan_removal(snapshot, &preset_id("default"), place, id(slug))
}

fn claude() -> Place {
    Place::Agent("claude-code".to_owned())
}

#[test]
fn a_page_reads_for_every_detected_agent_reading_its_directory() {
    let snapshot = fixture().build();
    assert_eq!(page_ids(&snapshot, &claude()), ["claude-code"]);
    assert_eq!(page_ids(&snapshot, &Place::Universal), ["amp", "codex"]);
    let linked = fixture().linked(".agents/skills").build();
    assert_eq!(
        page_ids(&linked, &Place::Universal),
        ["amp", "codex", "cursor"],
        "Cursor's directory links to the universal one"
    );
    assert_eq!(
        page_ids(&linked, &Place::Agent("cursor".to_owned())),
        ["amp", "codex", "cursor"]
    );
}

#[test]
fn removing_from_all_agents_keeps_the_other_detected_agents() {
    let snapshot = fixture()
        .agent("zed", "Zed", ".zed/skills", ".zed/skills", false)
        .skill("review", "", None)
        .hold("review", "default", Targets::AllAgents)
        .build();
    let plan = remove(&snapshot, &claude(), "review");
    let Some(RemovalPlan { targets, undo, row }) = plan else {
        unreachable!("the page holds review");
    };
    assert_eq!(
        targets,
        agents(&["amp", "codex", "cursor"]),
        "an undetected agent never joins the list"
    );
    assert_eq!(
        undo,
        Undo::Preset {
            id: preset_id("default"),
            skill: id("review"),
            previous: Targets::AllAgents,
        },
        "Undo saves the entry back to the preset it came from"
    );
    assert_eq!(row.resolved, format!("{HOME}/.claude/skills"));
}

#[test]
fn removing_a_shared_directory_drops_all_its_agents_together() {
    let snapshot = fixture()
        .linked(".agents/skills")
        .skill("review", "", None)
        .hold(
            "review",
            "default",
            agents(&["amp", "claude-code", "cursor"]),
        )
        .build();
    let targets = remove(&snapshot, &Place::Universal, "review").map(|plan| plan.targets);
    assert_eq!(targets, Some(agents(&["claude-code"])));
}

#[test]
fn removing_the_only_directory_leaves_an_empty_list_and_keeps_undetected_agents() {
    let snapshot = fixture()
        .skill("review", "", None)
        .hold("review", "default", agents(&["claude-code"]))
        .skill("kept", "", None)
        .hold("kept", "default", agents(&["claude-code", "zed"]))
        .build();
    let targets = |slug| remove(&snapshot, &claude(), slug).map(|plan| plan.targets);
    assert_eq!(targets("review"), Some(agents(&[])));
    assert_eq!(targets("kept"), Some(agents(&["zed"])));
}

#[test]
fn a_skill_the_page_does_not_hold_is_never_added() {
    let snapshot = fixture()
        .preset("frontend", "Frontend")
        .skill("codex-only", "", None)
        .hold("codex-only", "default", agents(&["codex"]))
        .skill("elsewhere", "", None)
        .hold("elsewhere", "frontend", Targets::AllAgents)
        .build();
    assert_eq!(remove(&snapshot, &claude(), "codex-only"), None);
    assert_eq!(remove(&snapshot, &claude(), "elsewhere"), None);
    assert_eq!(remove(&snapshot, &claude(), "gone"), None);
}

#[test]
fn a_page_without_an_active_preset_lists_nothing() {
    let snapshot = fixture()
        .skill("review", "", None)
        .hold("review", "default", Targets::AllAgents)
        .build();
    let ids = page_ids(&snapshot, &claude());
    assert_eq!(page_skills(&snapshot, None, &ids).len(), 0);
    assert_eq!(
        page_skills(&snapshot, Some(&preset_id("default")), &ids).len(),
        1
    );
}

#[test]
fn pages_of_agents_no_longer_detected_open_discovery() {
    let agents = [
        agent("codex", "/home/.agents/skills", true),
        agent("claude-code", "/home/.claude/skills", true),
        agent("cursor", "/home/.cursor/skills", false),
    ];
    let fallback = |route| agent_fallback(&agents, "/home", route);
    let page = |id: &str| Route::Agent(id.to_owned());
    assert_eq!(fallback(page("claude-code")), page("claude-code"));
    assert_eq!(fallback(page("codex")), Route::Universal);
    assert_eq!(fallback(page("cursor")), Route::Discovery);
    assert_eq!(fallback(Route::Universal), Route::Universal);
    let undetected = [agent("codex", "/home/.agents/skills", false)];
    assert_eq!(
        agent_fallback(&undetected, "/home", Route::Universal),
        Route::Discovery
    );
}

#[test]
fn agents_reading_one_folder_through_a_link_share_it() {
    let mut claude = agent("claude-code", "/home/.claude/skills", true);
    claude.resolved_skills_dir = Some("/home/.agents/skills".to_owned());
    let agents = [
        agent("codex", "/home/.agents/skills", true),
        claude,
        agent("cursor", "/home/.cursor/skills", true),
        agent("zed", "/home/.agents/skills", false),
    ];
    let ids = |resolved: &str, except: Option<&str>| {
        readers(&agents, resolved, except)
            .into_iter()
            .map(|agent| agent.id.as_str())
            .collect::<Vec<_>>()
    };
    let shared = "/home/.agents/skills";
    assert_eq!(ids(shared, Some("claude-code")), ["codex"]);
    assert_eq!(ids(shared, Some("codex")), ["claude-code"]);
    assert_eq!(ids(shared, None), ["codex", "claude-code"]);
    assert!(ids("/home/.cursor/skills", Some("cursor")).is_empty());
}

#[test]
fn members_are_named_as_the_agents_section_names_them() {
    let config = "/home/.config/agents/skills";
    let mut registry_universal = agent("universal", config, true);
    registry_universal.display_name = UNIVERSAL.to_owned();
    let agents = [agent("amp", config, true), registry_universal];
    assert_eq!(
        names(&readers(&agents, config, Some("amp")), "/home"),
        ["Universal (~/.config/agents/skills)"]
    );
}
