use super::fixture::{Fixture, agents, dirs, id, preset_id, set};
use super::*;
use skillbinder_proto::{Drift, UpdateCheck};

/// Universal (Amp and Codex), Claude Code, and Cursor are detected; Zed is not. acme-web uses
/// `.agents/skills` and `.claude/skills`, api-server `.claude/skills`, and docs-site none.
fn base() -> Fixture {
    Fixture::new()
        .agent("amp", "Amp", ".agents/skills", ".agents/skills", true)
        .agent(
            "claude-code",
            "Claude Code",
            ".claude/skills",
            ".claude/skills",
            true,
        )
        .agent("codex", "Codex", ".agents/skills", ".agents/skills", true)
        .agent("cursor", "Cursor", ".agents/skills", ".cursor/skills", true)
        .own_dir(".cursor/skills")
        .agent("zed", "Zed", ".zed/skills", ".zed/skills", false)
        .preset("frontend", "Frontend")
        .folder("web", "Web")
        .workspace("acme-web", &[".agents/skills", ".claude/skills"])
        .workspace("api-server", &[".claude/skills"])
        .workspace("docs-site", &[])
}

/// Skills a to f: a for Claude Code, b only in Frontend, c only in api-server, d only for Zed,
/// e for all agents, f nowhere.
fn library() -> Fixture {
    base()
        .skill("a-review", "Review diffs.", Some("web"))
        .skill("b-design", "Design pages.", Some("web"))
        .skill("c-sql", "Write SQL.", None)
        .skill("d-zed", "Only Zed.", None)
        .skill("e-all", "Everywhere, debug.", Some("web"))
        .skill("f-off", "Nowhere, debug.", None)
        .hold("a-review", "default", agents(&["claude-code"]))
        .hold("b-design", "frontend", Targets::AllAgents)
        .ws_hold("api-server", "c-sql", WorkspaceTargets::AllAgents)
        .hold("d-zed", "default", agents(&["zed"]))
        .hold("e-all", "default", Targets::AllAgents)
}

fn skill<'a>(snapshot: &'a AppSnapshot, slug: &str) -> &'a Skill {
    snapshot
        .library
        .iter()
        .flat_map(|library| &library.library.skills)
        .find(|skill| skill.id == id(slug))
        .unwrap_or_else(|| unreachable!("no skill {slug}"))
}

fn query(filter: LibraryFilter) -> Query<'static> {
    Query {
        folder: None,
        search: "",
        filter,
        keep: None,
    }
}

fn slugs(skills: &[&Skill]) -> Vec<String> {
    skills.iter().map(|skill| skill.id.to_string()).collect()
}

fn segment(active: ActiveFilter) -> LibraryFilter {
    LibraryFilter {
        active,
        updates: false,
    }
}

#[test]
fn active_means_on_now_for_a_detected_agent_or_in_a_workspace() {
    // The snapshot's active preset is Frontend, but the view passes Default.
    let snapshot = library().active("frontend").build();
    let places = Places::new(&snapshot);
    let default = preset_id("default");
    let active = |slug: &str| is_active(&places, skill(&snapshot, slug), &default);
    assert!(active("a-review"), "held for a detected agent");
    assert!(!active("b-design"), "held only by a preset that is off");
    assert!(active("c-sql"), "held only by a workspace");
    assert!(
        !active("d-zed"),
        "held only for an agent that is not detected"
    );
    assert!(!active("f-off"));
    let frontend = preset_id("frontend");
    assert!(
        is_active(&places, skill(&snapshot, "b-design"), &frontend),
        "the preset passed in decides, not the snapshot's"
    );
    assert!(!is_active(&places, skill(&snapshot, "a-review"), &frontend));
}

