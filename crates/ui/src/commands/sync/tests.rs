use super::{plan, plan_remove, plan_set};
use crate::commands::plan::Plan;
use crate::commands::run::refusal;
use crate::gate::Gate;
use crate::native_view::GIT_NEEDED;
use crate::native_view::fixture::Fixture;
use skillbinder_proto::command_line::{ErrorCode, SyncKind};
use skillbinder_proto::{AppSnapshot, RemoteConfig, SyncAction};

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
