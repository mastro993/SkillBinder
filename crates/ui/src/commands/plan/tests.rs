use super::{Plan, Save, plan};
use crate::native_view::fixture::{Fixture, agents, id, preset_id};
use skillbinder_proto::command_line::{Change, ErrorCode};
use skillbinder_proto::{
    AppSnapshot, CopyChoice, CopyState, ManagedCopy, OrganizationChange, PresetSwitch, Targets,
};

/// Two of your agents, a Work preset, and pdf held by Default for Claude Code alone.
fn world() -> Fixture {
    Fixture::new()
        .agent(
            "claude-code",
            "Claude Code",
            ".claude/skills",
            ".claude/skills",
            true,
        )
        .agent("cursor", "Cursor", ".cursor/skills", ".cursor/skills", true)
        .preset("work", "Work")
        .folder("docs", "Documents")
        .skill("pdf", "", None)
        .skill("sql", "", None)
        .hold("pdf", "default", agents(&["claude-code"]))
        .hold("sql", "work", Targets::AllAgents)
}

fn changes(change: Change, snapshot: &AppSnapshot) -> Vec<String> {
    match plan(&change, snapshot) {
        Ok(Plan::Save { saves, .. }) => saves
            .iter()
            .map(|save| match save {
                Save::Organize(change) => format!("{change:?}"),
                other => panic!("{other:?} organizes"),
            })
            .collect(),
        other => panic!("{change:?} organizes, not {other:?}"),
    }
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn enable_holds_for_all_agents_or_adds_the_named_ones() {
    let snapshot = world().build();
    let enable = |skills: &[&str], agents: &[&str]| Change::Enable {
        skills: names(skills),
        agents: names(agents),
    };
    assert_eq!(
        changes(enable(&["PDF", "sql"], &[]), &snapshot),
        [
            format!(
                "{:?}",
                OrganizationChange::AddToPreset {
                    id: preset_id("default"),
                    skills: vec![id("sql")]
                }
            ),
            format!(
                "{:?}",
                OrganizationChange::SetTargets {
                    id: preset_id("default"),
                    skill: id("pdf"),
                    targets: Targets::AllAgents
                }
            ),
        ]
    );
    assert_eq!(
        changes(enable(&["pdf"], &["cursor"]), &snapshot),
        [format!(
            "{:?}",
            OrganizationChange::SetTargets {
                id: preset_id("default"),
                skill: id("pdf"),
                targets: agents(&["claude-code", "cursor"]),
            }
        )]
    );
    assert!(
        changes(enable(&["pdf"], &["Claude Code"]), &snapshot).is_empty(),
        "already held"
    );
}

#[test]
fn disable_removes_or_keeps_the_other_agents() {
    let snapshot = world().active("work").build();
    let disable = |agents: &[&str]| Change::Disable {
        skills: names(&["sql", "pdf"]),
        agents: names(agents),
    };
    assert_eq!(
        changes(disable(&[]), &snapshot),
        [format!(
            "{:?}",
            OrganizationChange::RemoveFromPreset {
                id: preset_id("work"),
                skills: vec![id("sql")]
            }
        )],
        "pdf isn't in Work"
    );
    assert_eq!(
        changes(disable(&["cursor"]), &snapshot),
        [format!(
            "{:?}",
            OrganizationChange::SetTargets {
                id: preset_id("work"),
                skill: id("sql"),
                targets: agents(&["claude-code"]),
            }
        )]
    );
}

#[test]
fn activating_names_what_the_switch_adds_and_removes() {
    let snapshot = world().build();
    let activate = |preset: &str| Change::ActivatePreset {
        preset: preset.to_owned(),
    };
    let Ok(Plan::Switch {
        id: target,
        name,
        switch,
    }) = plan(&activate("work"), &snapshot)
    else {
        panic!("Work isn't active");
    };
    assert_eq!((target, name.as_str()), (preset_id("work"), "Work"));
    assert_eq!(
        switch,
        PresetSwitch {
            adds: vec![id("sql")],
            removes: vec![id("pdf")]
        }
    );
    let Ok(Plan::Save { saves, done }) = plan(&activate("default"), &snapshot) else {
        panic!("Default is active");
    };
    assert!(saves.is_empty());
    assert_eq!(done, "Default is already active.");
}

#[test]
fn an_unknown_name_changes_nothing() {
    let snapshot = world().build();
    let refused = [
        Change::Enable {
            skills: names(&["pdf", "docx"]),
            agents: Vec::new(),
        },
        Change::Enable {
            skills: names(&["pdf"]),
            agents: names(&["windsurf"]),
        },
        Change::DeletePreset {
            preset: "Play".into(),
        },
        Change::MoveToFolder {
            skills: names(&["pdf"]),
            folder: Some("Notes".into()),
        },
    ];
    for change in refused {
        let Err(error) = plan(&change, &snapshot) else {
            panic!("{change:?} names something unknown");
        };
        assert_eq!(error.code, ErrorCode::NotFound, "{change:?}");
    }
}

#[test]
fn deleting_a_folder_confirms_with_the_window_s_text() {
    let snapshot = world().skill("docx", "", Some("docs")).build();
    let Ok(Plan::Confirm { prompt, .. }) = plan(
        &Change::DeleteFolder {
            folder: "documents".into(),
        },
        &snapshot,
    ) else {
        panic!("Documents exists");
    };
    assert_eq!(
        prompt,
        "Delete Documents? Its 1 skills stay in the library."
    );
}

#[test]
fn a_copy_waiting_in_two_directories_needs_one_named() {
    let copy = |directory: &str, state| ManagedCopy {
        skill: id("pdf"),
        directory: format!("/home/{directory}"),
        state,
    };
    let mut snapshot = world().build();
    if let Some(library) = snapshot.library.as_mut() {
        library.copies = vec![
            copy(".claude/skills", CopyState::Conflict),
            copy(".cursor/skills", CopyState::Edited { active: false }),
        ];
    }
    let answer = |directory: Option<&str>| Change::ReplaceCopy {
        skill: "pdf".into(),
        directory: directory.map(str::to_owned),
    };
    let Err(error) = plan(&answer(None), &snapshot) else {
        panic!("pdf waits twice");
    };
    assert_eq!(error.code, ErrorCode::Ambiguous);
    assert_eq!(
        error.message,
        "pdf waits in ~/.claude/skills, ~/.cursor/skills."
    );
    let Ok(Plan::Copy { copy, choice, done }) = plan(&answer(Some("~/.cursor/skills/")), &snapshot)
    else {
        panic!("one directory is named");
    };
    assert_eq!(
        (copy.directory.as_str(), choice),
        ("/home/.cursor/skills", CopyChoice::Replace)
    );
    assert_eq!(done, "Removed your copy of pdf from ~/.cursor/skills.");
    let Err(error) = plan(&answer(Some("~/.gemini/skills")), &snapshot) else {
        panic!("pdf doesn't wait there");
    };
    assert_eq!(error.code, ErrorCode::NotFound);
    assert_eq!(error.message, "No copy of pdf waits in ~/.gemini/skills.");
}

#[test]
fn ignoring_saves_every_slug_at_once_and_unignoring_stops() {
    let snapshot = world().build();
    for (stop, done) in [
        (false, "Ignored storybook, tdd."),
        (true, "Stopped ignoring storybook, tdd."),
    ] {
        let change = Change::Ignore {
            skills: names(&["storybook", "tdd"]),
            stop,
        };
        let Ok(Plan::Save { saves, done: said }) = plan(&change, &snapshot) else {
            panic!("{change:?} saves");
        };
        let [Save::Ignore { skills, ignored }] = saves.as_slice() else {
            panic!("one save ignores them all: {saves:?}");
        };
        assert_eq!(
            (skills.as_slice(), *ignored),
            (&names(&["storybook", "tdd"])[..], !stop)
        );
        assert_eq!(said, done);
    }
}