#[test]
fn all_agents_with_none_detected_is_inactive_and_a_missing_workspace_still_holds() {
    let snapshot = Fixture::new()
        .agent("zed", "Zed", ".zed/skills", ".zed/skills", false)
        .workspace("gone", &[".zed/skills"])
        .missing()
        .skill("all", "", None)
        .skill("kept", "", None)
        .hold("all", "default", Targets::AllAgents)
        .ws_hold("gone", "kept", WorkspaceTargets::AllAgents)
        .build();
    let places = Places::new(&snapshot);
    let default = preset_id("default");
    assert!(!is_active(&places, skill(&snapshot, "all"), &default));
    let kept = places.of(skill(&snapshot, "kept"), &default);
    assert!(kept.is_active());
    assert!(kept.workspaces[0].missing());
}

#[test]
fn all_active_and_inactive_partition_the_library_by_name() {
    let snapshot = library().build();
    let places = Places::new(&snapshot);
    let default = preset_id("default");
    let shown = |active| {
        slugs(&library_skills(
            &snapshot,
            &places,
            &default,
            query(segment(active)),
        ))
    };
    assert_eq!(
        shown(ActiveFilter::All),
        ["a-review", "b-design", "c-sql", "d-zed", "e-all", "f-off"]
    );
    assert_eq!(shown(ActiveFilter::Active), ["a-review", "c-sql", "e-all"]);
    assert_eq!(
        shown(ActiveFilter::Inactive),
        ["b-design", "d-zed", "f-off"]
    );
}

#[test]
fn updates_keep_only_a_newer_version_at_the_source() {
    let snapshot = library()
        .update("a-review", UpdateCheck::Available)
        .update("b-design", UpdateCheck::Available)
        .update("c-sql", UpdateCheck::Failed("Offline.".to_owned()))
        .update("e-all", UpdateCheck::UpToDate)
        .drift("e-all", Drift::Restorable)
        .build();
    let places = Places::new(&snapshot);
    let default = preset_id("default");
    let updates = LibraryFilter {
        active: ActiveFilter::All,
        updates: true,
    };
    assert_eq!(
        slugs(&library_skills(
            &snapshot,
            &places,
            &default,
            query(updates)
        )),
        ["a-review", "b-design"],
        "a failed check and a drifted skill are not updates"
    );
    let active_updates = LibraryFilter {
        active: ActiveFilter::Active,
        updates: true,
    };
    assert_eq!(
        slugs(&library_skills(
            &snapshot,
            &places,
            &default,
            query(active_updates)
        )),
        ["a-review"]
    );
    assert!(!has_update(&Fixture::new().build(), &id("a-review")));
}

#[test]
fn filters_combine_with_the_folder_tab_and_search() {
    let snapshot = library()
        .update("a-review", UpdateCheck::Available)
        .update("c-sql", UpdateCheck::Available)
        .build();
    let places = Places::new(&snapshot);
    let default = preset_id("default");
    let web = FolderId("web".to_owned());
    let in_web = Query {
        folder: Some(&web),
        ..query(segment(ActiveFilter::Active))
    };
    assert_eq!(
        slugs(&library_skills(&snapshot, &places, &default, in_web)),
        ["a-review", "e-all"]
    );
    let searched = Query {
        search: "debug",
        ..query(segment(ActiveFilter::Inactive))
    };
    assert_eq!(
        slugs(&library_skills(&snapshot, &places, &default, searched)),
        ["f-off"],
        "search matches the description"
    );
    let updates = LibraryFilter {
        active: ActiveFilter::All,
        updates: true,
    };
    let updates_in_web = Query {
        folder: Some(&web),
        ..query(updates)
    };
    assert_eq!(
        slugs(&library_skills(
            &snapshot,
            &places,
            &default,
            updates_in_web
        )),
        ["a-review"]
    );
    let updates_searched = Query {
        search: "sql",
        ..query(updates)
    };
    assert_eq!(
        slugs(&library_skills(
            &snapshot,
            &places,
            &default,
            updates_searched
        )),
        ["c-sql"]
    );
}

