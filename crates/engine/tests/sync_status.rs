//! The Sync page's status: which skills wait on each side, how, and why, and a check of the remote that
//! fails showing as Unreachable until one reaches it. Every remote is a local bare repository.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/device.rs"]
mod device;
#[path = "support/fixture.rs"]
mod support;

use device::Device;
use skillbinder_engine::Engine;
use skillbinder_proto::{
    ChangeKind, ChangedSkill, Reason, RemoteConfig, SyncAction, SyncSnapshot, SyncState,
};
use std::fs;
use support::{Fixture, write_skill};

async fn sync(engine: &Engine) -> SyncSnapshot {
    engine.snapshot().await.unwrap().sync
}

/// The change `kind` to `slug`, and why.
fn change(slug: &str, kind: ChangeKind, reason: Option<Reason>) -> ChangedSkill {
    ChangedSkill {
        slug: slug.to_owned(),
        kind,
        reason,
    }
}

fn imported() -> Option<Reason> {
    Some(Reason::Imported {
        from: Some("workspace".into()),
    })
}

/// Imports the workspace skill `slug` into the library of `device`, whose workspace is added.
async fn import(device: &Device, slug: &str) {
    device.fixture.skill(slug, "First version");
    let scan = device.fixture.scan(&device.engine).await;
    let candidate = scan.candidates.iter().find(|c| c.slug == slug).unwrap();
    device.fixture.import_one(&device.engine, candidate).await;
}

#[tokio::test]
async fn lists_the_skills_waiting_on_each_side_by_kind_and_why() {
    let device = Device::published(&["alpha", "beta", "gamma", "zeta"]).await;
    let published = sync(&device.engine).await;
    assert_eq!(published.state, SyncState::Synced);
    assert!(published.local.is_empty());
    assert_eq!(published.incoming, None);

    import(&device, "delta").await;
    let skills = device.library().join("skills");
    write_skill(&skills, "alpha", "Edited here");
    fs::remove_dir_all(skills.join("beta")).unwrap();
    device.peer_push_with(
        &[
            "Laptop: add epsilon, update gamma, remove zeta",
            "SkillBinder-Device: Laptop\n\
             SkillBinder-Skill: epsilon installed\n\
             SkillBinder-Skill: gamma updated",
        ],
        |skills| {
            write_skill(skills, "gamma", "Edited there");
            write_skill(skills, "epsilon", "Added there");
            fs::remove_dir_all(skills.join("zeta")).unwrap();
        },
    );
    let both = device
        .engine
        .synchronize(SyncAction::Refresh)
        .await
        .unwrap();
    assert_eq!(both.state, SyncState::NeedsSync);
    let local = vec![
        change("delta", ChangeKind::Added, imported()),
        change("alpha", ChangeKind::Changed, Some(Reason::Outside)),
        change("beta", ChangeKind::Removed, Some(Reason::Outside)),
    ];
    assert_eq!(both.local, local);
    let incoming = both.incoming.unwrap();
    assert_eq!(incoming.devices, ["Laptop"]);
    assert!(incoming.at.is_some());
    assert_eq!(
        incoming.skills,
        [
            change("epsilon", ChangeKind::Added, Some(Reason::Installed)),
            change("gamma", ChangeKind::Changed, Some(Reason::Updated)),
            // A commit without a trailer for it does not say why.
            change("zeta", ChangeKind::Removed, None),
        ]
    );
    // The snapshot the page reads lists the same.
    assert_eq!(sync(&device.engine).await.local, local);
}

#[tokio::test]
async fn an_empty_remote_lists_the_whole_library_on_this_device() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    device::import(&fixture, &engine, &["alpha", "beta"]).await;
    let bare = fixture.workspace.with_file_name("remote.git");
    fs::create_dir(&bare).unwrap();
    device::git(&bare, &["init", "--bare", "--initial-branch=main"]);
    let connected = engine
        .connect_remote(RemoteConfig {
            url: url::Url::from_file_path(&bare).unwrap().into(),
            branch: "main".into(),
        })
        .await
        .unwrap();
    let local = vec![
        change("alpha", ChangeKind::Added, imported()),
        change("beta", ChangeKind::Added, imported()),
    ];
    assert_eq!(connected.state, SyncState::NeedsPush);
    assert_eq!(connected.local, local);
    assert_eq!(connected.incoming, None);
    assert_eq!(sync(&engine).await.local, local);
}

#[tokio::test]
async fn a_push_that_failed_keeps_its_skills_listed_with_why() {
    let device = Device::published(&["alpha"]).await;
    import(&device, "delta").await;
    write_skill(&device.library().join("skills"), "alpha", "Edited here");
    // The remote refuses the push after the commit is made.
    let bare = device.fixture.workspace.with_file_name("remote.git");
    let hook = bare.join("hooks/pre-receive");
    fs::write(&hook, "#!/bin/sh\nexit 1\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    }

    device
        .engine
        .synchronize(SyncAction::Push)
        .await
        .unwrap_err();
    let unpushed = sync(&device.engine).await;
    assert_eq!(unpushed.state, SyncState::NeedsPush);
    assert!(!unpushed.uncommitted);
    assert_eq!(
        unpushed.local,
        [
            change("delta", ChangeKind::Added, imported()),
            change("alpha", ChangeKind::Changed, Some(Reason::Outside)),
        ]
    );
}

#[tokio::test]
async fn a_failed_check_shows_unreachable_until_a_check_reaches_the_remote() {
    let device = Device::published(&["alpha"]).await;
    let checked = device
        .engine
        .synchronize(SyncAction::Refresh)
        .await
        .unwrap();
    let bare = device.fixture.workspace.with_file_name("remote.git");
    let moved = bare.with_file_name("moved.git");
    fs::rename(&bare, &moved).unwrap();

    let error = device
        .engine
        .synchronize(SyncAction::Refresh)
        .await
        .unwrap_err();
    let unreachable = sync(&device.engine).await;
    assert_eq!(unreachable.state, SyncState::Unreachable);
    assert_eq!(
        unreachable.last_error.map(|e| e.message),
        Some(error.message)
    );
    assert_eq!(unreachable.refreshed_at, checked.refreshed_at);
    assert!(unreachable.remote.is_some());

    fs::rename(&moved, &bare).unwrap();
    device
        .engine
        .synchronize(SyncAction::Refresh)
        .await
        .unwrap();
    let reached = sync(&device.engine).await;
    assert_eq!(reached.state, SyncState::Synced);
    assert!(reached.last_error.is_none());
}
