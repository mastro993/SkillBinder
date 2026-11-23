use super::{answer, running, status};
use crate::commands::run::{interrupted, refusal};
use crate::gate::{Change, ChangeState, Gate};
use crate::native_view::Activity;
use crate::native_view::fixture::{Fixture, agents, id};
use skillbinder_proto::command_line::{
    CopyStatus, ErrorCode, Output, Query, SkillFilter, SkillOutput,
};
use skillbinder_proto::{CopyState, ManagedCopy, Targets, UpdateCheck};

fn switching() -> Gate {
    Gate {
        change: Some(Change {
            activity: Activity::Switching {
                preset: "Work".into(),
            },
            state: ChangeState::Running,
        }),
        busy: false,
        ..Gate::default()
    }
}

/// Claude Code, Work active, pdf on for Claude Code and filed in Documents, sql off with an update.
fn world() -> Fixture {
    Fixture::new()
        .agent(
            "claude-code",
            "Claude Code",
            ".claude/skills",
            ".claude/skills",
            true,
        )
        .preset("work", "Work")
        .active("work")
        .folder("docs", "Documents")
        .skill("pdf", "Read PDFs.", Some("docs"))
        .skill("sql", "Write SQL.", None)
        .hold("pdf", "work", agents(&["claude-code"]))
        .hold("pdf", "default", Targets::AllAgents)
        .update("sql", UpdateCheck::Available)
}

#[test]
fn status_answers_before_onboarding_finishes() {
    let mut snapshot = Fixture::new().build();
    snapshot.bootstrap.onboarding.finished = false;
    assert!(!status(&snapshot, &Gate::default()).onboarding_finished);
}

#[test]
fn status_counts_the_library_under_the_active_preset() {
    let status = status(&world().build(), &Gate::default());
    assert_eq!(status.active_preset.as_deref(), Some("Work"));
    assert_eq!((status.skills, status.active, status.updates), (2, 1, 1));
}

#[test]
fn a_running_switch_is_named_and_refuses_changes_but_not_reads() {
    let snapshot = world().build();
    let gate = switching();
    assert_eq!(
        status(&snapshot, &gate).running.as_deref(),
        Some("Switching to Work")
    );
    assert_eq!(running(&Gate::default(), &snapshot), None);
    let Some(refused) = refusal(&gate, &snapshot) else {
        panic!("a switch refuses a change");
    };
    assert_eq!(
        (refused.code, refused.code.exit_code()),
        (ErrorCode::Busy, 4)
    );
    assert_eq!(refused.message, "SkillBinder is busy: Switching to Work…");
    assert_eq!(refused.hint.as_deref(), Some("Try again when it finishes."));
    assert!(refusal(&Gate::default(), &snapshot).is_none());
    assert!(
        answer(&Query::Presets, &snapshot).is_ok(),
        "reads never look at the gate"
    );
}

#[test]
fn skills_lists_library_s_filters_and_folders() {
    let snapshot = world().build();
    let slugs = |filter, folder: Option<&str>| match answer(
        &Query::Skills {
            filter,
            folder: folder.map(str::to_owned),
        },
        &snapshot,
    ) {
        Ok(Output::Skills(rows)) => rows.into_iter().map(|row| row.slug).collect::<Vec<_>>(),
        other => panic!("skills lists skills, not {other:?}"),
    };
    assert_eq!(slugs(SkillFilter::All, None), ["pdf", "sql"]);
    assert_eq!(slugs(SkillFilter::Active, None), ["pdf"]);
    assert_eq!(slugs(SkillFilter::Inactive, None), ["sql"]);
    assert_eq!(slugs(SkillFilter::Updates, None), ["sql"]);
    assert_eq!(slugs(SkillFilter::All, Some("documents")), ["pdf"]);
}

#[test]
fn a_skill_page_names_its_places_presets_and_copies() {
    let mut snapshot = world().build();
    if let Some(library) = snapshot.library.as_mut() {
        library.copies = vec![ManagedCopy {
            skill: id("pdf"),
            directory: "/home/.claude/skills".into(),
            state: CopyState::Conflict,
        }];
    }
    let Ok(Output::Skill(page)) = answer(
        &Query::Skill {
            skill: "PDF".into(),
        },
        &snapshot,
    ) else {
        panic!("pdf has a page");
    };
    let SkillOutput {
        skill,
        presets,
        copies,
        ..
    } = *page;
    assert_eq!(skill.agents, ["Claude Code"]);
    assert_eq!(skill.folder.as_deref(), Some("Documents"));
    assert_eq!(presets, ["Default", "Work"]);
    assert_eq!(copies[0].directory, "~/.claude/skills");
    assert_eq!(copies[0].state, CopyStatus::Conflict);
    let Ok(Output::Copies(waiting)) = answer(&Query::Copies, &snapshot) else {
        panic!("copies lists what waits");
    };
    assert_eq!(waiting.len(), 1);
}

#[test]
fn presets_mark_the_active_one() {
    let Ok(Output::Presets(presets)) = answer(&Query::Presets, &world().build()) else {
        panic!("presets lists presets");
    };
    let rows: Vec<_> = presets
        .iter()
        .map(|preset| (preset.name.as_str(), preset.active, preset.skills))
        .collect();
    assert_eq!(rows, [("Default", false, 1), ("Work", true, 1)]);
}

#[test]
fn a_command_s_own_change_refuses_another() {
    let snapshot = world().build();
    let gate = Gate {
        commands: 1,
        ..Gate::default()
    };
    assert!(gate.blocked(&snapshot), "the window refuses too");
    let refused = refusal(&gate, &snapshot).map(|error| error.message);
    assert_eq!(refused.as_deref(), Some("SkillBinder is busy."));
}

#[test]
fn replacing_after_a_question_is_refused_only_once_something_else_started() {
    let snapshot = world().build();
    let landed = Gate {
        change: Some(Change {
            activity: Activity::Switching {
                preset: "Work".into(),
            },
            state: ChangeState::Landed(crate::native_view::Landing::default()),
        }),
        ..Gate::default()
    };
    assert!(
        interrupted(&landed, &snapshot).is_none(),
        "the command's own switch, whose Done pill still shows"
    );
    let refused = interrupted(&switching(), &snapshot).map(|error| error.message);
    assert_eq!(
        refused.as_deref(),
        Some("SkillBinder is busy: Switching to Work…")
    );
}
