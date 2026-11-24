use super::{grammar, request};
use skillbinder_proto::command_line::{
    Change, Consent, Find, Permission, Query, Request, SkillFilter, SyncKind, WorkspaceName,
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
    // An absolute directory is kept as given; on Windows that needs a drive.
    let directory = if cfg!(windows) {
        r"C:\home\.claude\skills"
    } else {
        "/home/.claude/skills"
    };
    assert_eq!(
        change(&["copies", "keep", "pdf", "--in", directory]),
        Change::KeepCopy {
            skill: "pdf".into(),
            directory: Some(directory.into())
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

#[test]
fn scan_import_explore_and_repositories_carry_their_arguments() {
    let change = |line: &[&str], interactive| match parse(line, interactive) {
        Request::Change { change, consent } => (change, consent),
        other => panic!("{other:?} changes"),
    };
    assert_eq!(parse(&["scan"], false), Request::Find { find: Find::Scan });
    assert_eq!(
        parse(&["search", "pdf", "forms"], false),
        Request::Find {
            find: Find::Search {
                query: "pdf forms".into()
            }
        }
    );
    assert_eq!(
        parse(&["repos"], false),
        Request::Read {
            query: Query::Repositories
        }
    );
    assert_eq!(
        change(&["import", "pdf", "sql@3f9a2c1", "--replace"], true),
        (
            Change::Import {
                skills: vec!["pdf".into(), "sql@3f9a2c1".into()],
                all: false,
            },
            consent(Permission::Yes, Permission::Ask)
        )
    );
    assert_eq!(
        change(&["import", "--all"], false).0,
        Change::Import {
            skills: Vec::new(),
            all: true,
        }
    );
    assert!(
        grammar()
            .try_get_matches_from(["skillbinder", "import"])
            .is_err(),
        "import names skills or --all"
    );
    assert_eq!(
        change(
            &["install", "anthropics/skills/pdf", "acme/tools@lint"],
            false
        )
        .0,
        Change::Install {
            skills: vec!["anthropics/skills/pdf".into(), "acme/tools@lint".into()],
        }
    );
    let url = "https://github.com/acme/tools";
    assert_eq!(
        change(&["repo", "add", url], false).0,
        Change::AddRepository { url: url.into() }
    );
    assert_eq!(
        change(&["repo", "install-all", "acme/tools"], false).0,
        Change::InstallAll {
            repository: "acme/tools".into()
        }
    );
    assert_eq!(
        change(&["repo", "remove", url], false).0,
        Change::RemoveRepository {
            repository: url.into()
        }
    );
}

#[test]
fn updates_and_sync_read_into_their_requests() {
    let change = |line: &[&str]| match parse(line, false) {
        Request::Change { change, .. } => change,
        other => panic!("{other:?} changes"),
    };
    assert_eq!(
        parse(&["updates"], false),
        Request::Read {
            query: Query::Updates
        }
    );
    assert_eq!(
        parse(&["updates", "--check"], false),
        Request::Find { find: Find::Check }
    );
    assert_eq!(
        change(&["update", "pdf", "sql"]),
        Change::Update {
            skills: vec!["pdf".into(), "sql".into()]
        }
    );
    assert_eq!(change(&["update", "--all"]), Change::UpdateAll);
    assert!(
        grammar()
            .try_get_matches_from(["skillbinder", "update", "pdf", "--all"])
            .is_err(),
        "update names skills or --all, not both"
    );
    assert_eq!(
        change(&["reinstall", "pdf"]),
        Change::Reinstall {
            skill: "pdf".into()
        }
    );
    for (line, action) in [
        ("sync", SyncKind::Sync),
        ("push", SyncKind::Push),
        ("pull", SyncKind::Pull),
    ] {
        assert_eq!(change(&[line]), Change::Synchronize { action });
    }
    let url = "git@github.com:me/skills.git";
    assert_eq!(
        change(&["remote", "set", url]),
        Change::SetRemote {
            url: url.into(),
            branch: "main".into()
        },
        "the branch defaults to main, as the Sync page's does"
    );
    assert_eq!(
        change(&["remote", "set", url, "--branch", "library"]),
        Change::SetRemote {
            url: url.into(),
            branch: "library".into()
        }
    );
    assert_eq!(change(&["remote", "remove"]), Change::RemoveRemote);
}
