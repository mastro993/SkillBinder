use super::reads::{history_row, page_size};
use super::{
    plan, plan_change, plan_discard, plan_remove, plan_resolve, plan_restore, plan_set, status,
};
use crate::commands::plan::Plan;
use crate::commands::run::refusal;
use crate::gate::Gate;
use crate::native_view::GIT_NEEDED;
use crate::native_view::fixture::Fixture;
use chrono::{TimeZone, Utc};
use skillbinder_proto::command_line::{CommandError, ErrorCode, SyncCommand, SyncKind};
use skillbinder_proto::{
    AppSnapshot, ChangeKind, ChangedSkill, Choice, Conflict, ConflictSide, Reason, RemoteConfig,
    Side, SkillId, SyncAction, SyncChange, SyncState,
};
use std::collections::BTreeMap;

fn remote() -> RemoteConfig {
    RemoteConfig {
        url: "file:///remotes/library.git".into(),
        branch: "main".into(),
    }
}

/// Git, and the remote when `connected`.
fn world(connected: bool) -> AppSnapshot {
    let mut snapshot = Fixture::new().build();
    snapshot.bootstrap.git_version = Some("2.45.0".into());
    snapshot.sync.remote = connected.then(remote);
    snapshot
}

#[test]
fn sync_push_and_pull_run_against_the_remote() {
    let snapshot = world(true);
    for (kind, action) in [
        (SyncKind::Sync, SyncAction::Sync),
        (SyncKind::Push, SyncAction::Push),
        (SyncKind::Pull, SyncAction::Pull),
    ] {
        assert!(
            matches!(plan(&snapshot, kind), Ok(Plan::Sync(planned)) if planned == action),
            "{kind:?}"
        );
    }
    let Err(error) = plan(&world(false), SyncKind::Sync) else {
        panic!("there is no remote");
    };
    assert_eq!(
        (error.code.exit_code(), error.message.as_str()),
        (2, "No sync remote is set.")
    );
}

#[test]
fn without_git_sync_fails_with_the_app_s_words() {
    let mut snapshot = world(true);
    snapshot.bootstrap.git_version = None;
    let failures = [
        plan(&snapshot, SyncKind::Sync).err(),
        plan_set(&snapshot, "file:///remotes/library.git", "main").err(),
        plan_remove(&snapshot).err(),
        page_size(&snapshot, None).err(),
        plan_restore(&snapshot, "pdf", "3f2a9c1b").err(),
        plan_discard(&snapshot, "pdf").err(),
        plan_resolve(&snapshot, &BTreeMap::new()).err(),
    ];
    for error in failures {
        let Some(error) = error else {
            panic!("each needs Git");
        };
        assert_eq!(
            (error.code, error.code.exit_code(), error.message.as_str()),
            (ErrorCode::GitMissing, 1, GIT_NEEDED)
        );
    }
}

#[test]
fn the_remote_is_set_and_removed() {
    assert!(matches!(
        plan_set(&world(false), "file:///remotes/library.git", "main"),
        Ok(Plan::Connect(set)) if set == remote()
    ));
    assert!(matches!(
        plan_remove(&world(true)),
        Ok(Plan::Sync(SyncAction::Disconnect))
    ));
    let Ok(Plan::Save { saves, done }) = plan_remove(&world(false)) else {
        panic!("there is nothing to remove");
    };
    assert!(saves.is_empty());
    assert_eq!(done, "No sync remote is set.");
}

#[test]
fn a_running_push_is_named_when_it_refuses_a_change() {
    let mut snapshot = world(true);
    snapshot.syncing = Some(SyncAction::Push);
    // The window and the command hold the gate while they sync.
    let gate = Gate {
        busy: true,
        ..Gate::default()
    };
    let Some(refused) = refusal(&gate, &snapshot) else {
        panic!("a push refuses changes");
    };
    assert_eq!(refused.message, "SkillBinder is busy: Pushing…");
}

fn side(kind: ChangeKind, device: Option<&str>) -> ConflictSide {
    ConflictSide {
        kind,
        reason: Some(Reason::Updated),
        device: device.map(str::to_owned),
        at: None,
        version: String::new(),
    }
}

fn changed(slug: &str, kind: ChangeKind, reason: Option<Reason>) -> ChangedSkill {
    ChangedSkill {
        slug: slug.into(),
        kind,
        reason,
    }
}

/// Connected, with pdf and sql changed on both sides and docx changed here only.
fn conflicted() -> AppSnapshot {
    let mut snapshot = world(true);
    let sync = &mut snapshot.sync;
    sync.state = SyncState::NeedsResolve;
    sync.device = "MacBook".into();
    sync.local = vec![
        changed("docx", ChangeKind::Added, Some(Reason::Installed)),
        changed("pdf", ChangeKind::Changed, Some(Reason::Updated)),
    ];
    sync.conflicts = ["pdf", "sql"]
        .map(|slug| Conflict {
            slug: slug.into(),
            local: side(ChangeKind::Changed, None),
            remote: side(ChangeKind::Removed, Some("Mac Studio")),
        })
        .to_vec();
    snapshot
}

fn refused(planned: Result<Plan, CommandError>) -> (ErrorCode, u8, String) {
    match planned {
        Err(error) => (error.code, error.code.exit_code(), error.message),
        Ok(plan) => panic!("{plan:?} is refused"),
    }
}

