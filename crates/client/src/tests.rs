use super::*;
use skillbinder_engine::EngineConfig;
use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// An engine in `directory`, onboarded with an empty library.
async fn ready(directory: &Path) -> AppResult<Engine> {
    let engine = Engine::open(EngineConfig::isolated(
        directory.join("data"),
        directory.join("home"),
    ))?;
    for step in [
        OnboardingStep::Boundaries,
        OnboardingStep::SyncChoice,
        OnboardingStep::Ready,
    ] {
        engine.set_onboarding(step).await?;
    }
    engine.create_library().await?;
    engine.complete_onboarding(None).await?;
    Ok(engine)
}

/// The client's shared state showing `shown`, with two full reads requested and neither shown.
fn two_reads_requested(engine: &Engine, shown: AppSnapshot) -> Shared {
    let (snapshots, _) = watch::channel(Arc::new(shown));
    Shared {
        engine: engine.clone(),
        runtime: Handle::current(),
        snapshots,
        requested: AtomicU64::new(2),
        published: AtomicU64::new(0),
        checked_scans: AtomicU64::new(0),
    }
}

#[tokio::test]
async fn rejects_old_full_reads_without_losing_scan_or_mutation_results() -> TestResult {
    let directory = tempfile::tempdir()?;
    let engine = ready(directory.path()).await?;
    let original = engine.snapshot().await?;
    let shared = two_reads_requested(&engine, original.clone());
    engine
        .save_preferences(Preferences {
            sidebar_width: 320.0,
            sidebar_collapsed: true,
            appearance: Appearance::Dark,
            folded_sections: [SidebarSection::Presets].into(),
            library_view: LibraryView::Grid,
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
async fn committed_outcome_survives_projection_failure() -> TestResult {
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

#[tokio::test]
async fn a_read_shows_at_once_while_a_newer_one_is_still_on_its_way() -> TestResult {
    let directory = tempfile::tempdir()?;
    let engine = ready(directory.path()).await?;
    let original = engine.snapshot().await?;
    let shared = two_reads_requested(&engine, original.clone());
    let read = |sidebar_width| {
        let mut read = original.clone();
        read.preferences.sidebar_width = sidebar_width;
        read
    };
    // A change's own refresh lands while a refresh requested after it is still reading, such as a
    // source check's: the change's result shows before the change returns.
    shared.publish_full(1, read(320.0))?;
    assert_eq!(shared.snapshots.borrow().preferences.sidebar_width, 320.0);
    // The newer read replaces it once it lands.
    shared.publish_full(2, read(400.0))?;
    assert_eq!(shared.snapshots.borrow().preferences.sidebar_width, 400.0);
    engine.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn a_tracked_change_shows_its_result_and_leaves_no_progress_published() -> TestResult {
    let directory = tempfile::tempdir()?;
    let engine = ready(directory.path()).await?;
    let client = Client::new(engine.clone(), Handle::current()).await?;
    let library = client
        .change_organization(OrganizationChange::CreatePreset {
            name: "Work".into(),
        })
        .await?;
    let work = library.library.presets[1].id.clone();
    client
        .change_organization(OrganizationChange::ActivatePreset { id: work.clone() })
        .await?;
    let shown = client.snapshot();
    let active = shown
        .library
        .as_ref()
        .map(|library| &library.library.active_preset);
    assert_eq!(active, Some(&work));
    assert_eq!(shown.activation, None);
    engine.shutdown().await?;
    Ok(())
}
