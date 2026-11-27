//! The Sync page's status: how many skills wait on each side, and a check of the remote that
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
use skillbinder_proto::{SyncAction, SyncSnapshot, SyncState};
use std::fs;
use support::write_skill;

async fn sync(engine: &Engine) -> SyncSnapshot {
    engine.snapshot().await.unwrap().sync
}

#[tokio::test]
async fn counts_the_skills_waiting_on_each_side() {
    let device = Device::published(&["alpha", "beta"]).await;
    let published = sync(&device.engine).await;
    assert_eq!(published.state, SyncState::Synced);
    assert_eq!((published.local_skills, published.incoming_skills), (0, 0));

    write_skill(&device.library().join("skills"), "alpha", "Edited here");
    device.peer_push(|skills| {
        write_skill(skills, "beta", "Edited there");
        write_skill(skills, "gamma", "Added there");
    });
    let both = device
        .engine
        .synchronize(SyncAction::Refresh)
        .await
        .unwrap();
    assert_eq!(both.state, SyncState::NeedsSync);
    assert_eq!((both.local_skills, both.incoming_skills), (1, 2));
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