#[test]
fn sync_during_a_conflict_exits_2_and_names_sync_resolve() {
    let mut snapshot = conflicted();
    assert_eq!(
        refused(plan(&snapshot, SyncKind::Sync)),
        (
            ErrorCode::Usage,
            2,
            "2 skills changed on both sides. Resolve them with `skillbinder sync resolve`.".into()
        )
    );
    snapshot.sync.conflicts.truncate(1);
    assert_eq!(
        refused(plan(&snapshot, SyncKind::Sync)).2,
        "1 skill changed on both sides. Resolve them with `skillbinder sync resolve`."
    );
}

#[test]
fn resolve_needs_a_version_of_exactly_the_skills_changed_on_both_sides() {
    let snapshot = conflicted();
    let choices = |pairs: &[(&str, Side)]| -> BTreeMap<String, Side> {
        pairs
            .iter()
            .map(|(slug, side)| ((*slug).to_owned(), *side))
            .collect()
    };
    let Ok(Plan::Resolve(chosen)) = plan_resolve(
        &snapshot,
        &choices(&[("PDF", Side::Local), ("sql", Side::Remote)]),
    ) else {
        panic!("every conflict has a choice");
    };
    assert_eq!(
        chosen,
        choices(&[("pdf", Side::Local), ("sql", Side::Remote)]),
        "names compare ignoring case"
    );
    assert_eq!(
        refused(plan_resolve(&snapshot, &choices(&[("pdf", Side::Local)]))),
        (ErrorCode::Usage, 2, "Choose a version of sql too.".into())
    );
    assert_eq!(
        refused(plan_resolve(
            &snapshot,
            &choices(&[
                ("pdf", Side::Local),
                ("sql", Side::Local),
                ("docx", Side::Local)
            ])
        )),
        (
            ErrorCode::NotFound,
            2,
            "No skill changed on both sides is named \"docx\".".into()
        )
    );
    assert_eq!(
        refused(plan_resolve(
            &world(true),
            &choices(&[("pdf", Side::Local)])
        )),
        (
            ErrorCode::Usage,
            2,
            "No skill changed on both sides.".into()
        )
    );
}

#[test]
fn discard_takes_a_skill_not_synced_yet_and_restore_any_commit() {
    let snapshot = conflicted();
    assert!(matches!(
        plan_discard(&snapshot, "Docx"),
        Ok(Plan::Discard(skill)) if skill == SkillId("docx".into())
    ));
    let Err(error) = plan_discard(&snapshot, "pd") else {
        panic!("pd has no change");
    };
    assert_eq!(
        (
            error.code.exit_code(),
            error.message.as_str(),
            error.hint.as_deref()
        ),
        (
            2,
            "No change to pd waits to sync.",
            Some("Did you mean pdf?")
        )
    );
    assert!(matches!(
        plan_restore(&snapshot, "pdf", " 3f2a9c1b "),
        Ok(Plan::Restore { skill, commit }) if skill == SkillId("pdf".into()) && commit == "3f2a9c1b"
    ));
}

#[test]
fn renaming_this_device_needs_no_git() {
    let mut snapshot = world(false);
    snapshot.bootstrap.git_version = None;
    let rename = SyncCommand::RenameDevice {
        name: "Studio".into(),
    };
    assert!(matches!(
        plan_change(&snapshot, &rename),
        Ok(Plan::RenameDevice(name)) if name == "Studio"
    ));
}

#[test]
fn status_and_history_say_why_in_the_page_s_words() {
    let status = status(&conflicted());
    assert_eq!(status.device, "MacBook");
    assert_eq!(
        status.local[0].why.as_deref(),
        Some("Installed from Explore")
    );
    let theirs = &status.conflicts[0].theirs;
    assert_eq!(
        (theirs.why.as_deref(), theirs.device.as_deref()),
        (Some("Updated from its source"), Some("Mac Studio"))
    );
    assert!(status.incoming.is_empty() && status.devices.is_empty());
    let at = Utc
        .with_ymd_and_hms(2026, 11, 26, 9, 15, 0)
        .single()
        .unwrap_or_default();
    let merge = SyncChange {
        commit: "3f2a9c1b".into(),
        at: at.timestamp().unsigned_abs(),
        device: Some("Mac Studio".into()),
        skills: vec![changed("pdf", ChangeKind::Removed, Some(Reason::Outside))],
        choices: vec![Choice {
            slug: "sql".into(),
            side: Side::Remote,
            device: None,
            unpicked: Some("aa11bb22".into()),
        }],
    };
    let row = history_row(&merge, &(at + chrono::Duration::hours(1)));
    assert_eq!(row.when, "Today, 09:15");
    assert_eq!(
        row.skills[0].why.as_deref(),
        Some("Removed outside SkillBinder")
    );
    assert_eq!(row.choices[0].kept, "Kept the remote's sql");
    assert_eq!(row.choices[0].unpicked.as_deref(), Some("aa11bb22"));
    assert_eq!(
        page_size(&conflicted(), Some(0)).map_err(|error| error.code),
        Err(ErrorCode::Usage)
    );
    assert_eq!(page_size(&conflicted(), None).ok(), Some(50));
}
