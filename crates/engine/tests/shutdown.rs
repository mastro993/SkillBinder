//! Quitting must not wait for synchronization work that is stuck on the network: the native quit
//! path drains the engine on the main thread, so an in-flight Git command would freeze the app
//! until its own timeout.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/fixture.rs"]
mod support;

use diesel::{prelude::*, sql_types::Text};
use skillbinder_proto::{ErrorCategory, RemoteConfig, SyncAction};
use std::{
    fs,
    net::TcpListener,
    path::Path,
    process::Command,
    sync::mpsc,
    time::{Duration, Instant},
};
use support::Fixture;

/// Runs Git in `dir` as the remote's owner would.
fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}");
}

/// Listens on a port whose peer accepts the connection and never answers, as an unreachable
/// remote does, and reports the port once a client connects.
fn silent_remote() -> (u16, mpsc::Receiver<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (accepted, connected) = mpsc::channel();
    std::thread::spawn(move || {
        if let Ok((connection, _)) = listener.accept() {
            let _ = accepted.send(());
            std::thread::sleep(Duration::from_secs(30));
            drop(connection);
        }
    });
    (port, connected)
}

/// Points the saved remote at `url` without connecting to it, as a remote that was reachable
/// when it was chosen and is stuck now.
fn redirect_remote(fixture: &Fixture, url: String) {
    let remote = serde_json::to_string(&RemoteConfig {
        url,
        branch: "main".into(),
    })
    .unwrap();
    diesel::sql_query(
        "INSERT INTO device_settings (key, value) VALUES ('remote', ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind::<Text, _>(remote)
    .execute(&mut fixture.database())
    .unwrap();
}

#[tokio::test]
async fn shutdown_stops_a_synchronization_waiting_on_the_network() {
    // An ssh remote holds a connection without a helper server; hosts without an ssh client
    // cannot run this check.
    if Command::new("ssh").arg("-V").output().is_err() {
        return;
    }
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    fixture.skill("alpha", "First version");
    fixture.register(&engine).await;
    let mut candidates = fixture.scan(&engine).await.candidates;
    fixture.import_one(&engine, &candidates.remove(0)).await;
    let bare = fixture.workspace.with_file_name("remote.git");
    fs::create_dir(&bare).unwrap();
    git(&bare, &["init", "--bare", "--initial-branch=main"]);
    let remote = RemoteConfig {
        url: url::Url::from_file_path(&bare).unwrap().into(),
        branch: "main".into(),
    };
    engine.connect_remote(remote).await.unwrap();
    let (port, connected) = silent_remote();
    redirect_remote(&fixture, format!("ssh://127.0.0.1:{port}/repo.git"));

    let syncing = tokio::spawn({
        let engine = engine.clone();
        async move { engine.synchronize(SyncAction::Refresh).await }
    });
    tokio::task::spawn_blocking(move || connected.recv_timeout(Duration::from_secs(10)))
        .await
        .unwrap()
        .unwrap();

    let started = Instant::now();
    let shutdown = tokio::time::timeout(Duration::from_secs(2), engine.shutdown()).await;
    assert!(
        shutdown.is_ok(),
        "shutdown waited {:?} for a synchronization stuck on the network",
        started.elapsed()
    );
    let cancelled = syncing.await.unwrap().unwrap_err();
    assert_eq!(cancelled.category, ErrorCategory::Cancelled);
}
