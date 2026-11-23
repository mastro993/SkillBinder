use super::{grammar, request};
use skillbinder_proto::command_line::{
    Change, Consent, Permission, Query, Request, SkillFilter, WorkspaceName,
};

fn parse(line: &[&str], interactive: bool) -> Request {
    let matches = grammar()
        .try_get_matches_from(std::iter::once("skillbinder").chain(line.iter().copied()))
        .unwrap_or_else(|error| panic!("{line:?}: {error}"));
    request(&matches, interactive).unwrap_or_else(|error| panic!("{line:?}: {error:?}"))
}

fn consent(replace: Permission, confirm: Permission) -> Consent {
    Consent { replace, confirm }
}

#[test]
fn the_grammar_is_valid() {
    grammar().debug_assert();
}

#[test]
fn reads_carry_their_filters() {
    assert_eq!(
        parse(&["skills", "--updates", "--folder", "Docs"], false),
        Request::Read {
            query: Query::Skills {
                filter: SkillFilter::Updates,
                folder: Some("Docs".into()),
            }
        }
    );
    assert_eq!(
        parse(&["copies"], false),
        Request::Read {
            query: Query::Copies
        }
    );
    assert!(
        grammar()
            .try_get_matches_from(["skillbinder", "skills", "--active", "--inactive"])
            .is_err(),
        "one filter at a time"
    );
}

#[test]
fn a_terminal_asks_and_flags_answer_yes() {
    let activate = |line: &[&str], interactive| match parse(line, interactive) {
        Request::Change { change, consent } => {
            assert_eq!(
                change,
                Change::ActivatePreset {
                    preset: "Work".into()
                }
            );
            consent
        }
        other => panic!("{other:?} changes"),
    };
    let line = ["preset", "activate", "Work"];
    assert_eq!(
        activate(&line, false),
        consent(Permission::No, Permission::No)
    );
    assert_eq!(
        activate(&line, true),
        consent(Permission::Ask, Permission::Ask)
    );
    assert_eq!(
        activate(&["preset", "activate", "Work", "--replace"], false),
        consent(Permission::Yes, Permission::No),
        "--replace never confirms a deletion"
    );
    let Request::Change {
        consent: delete, ..
    } = parse(&["preset", "delete", "Work", "--yes"], false)
    else {
        panic!("delete changes");
    };
    assert_eq!(
        delete,
        consent(Permission::No, Permission::Yes),
        "--yes never replaces copies"
    );
}

#[test]
fn changes_carry_their_names() {
    let change = |line: &[&str]| match parse(line, false) {
        Request::Change { change, .. } => change,
        other => panic!("{other:?} changes"),
    };
    assert_eq!(
        change(&[
            "enable", "pdf", "sql", "--agent", "cursor", "--agent", "codex"
        ]),
        Change::Enable {
            skills: vec!["pdf".into(), "sql".into()],
            agents: vec!["cursor".into(), "codex".into()],
        }
    );
    assert_eq!(
        change(&["folder", "move", "pdf", "--none"]),
        Change::MoveToFolder {
            skills: vec!["pdf".into()],
            folder: None
        }
    );
    assert_eq!(
        change(&["copies", "keep", "pdf", "--in", "/home/.claude/skills"]),
        Change::KeepCopy {
            skill: "pdf".into(),
            directory: Some("/home/.claude/skills".into())
        }
    );
    assert!(
        grammar()
            .try_get_matches_from(["skillbinder", "folder", "move", "pdf"])
            .is_err(),
        "move needs --to or --none"
    );
}

#[test]
fn a_workspace_is_named_with_the_folder_it_names() -> anyhow::Result<()> {
    let change = |line: &[&str]| match parse(line, false) {
        Request::Change { change, .. } => change,
        other => panic!("{other:?} changes"),
    };
    let project = tempfile::tempdir()?;
    let inside = project.path().join("src");
    std::fs::create_dir(&inside)?;
    let typed = inside.display().to_string();
    // Links resolved, as the app stores workspace folders (macOS's /var is /private/var).
    let folder = dunce::canonicalize(&inside)?.display().to_string();
    assert_eq!(
        change(&[
            "workspace",
            "enable",
            &typed,
            "pdf",
            "--agent",
            "claude-code"
        ]),
        Change::EnableInWorkspace {
            workspace: WorkspaceName {
                name: typed.clone(),
                folder: Some(folder.clone()),
            },
            skills: vec!["pdf".into()],
            agents: vec!["claude-code".into()],
        }
    );
    assert_eq!(
        change(&["workspace", "add", &typed]),
        Change::AddWorkspace { folder }
    );
    assert_eq!(
        change(&["workspace", "remove", "no-such-folder-here", "--yes"]),
        Change::RemoveWorkspace {
            workspace: WorkspaceName {
                name: "no-such-folder-here".into(),
                folder: None,
            }
        },
        "a name that is no folder stays a name"
    );
    assert_eq!(
        change(&["agent", "on", "cursor", "--replace"]),
        Change::TurnOnAgent {
            agent: "cursor".into()
        }
    );
    Ok(())
}