#[test]
fn each_count_applies_the_other_filters() {
    let snapshot = library()
        .update("a-review", UpdateCheck::Available)
        .update("f-off", UpdateCheck::Available)
        .build();
    let places = Places::new(&snapshot);
    let default = preset_id("default");
    let counts = |filter, search| {
        filter_counts(
            &snapshot,
            &places,
            &default,
            Query {
                search,
                ..query(filter)
            },
        )
    };
    let all = FilterCounts {
        all: 6,
        active: 3,
        inactive: 3,
        updates: 2,
    };
    assert_eq!(counts(segment(ActiveFilter::All), ""), all);
    assert_eq!(
        counts(segment(ActiveFilter::Active), "").updates,
        1,
        "Updates counts the chosen segment"
    );
    let updates = LibraryFilter {
        active: ActiveFilter::All,
        updates: true,
    };
    assert_eq!(
        counts(updates, ""),
        FilterCounts {
            all: 2,
            active: 1,
            inactive: 1,
            updates: 2,
        },
        "segments count Updates"
    );
    assert_eq!(
        counts(segment(ActiveFilter::All), "debug"),
        FilterCounts {
            all: 2,
            active: 1,
            inactive: 1,
            updates: 1,
        }
    );
}

#[test]
fn the_open_popovers_skill_stays_listed() {
    let snapshot = library().build();
    let places = Places::new(&snapshot);
    let default = preset_id("default");
    let kept = id("f-off");
    let active = Query {
        keep: Some(&kept),
        ..query(segment(ActiveFilter::Active))
    };
    assert_eq!(
        slugs(&library_skills(&snapshot, &places, &default, active)),
        ["a-review", "c-sql", "e-all", "f-off"]
    );
}

#[test]
fn skills_sort_by_slug_ignoring_case() {
    let snapshot = Fixture::new()
        .skill("pdf", "", None)
        .skill("zeta", "", None)
        .skill("Docx", "", None)
        .skill("beta", "", None)
        .build();
    let places = Places::new(&snapshot);
    let shown = library_skills(
        &snapshot,
        &places,
        &preset_id("default"),
        query(LibraryFilter::default()),
    );
    assert_eq!(slugs(&shown), ["beta", "Docx", "pdf", "zeta"]);
}

#[test]
fn a_global_directory_linked_to_the_universal_one_is_one_entry_naming_both() {
    let snapshot = base()
        .agent(
            "windsurf",
            "Windsurf",
            ".windsurf/skills",
            ".windsurf/skills",
            true,
        )
        .linked(".agents/skills")
        .build();
    let places = Places::new(&snapshot);
    let names: Vec<_> = places
        .agents
        .iter()
        .map(|entry| entry.row.name.as_str())
        .collect();
    assert_eq!(names, ["Universal", "Claude Code", "Cursor"]);
    let universal = &places.agents[0];
    assert_eq!(universal.keys, ["amp", "codex", "windsurf"]);
    assert_eq!(
        universal.detail().as_deref(),
        Some("Amp, Codex, Windsurf"),
        "the tooltip names every agent that reads it"
    );
    assert_eq!(places.agents[1].detail(), None);
    assert_eq!(
        places.every,
        ["amp", "codex", "windsurf", "claude-code", "cursor"]
    );
}

#[test]
fn all_agents_holds_every_entry() {
    let snapshot = library().build();
    let places = Places::new(&snapshot);
    let default = preset_id("default");
    let everywhere = places.of(skill(&snapshot, "e-all"), &default);
    assert!(everywhere.all_agents());
    assert!(everywhere.agents.iter().all(|held| held.held));
    let one = places.of(skill(&snapshot, "a-review"), &default);
    assert!(!one.all_agents());
    let held: Vec<_> = one
        .held_agents()
        .map(|entry| entry.row.name.as_str())
        .collect();
    assert_eq!(held, ["Claude Code"]);
}

