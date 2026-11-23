use super::{plan_reinstall, plan_update, plan_update_all, updated, updates};
use crate::commands::plan::{Plan, plan};
use crate::commands::run::refusal;
use crate::gate::Gate;
use crate::native_view::fixture::{Fixture, id};
use crate::native_view::{GIT_NEEDED, fixture::agents};
use skillbinder_proto::command_line::{Change, ErrorCode, Output, Query};
use skillbinder_proto::{AppSnapshot, Drift, SkillUpdate, UpdateCheck, UpdateOutcome};

/// pdf on for Claude Code with an update, sql with none, docx whose source couldn't be checked,
/// and Git.
fn world() -> Fixture {
    Fixture::new()
        .agent(
            "claude-code",
            "Claude Code",
            ".claude/skills",
            ".claude/skills",
            true,
        )
        .skill("docx", "", None)
        .skill("pdf", "Read PDFs.", None)
        .skill("sql", "", None)
        .hold("pdf", "default", agents(&["claude-code"]))
        .update("pdf", UpdateCheck::Available)
        .update("sql", UpdateCheck::UpToDate)
        .update(
            "docx",
            UpdateCheck::Failed("The source is unreachable.".into()),
        )
}

fn with_git(fixture: Fixture) -> AppSnapshot {
    let mut snapshot = fixture.build();
    snapshot.bootstrap.git_version = Some("2.45.0".into());
    snapshot
}

fn change(snapshot: &AppSnapshot, change: Change) -> Result<Plan, ErrorCode> {
    plan(&change, snapshot).map_err(|error| error.code)
}

#[test]
fn updates_lists_the_skills_with_an_update_and_those_unchecked() {
    let snapshot = world().build();
    let library = snapshot
        .library
        .as_ref()
        .map(|library| updates(&snapshot, library));
    let Some(Ok(output)) = library else {
        panic!("the world has a library");
    };
    let skills: Vec<(&str, bool, &[String])> = output
        .skills
        .iter()
        .map(|row| (row.slug.as_str(), row.update, row.agents.as_slice()))
        .collect();
    assert_eq!(skills, [("pdf", true, &["Claude Code".to_owned()][..])]);
    assert_eq!(output.unchecked.len(), 1);
    assert_eq!(output.unchecked[0].slug, "docx");
    assert_eq!(output.unchecked[0].reason, "The source is unreachable.");
    assert!(
        matches!(
            crate::commands::reads::answer(&Query::Updates, &snapshot),
            Ok(Output::Updates(_))
        ),
        "listing what the last check found needs no Git"
    );
}

#[test]
fn update_takes_the_named_skills_with_an_update() {
    let snapshot = with_git(world());
    let Ok(Plan::Update { skills, current }) = change(
        &snapshot,
        Change::Update {
            skills: vec!["PDF".into(), "sql".into()],
        },
    ) else {
        panic!("pdf has an update");
    };
    assert_eq!((skills, current), (vec![id("pdf")], vec![id("sql")]));
    let Ok(Plan::Save { saves, done }) = plan_update(
        &snapshot,
        snapshot.library.as_ref().unwrap_or_else(|| unreachable!()),
        &["sql".into(), "docx".into()],
    ) else {
        panic!("neither has an update");
    };
    assert!(saves.is_empty());
    assert_eq!(done, "sql, docx have no update.");
    let unknown = Change::Update {
        skills: vec!["nope".into()],
    };
    assert_eq!(change(&snapshot, unknown).err(), Some(ErrorCode::NotFound));
}

#[test]
fn update_all_runs_only_while_a_skill_has_an_update() {
    assert!(matches!(
        plan_update_all(&with_git(world())),
        Ok(Plan::UpdateAll)
    ));
    let Ok(Plan::Save { saves, done }) =
        plan_update_all(&with_git(world().update("pdf", UpdateCheck::UpToDate)))
    else {
        panic!("nothing to update");
    };
    assert!(saves.is_empty());
    assert_eq!(done, "No skill has an update.");
}

#[test]
fn a_second_update_all_meanwhile_is_busy() {
    let mut snapshot = with_git(world());
    snapshot.updating = Some((1, 3));
    let Some(refused) = refusal(&Gate::default(), &snapshot) else {
        panic!("Update all refuses changes while it runs");
    };
    assert_eq!(
        (refused.code.exit_code(), refused.message.as_str()),
        (4, "SkillBinder is busy: Updating 1 of 3 skills…")
    );
}

#[test]
fn reinstall_restores_only_a_restorable_drift() {
    let snapshot = with_git(
        world()
            .drift("pdf", Drift::Restorable)
            .drift("docx", Drift::Lost),
    );
    let library = snapshot.library.as_ref().unwrap_or_else(|| unreachable!());
    assert!(matches!(
        plan_reinstall(&snapshot, library, "pdf"),
        Ok(Plan::Reinstall(skill)) if skill == id("pdf")
    ));
    let Ok(Plan::Save { done, .. }) = plan_reinstall(&snapshot, library, "sql") else {
        panic!("sql has not drifted");
    };
    assert_eq!(done, "sql matches its record.");
    let Err(lost) = plan_reinstall(&snapshot, library, "docx") else {
        panic!("docx's recorded content is lost");
    };
    assert_eq!(
        (lost.code, lost.message.as_str()),
        (ErrorCode::Failed, "docx can't be reinstalled.")
    );
}

#[test]
fn updates_and_reinstall_without_git_fail_with_the_app_s_words() {
    let snapshot = world().drift("pdf", Drift::Restorable).build();
    for without in [
        Change::Update {
            skills: vec!["pdf".into()],
        },
        Change::UpdateAll,
        Change::Reinstall {
            skill: "pdf".into(),
        },
    ] {
        let Err(error) = plan(&without, &snapshot) else {
            panic!("{without:?} needs Git");
        };
        assert_eq!(
            (error.code, error.code.exit_code(), error.message.as_str()),
            (ErrorCode::GitMissing, 1, GIT_NEEDED),
            "{without:?}"
        );
    }
}

#[test]
fn a_failed_update_says_why_beside_what_landed() {
    let mut snapshot = world().build();
    snapshot.updates.insert(
        id("sql"),
        SkillUpdate {
            check: UpdateCheck::Available,
            failure: Some("The source no longer has this skill.".into()),
        },
    );
    let all = UpdateOutcome {
        updated: vec![id("pdf")],
        failed: Vec::new(),
    };
    let Ok(changed) = updated(&all, &[id("docx")], &snapshot) else {
        panic!("pdf updated");
    };
    assert_eq!(changed.message, "Updated pdf. docx has no update.");
    assert_eq!(changed.updated, ["pdf"]);
    let partly = UpdateOutcome {
        updated: vec![id("pdf")],
        failed: vec![id("sql"), id("gone")],
    };
    let Err(failure) = updated(&partly, &[], &snapshot) else {
        panic!("sql failed");
    };
    assert_eq!(
        (
            failure.error.code.exit_code(),
            failure.error.message.as_str()
        ),
        (1, "2 skills couldn't be updated")
    );
    assert_eq!(
        failure.error.hint.as_deref(),
        Some("sql: The source no longer has this skill.\ngone: it is no longer in the library.")
    );
    let Some(Output::Changed(landed)) = failure.result.as_deref() else {
        panic!("what landed comes back");
    };
    assert_eq!(landed.updated, ["pdf"]);
}
