use super::once_free;
use crate::{Client, Shared, UPDATE_CHECK_INTERVAL};
use skillbinder_engine::{Engine, EngineConfig};
use skillbinder_proto::*;
use std::{
    collections::BTreeSet,
    path::Path,
    sync::{Arc, atomic::AtomicU64},
    time::Duration,
};
use tokio::{runtime::Handle, sync::Semaphore, sync::watch};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A client of an engine in `directory`, onboarded with an empty library, without the
/// background work `Client::new` starts.
async fn client(directory: &Path) -> AppResult<Client> {
    let engine = Engine::open(EngineConfig::isolated(
        directory.join("data"),
        directory.join("home"),
    ))?;
    engine
        .choose_agents(BTreeSet::new(), BTreeSet::new())
        .await?;
    engine.finish_onboarding(None).await?;
    let (snapshots, _) = watch::channel(Arc::new(engine.snapshot().await?));
    Ok(Client {
        shared: Arc::new(Shared {
            engine,
            runtime: Handle::current(),
            snapshots,
            requested: AtomicU64::new(0),
            published: AtomicU64::new(0),
            checked_scans: AtomicU64::new(0),
        }),
    })
}

fn id(slug: &str) -> SkillId {
    SkillId(slug.into())
}

/// Publishes that the last check found an update for each of `slugs`.
fn with_updates(client: &Client, slugs: &[&str]) {
    client.shared.snapshots.send_modify(|current| {
        let mut next = (**current).clone();
        next.updates = slugs
            .iter()
            .map(|slug| {
                let update = SkillUpdate {
                    check: UpdateCheck::Available,
                    failure: None,
                };
                (id(slug), update)
            })
            .collect();
        *current = Arc::new(next);
    });
}

/// Waits for the scan running now, and returns how many scans have completed.
async fn settled_scans(engine: &Engine) -> AppResult<u64> {
    while engine.current_scan()?.1 {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    engine.completed_scans()
}

#[tokio::test]
async fn update_all_counts_each_skill_then_scans_once_and_keeps_its_outcome() -> TestResult {
    let directory = tempfile::tempdir()?;
    let client = client(directory.path()).await?;
    with_updates(&client, &["lint", "review"]);
    client.publish_update_all(None, Some(UpdateOutcome::default()));
    let scans = client.shared.engine.completed_scans()?;
    // Each skill updates once the test lets it, so every count can be seen.
    let permits = Arc::new(Semaphore::new(0));
    let run = client.start_update_all({
        let permits = permits.clone();
        move |skill| {
            let permits = permits.clone();
            async move {
                permits
                    .acquire()
                    .await
                    .map_err(|_| AppError::storage())?
                    .forget();
                Ok(match skill.0.as_str() {
                    "lint" => UpdateOutcome {
                        updated: vec![skill],
                        failed: Vec::new(),
                    },
                    _ => UpdateOutcome {
                        updated: Vec::new(),
                        failed: vec![skill],
                    },
                })
            }
        }
    });
    let started = client.snapshot();
    assert_eq!(started.updating, Some((0, 2)));
    assert_eq!(
        started.update_all_outcome, None,
        "a new run drops the last outcome"
    );
    let refused = client
        .update_all()
        .await
        .err()
        .ok_or("a second run starts")?;
    assert_eq!(refused.category, ErrorCategory::Busy);

    let mut snapshots = client.subscribe();
    snapshots.borrow_and_update();
    permits.add_permits(1);
    snapshots.changed().await?;
    assert_eq!(client.snapshot().updating, Some((1, 2)));
    permits.add_permits(1);
    let outcome = run.await?;
    assert_eq!(outcome.updated, [id("lint")]);
    assert_eq!(outcome.failed, [id("review")]);
    let finished = client.snapshot();
    assert_eq!(finished.updating, None);
    assert_eq!(finished.update_all_outcome, Some(outcome));
    assert_eq!(settled_scans(&client.shared.engine).await?, scans + 1);
    client.shared.engine.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn update_all_with_nothing_to_update_does_nothing() -> TestResult {
    let directory = tempfile::tempdir()?;
    let client = client(directory.path()).await?;
    let failed = UpdateOutcome {
        updated: Vec::new(),
        failed: vec![id("review")],
    };
    client.publish_update_all(None, Some(failed.clone()));
    assert_eq!(client.update_all().await?, UpdateOutcome::default());
    let snapshot = client.snapshot();
    assert_eq!(snapshot.updating, None);
    assert_eq!(snapshot.update_all_outcome, Some(failed));
    client.shared.engine.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn a_completed_source_check_drops_the_outcome_and_a_read_keeps_it() -> TestResult {
    let directory = tempfile::tempdir()?;
    let client = client(directory.path()).await?;
    let failed = UpdateOutcome {
        updated: Vec::new(),
        failed: vec![id("review")],
    };
    client.publish_update_all(Some((1, 3)), Some(failed.clone()));
    client.publish_syncing(Some(SyncAction::Push));
    // The engine knows nothing of the client's running work, so a full read keeps it.
    client.refresh().await?;
    let read = client.snapshot();
    assert_eq!(read.updating, Some((1, 3)));
    assert_eq!(read.update_all_outcome, Some(failed));
    assert_eq!(read.syncing, Some(SyncAction::Push));
    client.check_now().await?;
    let checked = client.snapshot();
    assert_eq!(checked.update_all_outcome, None);
    assert_eq!(checked.updating, Some((1, 3)));
    client.shared.engine.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn sync_shows_as_syncing_until_it_returns() -> TestResult {
    let directory = tempfile::tempdir()?;
    let client = client(directory.path()).await?;
    let mut snapshots = client.subscribe();
    let sync = tokio::spawn({
        let client = client.clone();
        async move { client.synchronize(SyncAction::Refresh).await }
    });
    snapshots.changed().await?;
    assert_eq!(snapshots.borrow().syncing, Some(SyncAction::Refresh));
    // With no remote connected, the engine refuses it.
    let _ = sync.await?;
    assert_eq!(client.snapshot().syncing, None);
    client.shared.engine.shutdown().await?;
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn sources_are_checked_every_hour_from_an_hour_after_launch() -> TestResult {
    let directory = tempfile::tempdir()?;
    let client = client(directory.path()).await?;
    let failed = UpdateOutcome {
        updated: Vec::new(),
        failed: vec![id("review")],
    };
    client.publish_update_all(None, Some(failed));
    client.start_source_checks();
    tokio::time::sleep(UPDATE_CHECK_INTERVAL - Duration::from_secs(1)).await;
    assert!(
        client.snapshot().update_all_outcome.is_some(),
        "no check before the hour"
    );
    let mut snapshots = client.subscribe();
    tokio::time::sleep(Duration::from_secs(2)).await;
    while snapshots.borrow_and_update().update_all_outcome.is_some() {
        snapshots.changed().await?;
    }
    client.shared.engine.shutdown().await?;
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_check_another_check_refuses_tries_again_once_it_ends() -> TestResult {
    // Another caller's check refuses two tries, then this one runs.
    let mut answers = vec![Ok(true), Ok(false), Ok(false)];
    let mut tries = 0;
    let checked = once_free(|| {
        tries += 1;
        std::future::ready(answers.pop().unwrap_or(Ok(true)))
    })
    .await?;
    assert!(checked);
    assert_eq!(tries, 3);
    let failed = once_free(|| std::future::ready(Err::<bool, _>(AppError::storage()))).await;
    assert!(failed.is_err(), "an error ends it at once");
    Ok(())
}
