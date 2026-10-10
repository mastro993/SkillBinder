use super::*;
use skillbinder_engine::EngineConfig;

#[tokio::test]
async fn rejects_old_full_reads_without_losing_scan_or_mutation_results()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let engine = Engine::open(EngineConfig::isolated(
        directory.path().join("data"),
        directory.path().join("home"),
    ))?;
    for step in [
        OnboardingStep::Boundaries,
        OnboardingStep::SyncChoice,
        OnboardingStep::Ready,
    ] {
        engine.set_onboarding(step).await?;
    }
    engine.complete_onboarding().await?;
    let original = engine.snapshot().await?;
    let (snapshots, _) = watch::channel(Arc::new(original.clone()));
    let shared = Shared {
        engine: engine.clone(),
        runtime: Handle::current(),
        snapshots,
        requested: AtomicU64::new(2),
    };
    engine
        .save_preferences(Preferences {
            sidebar_width: 320.0,
            sidebar_collapsed: true,
        })
        .await?;
    let updated = engine.snapshot().await?;
    let scan = engine.start_scan().await?;
    shared.publish_scan()?;
    shared.publish_full(2, updated)?;
    shared.publish_full(1, original)?;
    let current = shared.snapshots.borrow().clone();
    assert_eq!(current.preferences.sidebar_width, 320.0);
    assert!(current.preferences.sidebar_collapsed);
    assert_eq!(current.scan.as_ref().map(|scan| &scan.id), Some(&scan.id));
    assert_eq!(shared.requested.load(Ordering::Acquire), 2);
    assert!(current.generation >= 2);
    engine.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn committed_outcome_survives_projection_failure() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let engine = Engine::open(EngineConfig::isolated(
        directory.path().join("data"),
        directory.path().join("home"),
    ))?;
    let client = Client::new(engine.clone(), Handle::current()).await?;
    engine.shutdown().await?;
    assert_eq!(client.refresh_preserving_outcome(Ok(42)).await?, 42);
    assert!(client.snapshot().background_error.is_some());
    let failure: AppResult<()> = Err(AppError::stale("Original operation failed"));
    let error = client
        .refresh_preserving_outcome(failure)
        .await
        .err()
        .ok_or("Expected error")?;
    assert_eq!(error.message, "Original operation failed");
    Ok(())
}
