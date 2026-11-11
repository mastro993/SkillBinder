use super::super::snapshot_query::fixture::{Fixture, id, set};
use super::*;
use skillbinder_proto::{AppSnapshot, WorkspaceTargets};

/// Universal (Amp), Claude Code, and Cursor are detected; acme-web uses `.claude/skills`, and
/// docs-site uses none.
fn snapshot() -> AppSnapshot {
    Fixture::new()
        .agent("amp", "Amp", ".agents/skills", ".agents/skills", true)
        .agent(
            "claude-code",
            "Claude Code",
            ".claude/skills",
            ".claude/skills",
            true,
        )
        .agent("cursor", "Cursor", ".agents/skills", ".cursor/skills", true)
        .workspace("acme-web", &[".claude/skills"])
        .workspace("docs-site", &[])
        .build()
}

fn workspace(name: &str) -> Place {
    Place::Workspace(WorkspaceId(name.to_owned()))
}

/// The change Add saves for `selection` at `place`.
fn added(snapshot: &AppSnapshot, place: &Place, selection: &Selection) -> Option<AddSkills> {
    let resolved = Resolved::of(snapshot, place)?;
    change(selection, resolved.scope())
}

fn agents_change(slugs: &[&str], agents: &[&str]) -> AddSkills {
    AddSkills::YourAgents {
        skills: slugs.iter().map(|slug| (id(slug), set(agents))).collect(),
    }
}

#[test]
fn an_agents_page_adds_each_skill_for_the_agents_reading_its_directory() {
    let snapshot = snapshot();
    let mut selection = Selection::new(&Place::Universal);
    for slug in ["pdf", "review"] {
        selection.pick(id(slug), &BTreeSet::new());
    }
    assert_eq!(
        added(&snapshot, &Place::Universal, &selection),
        Some(agents_change(&["pdf", "review"], &["amp"]))
    );
    let claude = Place::Agent("claude-code".to_owned());
    assert_eq!(
        added(&snapshot, &claude, &selection),
        Some(agents_change(&["pdf", "review"], &["claude-code"]))
    );
}

#[test]
fn an_agent_linked_to_the_universal_directory_adds_for_both() {
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
        .build();
    let mut selection = Selection::new(&Place::Universal);
    selection.pick(id("pdf"), &BTreeSet::new());
    let claude = Place::Agent("claude-code".to_owned());
    let both = Some(agents_change(&["pdf"], &["amp", "claude-code"]));
    assert_eq!(added(&snapshot, &claude, &selection), both);
    assert_eq!(added(&snapshot, &Place::Universal, &selection), both);
}

#[test]
fn a_workspace_adds_all_agents_or_the_ticked_directories_and_joins_new_ones() {
    let snapshot = snapshot();
    let place = workspace("acme-web");
    let uses = set(&[".claude/skills"]);
    let mut selection = Selection::new(&place);
    selection.pick(id("pdf"), &uses);
    selection.pick(id("review"), &uses);
    selection.edit_pills(&id("review"), |pills| {
        pills.toggle_all(&uses);
        pills.toggle(&[".agents/skills".to_owned()], &uses);
    });
    assert_eq!(
        added(&snapshot, &place, &selection),
        Some(AddSkills::Workspace {
            id: WorkspaceId("acme-web".to_owned()),
            dirs: set(&[".agents/skills"]),
            skills: [
                (id("pdf"), WorkspaceTargets::AllAgents),
                (
                    id("review"),
                    WorkspaceTargets::Dirs(set(&[".agents/skills", ".claude/skills"]))
                ),
            ]
            .into(),
        })
    );
    assert_eq!(
        added(&snapshot, &Place::Universal, &selection),
        None,
        "a workspace's picks never go to your agents"
    );
}

