use super::{failure, output};
use skillbinder_proto::SyncState;
use skillbinder_proto::command_line::{
    Changed, CommandError, CopyRow, CopyStatus, ErrorCode, Failure, Output, PresetRow, SkillRow,
    StatusOutput,
};

fn status() -> StatusOutput {
    StatusOutput {
        version: "0.1.0".into(),
        onboarding_finished: true,
        active_preset: Some("Work".into()),
        skills: 42,
        active: 12,
        updates: 3,
        attention: 1,
        sync: SyncState::NeedsPush,
        running: Some("Switching to Work".into()),
    }
}

#[test]
fn status_prints_one_fact_per_line() {
    assert_eq!(
        output(&Output::Status(status())),
        "SkillBinder 0.1.0\nActive preset: Work\nLibrary: 42 skills, 12 active, 3 with updates\nCopies needing your answer: 1\nSync: needs push\nRunning: Switching to Work…\n"
    );
}

#[test]
fn before_setup_status_says_to_finish_it() {
    let status = StatusOutput {
        onboarding_finished: false,
        ..status()
    };
    assert_eq!(
        output(&Output::Status(status)),
        "SkillBinder 0.1.0\nSetup isn't finished. Open SkillBinder to finish setting up.\n"
    );
}

#[test]
fn skills_print_their_marks_and_places() {
    let row = |slug: &str, agents: &[&str], update: bool| SkillRow {
        slug: slug.into(),
        description: String::new(),
        active: !agents.is_empty(),
        agents: agents.iter().map(|agent| (*agent).to_owned()).collect(),
        workspaces: Vec::new(),
        folder: None,
        update,
        drifted: false,
        new: update,
    };
    assert_eq!(
        output(&Output::Skills(vec![
            row("pdf", &["Claude Code", "Cursor"], false),
            row("sql", &[], true)
        ])),
        "pdf: on for Claude Code, Cursor\nsql [update, new]: off\n"
    );
    assert_eq!(output(&Output::Skills(Vec::new())), "No skills.\n");
    let preset = |name: &str, active, skills| PresetRow {
        name: name.into(),
        active,
        skills,
    };
    assert_eq!(
        output(&Output::Presets(vec![
            preset("Default", false, 1),
            preset("Work", true, 3)
        ])),
        "  Default (1 skill)\n* Work (3 skills)\n"
    );
}

#[test]
fn a_switch_that_left_a_copy_waiting_prints_as_the_design_shows() {
    let changed = Changed {
        message: "Work is active: 3 added, 5 removed.".into(),
        waiting: vec![CopyRow {
            skill: "pdf".into(),
            directory: "~/.claude/skills".into(),
            state: CopyStatus::Conflict,
            reason: None,
        }],
        ..Changed::default()
    };
    let failed = Failure {
        error: CommandError::new(ErrorCode::NeedsAnswer, "1 copy needs your answer")
            .hint("Run `skillbinder copies replace pdf --in ~/.claude/skills`, or review it in SkillBinder."),
        result: Some(Box::new(Output::Changed(changed.clone()))),
    };
    assert_eq!(
        output(&Output::Changed(changed)),
        "Work is active: 3 added, 5 removed.\n"
    );
    assert_eq!(
        failure(&failed),
        "1 copy needs your answer:\n  pdf in ~/.claude/skills differs from the library.\nRun `skillbinder copies replace pdf --in ~/.claude/skills`, or review it in SkillBinder.\n"
    );
}
