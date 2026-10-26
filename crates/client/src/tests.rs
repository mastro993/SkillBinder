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
        checked_scans: AtomicU64::new(0),
    };
    engine
        .save_preferences(Preferences {
            sidebar_width: 320.0,
            sidebar_collapsed: true,
            appearance: Appearance::Dark,
            folded_sections: [SidebarSection::Presets].into(),
            library_view: LibraryView::Grid,
            active_preset: None,
        })
        .await?;
    let updated = engine.snapshot().await?;
    engine.start_scan().await?;
    while engine.current_scan()?.1 {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    shared.publish_progress()?;
    shared.publish_full(2, updated)?;
    shared.publish_full(1, original)?;
    let current = shared.snapshots.borrow().clone();
    assert_eq!(current.preferences.sidebar_width, 320.0);
    assert!(current.preferences.sidebar_collapsed);
    assert_eq!(current.preferences.appearance, Appearance::Dark);
    assert_eq!(
        current.preferences.folded_sections,
        [SidebarSection::Presets].into()
    );
    assert_eq!(current.preferences.library_view, LibraryView::Grid);
    assert!(current.scan.is_some() && !current.scanning);
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