#[test]
fn new_cards_start_with_the_workspace_agents_or_the_pills_last_ticked() {
    let uses = set(&[".claude/skills"]);
    assert_eq!(
        Pills::start(&uses, &set(&[".zed/skills"])),
        Pills {
            all: true,
            dirs: uses.clone()
        }
    );
    let none = BTreeSet::new();
    let mut selection = Selection::new(&workspace("docs-site"));
    selection.pick(id("pdf"), &none);
    selection.edit_pills(&id("pdf"), |pills| {
        pills.toggle(&[".claude/skills".to_owned()], &none);
    });
    selection.pick(id("review"), &none);
    let Selection::Workspace { cards, .. } = &selection else {
        unreachable!("a workspace's selection")
    };
    let starts: Vec<_> = cards.iter().map(|(_, pills)| pills.clone()).collect();
    assert_eq!(
        starts,
        [
            Pills {
                all: true,
                dirs: set(&[".claude/skills"])
            },
            Pills {
                all: true,
                dirs: set(&[".claude/skills"])
            },
        ]
    );
    assert_eq!(
        Pills::start(&none, &none).dirs,
        none,
        "the first pick ticks none"
    );
}

#[test]
fn the_all_pill_follows_the_ticked_directories_and_never_turns_on_by_itself() {
    let uses = set(&[".agents/skills", ".claude/skills"]);
    let group = |dir: &str| vec![dir.to_owned()];
    let mut pills = Pills::start(&uses, &BTreeSet::new());
    assert!(pills.all, "on by default");

    pills.toggle(&group(".cursor/skills"), &uses);
    assert!(pills.all, "a directory not used yet leaves All on");
    pills.toggle(&group(".claude/skills"), &uses);
    assert!(!pills.all, "unticking a used directory turns All off");
    assert_eq!(pills.dirs, set(&[".agents/skills", ".cursor/skills"]));

    pills.toggle(&group(".claude/skills"), &uses);
    assert!(!pills.all, "ticking every pill by hand never turns All on");

    pills.toggle_all(&uses);
    pills.toggle(&group(".cursor/skills"), &uses);
    pills.toggle_all(&uses);
    assert!(!pills.all);
    assert_eq!(pills.dirs, uses, "turning All off keeps the ticked pills");

    pills.toggle(&group(".agents/skills"), &uses);
    pills.toggle_all(&uses);
    assert!(pills.all);
    assert_eq!(
        pills.dirs, uses,
        "turning All on ticks every used directory"
    );
}

#[test]
fn add_says_why_it_is_disabled() {
    let none = BTreeSet::new();
    let mut selection = Selection::new(&workspace("docs-site"));
    assert_eq!(selection.blocked(), Some("Select skills".to_owned()));
    selection.pick(id("pdf"), &none);
    selection.pick(id("review"), &none);
    assert_eq!(
        selection.blocked(),
        Some("Pick an agent for pdf".to_owned())
    );
    selection.edit_pills(&id("pdf"), |pills| {
        pills.toggle(&[".claude/skills".to_owned()], &none);
    });
    assert_eq!(
        selection.blocked(),
        Some("Pick an agent for review".to_owned())
    );
    selection.pick(id("review"), &none);
    assert_eq!(selection.blocked(), None);
}

#[test]
fn directories_that_resolve_to_one_folder_are_one_pill() {
    let snapshot = Fixture::new()
        .agent("amp", "Amp", ".agents/skills", ".agents/skills", true)
        .agent(
            "claude-code",
            "Claude Code",
            ".claude/skills",
            ".claude/skills",
            true,
        )
        .workspace("acme-web", &[".agents/skills", ".claude/skills"])
        .dir_link(".claude/skills", ".agents/skills")
        .build();
    let Some(Resolved::Workspace { entries, workspace }) =
        Resolved::of(&snapshot, &workspace("acme-web"))
    else {
        unreachable!("acme-web resolves")
    };
    assert_eq!(entries.len(), 1);
    let mut pills = Pills::start(&workspace.dirs, &BTreeSet::new());
    pills.toggle(&entries[0].keys, &workspace.dirs);
    assert!(pills.dirs.is_empty(), "both directories untick together");
    pills.toggle(&entries[0].keys, &workspace.dirs);
    assert_eq!(pills.dirs, set(&[".agents/skills", ".claude/skills"]));
}

#[test]
fn a_gone_place_does_not_resolve() {
    let snapshot = snapshot();
    assert!(Resolved::of(&snapshot, &workspace("gone")).is_none());
    assert!(Resolved::of(&snapshot, &Place::Agent("zed".to_owned())).is_none());
}
