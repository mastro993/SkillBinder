use super::*;
use crate::tests::ready;
use std::path::Path;
use tokio::runtime::Handle;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A `file:` URL of `path`, which Git fetches as a remote.
fn file_url(path: &Path) -> String {
    let path = path.to_string_lossy().replace('\\', "/");
    if path.starts_with('/') {
        format!("file://{path}")
    } else {
        format!("file:///{path}")
    }
}

/// Waits up to a minute for the client's snapshot to satisfy `done`.
async fn until(client: &Client, done: impl Fn(&AppSnapshot) -> bool) -> AppSnapshot {
    let mut snapshots = client.subscribe();
    let wait = snapshots.wait_for(|snapshot| done(snapshot));
    let shown = tokio::time::timeout(Duration::from_secs(60), wait).await;
    let snapshot = (*client.snapshot()).clone();
    assert!(
        shown.is_ok_and(|shown| shown.is_ok()),
        "{:?}",
        snapshot.sync
    );
    snapshot
}

#[tokio::test(flavor = "multi_thread")]
async fn checks_the_remote_every_period_and_shows_a_failed_check_as_unreachable() -> TestResult {
    // The remote is another library's repository, in a folder of its own that goes away later.
    let remote_folder = tempfile::tempdir()?;
    let remote = ready(remote_folder.path()).await?;
    let directory = tempfile::tempdir()?;
    let engine = ready(directory.path()).await?;
    engine
        .connect_remote(RemoteConfig {
            url: file_url(&remote_folder.path().join("data/library")),
            branch: "main".to_owned(),
        })
        .await?;
    let client = Client::new(engine.clone(), Handle::current()).await?;
    let first = client.snapshot().sync.refreshed_at;
    client.start_sync_checks(Duration::from_millis(200));

    // Checked seconds apply, so a later second shows a check after the first.
    let checked = until(&client, |s| s.sync.refreshed_at > first).await;
    assert_eq!(checked.sync.state, SyncState::NeedsPush);

    remote.shutdown().await?;
    drop(remote_folder);
    let unreachable = until(&client, |s| s.sync.state == SyncState::Unreachable).await;
    assert!(unreachable.sync.last_error.is_some());
    assert!(unreachable.background_error.is_none(), "no toast");
    engine.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn a_check_waits_while_blocking_work_runs() -> TestResult {
    let directory = tempfile::tempdir()?;
    let engine = ready(directory.path()).await?;
    let mut idle = engine.snapshot().await?;
    assert!(!due(&idle), "no remote");
    idle.sync.remote = Some(RemoteConfig {
        url: "git@github.com:ada/skills.git".to_owned(),
        branch: "main".to_owned(),
    });
    assert!(due(&idle));
    let mut syncing = idle.clone();
    syncing.syncing = Some(SyncAction::Push);
    let mut updating = idle.clone();
    updating.updating = Some((0, 2));
    let mut installing = idle.clone();
    installing
        .explore
        .installs
        .insert("catalog/skill".to_owned(), InstallState::Installing);
    let mut changing = idle.clone();
    changing.activation = Some(ActivationProgress::default());
    for busy in [syncing, updating, installing, changing] {
        assert!(!due(&busy));
    }
    engine.shutdown().await?;
    Ok(())
}