#[test]
fn workspaces_come_in_sidebar_order_with_the_directories_they_hold() {
    let snapshot = library()
        .ws_hold("acme-web", "a-review", WorkspaceTargets::AllAgents)
        .ws_hold("api-server", "a-review", dirs(&[".claude/skills"]))
        .build();
    let places = Places::new(&snapshot);
    let review = places.of(skill(&snapshot, "a-review"), &preset_id("default"));
    let holding: Vec<_> = review
        .holding_workspaces()
        .map(|place| (place.workspace.name.as_str(), place.held_names()))
        .collect();
    assert_eq!(
        holding,
        [
            ("acme-web", "Universal, Claude Code".to_owned()),
            ("api-server", "Claude Code".to_owned()),
        ],
        "a workspace held for all its agents holds each directory it uses"
    );
    let acme = &review.workspaces[0];
    let lines: Vec<_> = acme
        .entries
        .iter()
        .map(|held| (held.entry.keys.clone(), held.held))
        .collect();
    assert_eq!(
        lines,
        [
            (vec![".agents/skills".to_owned()], true),
            (vec![".claude/skills".to_owned()], true),
        ]
    );
}

#[test]
fn a_workspace_without_agents_offers_the_detected_agents_directories() {
    let snapshot = library().build();
    let places = Places::new(&snapshot);
    let review = places.of(skill(&snapshot, "a-review"), &preset_id("default"));
    let docs = &review.workspaces[2];
    assert!(docs.uses_none() && !docs.holds());
    let offered: Vec<_> = docs
        .entries
        .iter()
        .map(|held| held.entry.keys[0].as_str())
        .collect();
    assert_eq!(
        offered,
        [".agents/skills", ".claude/skills", ".cursor/skills"],
        "Zed is not detected, so its directory is not offered"
    );
    assert_eq!(docs.entries[0].entry.row.name, "Universal");
    assert_eq!(
        docs.toggled(docs.entries[1].entry),
        Some(dirs(&[".claude/skills"])),
        "choosing one makes it the workspace's agent"
    );
}

#[test]
fn directories_resolving_to_one_folder_share_a_pill() {
    let snapshot = base()
        .workspace("linked", &[".agents/skills", ".claude/skills"])
        .dir_link(".claude/skills", ".agents/skills")
        .build();
    let places = Places::new(&snapshot);
    let linked = &places.workspaces[3];
    assert_eq!(linked.entries.len(), 1);
    assert_eq!(linked.entries[0].keys, [".agents/skills", ".claude/skills"]);
}

#[test]
fn the_switch_gives_all_agents_or_the_fixed_list_listed_now() {
    let snapshot = library().build();
    let places = Places::new(&snapshot);
    let everywhere = places.of(skill(&snapshot, "e-all"), &preset_id("default"));
    assert_eq!(
        everywhere.switched(false),
        Targets::Agents(set(&["amp", "claude-code", "codex", "cursor"]))
    );
    assert_eq!(everywhere.switched(true), Targets::AllAgents);
    let universal = everywhere.agents[0].entry;
    assert_eq!(
        everywhere.toggled(universal),
        Targets::Agents(set(&["claude-code", "cursor"])),
        "a pill off from All agents keeps the others as a fixed list"
    );
}

#[test]
fn the_last_pill_off_empties_the_preset_or_deactivates_the_workspace() {
    let snapshot = library()
        .ws_hold("api-server", "a-review", WorkspaceTargets::AllAgents)
        .build();
    let places = Places::new(&snapshot);
    let review = places.of(skill(&snapshot, "a-review"), &preset_id("default"));
    let claude = review
        .agents
        .iter()
        .find(|held| held.held)
        .map(|held| held.entry)
        .unwrap_or_else(|| unreachable!("a-review is held for Claude Code"));
    assert_eq!(review.toggled(claude), Targets::Agents(set(&[])));
    let api = &review.workspaces[1];
    assert_eq!(api.toggled(api.entries[0].entry), None);
}
