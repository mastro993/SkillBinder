#![expect(
    clippy::unwrap_used,
    reason = "Assertions describe isolated fixture failures."
)]
use super::*;

fn fixture() -> (tempfile::TempDir, EngineConfig, Logger) {
    let directory = tempfile::tempdir().unwrap();
    let config =
        EngineConfig::isolated(directory.path().join("data"), directory.path().join("home"));
    std::fs::create_dir_all(&config.home).unwrap();
    let log = Logger::start(config.data_dir.join("logs"), &config.home).unwrap();
    (directory, config, log)
}

#[test]
fn active_scans_never_expire_or_launch_a_duplicate() {
    let (_directory, config, log) = fixture();
    let scans = Scans::default();
    let mut pending = None;
    let started = scans
        .start_with(
            config.clone(),
            Vec::new(),
            BTreeSet::new(),
            log.clone(),
            |work| {
                pending = Some(work);
                Ok(())
            },
        )
        .unwrap();
    let later = Instant::now() + Duration::from_secs(10_000);
    assert_eq!(scans.current_at(later).unwrap().unwrap().id, started.id);
    let duplicate = scans
        .start_with(config, Vec::new(), BTreeSet::new(), log, |_| {
            panic!("active scan launched a duplicate")
        })
        .unwrap();
    assert_eq!(duplicate.id, started.id);
    pending.unwrap()();
    assert_eq!(
        scans.current().unwrap().unwrap().status,
        ScanStatus::Finished
    );
    let finished = scans
        .0
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .terminal_at
        .unwrap();
    assert!(
        scans
            .current_at(finished + Duration::from_secs(599))
            .unwrap()
            .is_some()
    );
    assert!(
        scans
            .current_at(finished + Duration::from_secs(600))
            .unwrap()
            .is_none()
    );
}

#[test]
fn cancellation_retains_the_run_for_600_seconds_after_worker_finishes() {
    let (_directory, config, log) = fixture();
    let scans = Scans::default();
    let mut pending = None;
    scans
        .start_with(config, Vec::new(), BTreeSet::new(), log, |work| {
            pending = Some(work);
            Ok(())
        })
        .unwrap();
    scans.cancel().unwrap();
    assert_eq!(
        scans.current().unwrap().unwrap().status,
        ScanStatus::Running
    );
    assert!(
        scans
            .current_at(Instant::now() + Duration::from_secs(600))
            .unwrap()
            .is_some()
    );
    pending.unwrap()();
    assert_eq!(
        scans.current().unwrap().unwrap().status,
        ScanStatus::Cancelled
    );
    let terminal = scans
        .0
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .terminal_at
        .unwrap();
    assert!(
        scans
            .current_at(terminal + Duration::from_secs(599))
            .unwrap()
            .is_some()
    );
    assert!(
        scans
            .current_at(terminal + Duration::from_secs(600))
            .unwrap()
            .is_none()
    );
}

#[test]
fn launch_failure_is_terminal_and_retryable_without_hanging_shutdown() {
    let (_directory, config, log) = fixture();
    let scans = Scans::default();
    let result = scans.start_with(
        config.clone(),
        Vec::new(),
        BTreeSet::new(),
        log.clone(),
        |_| Err(std::io::Error::other("injected spawn refusal")),
    );
    assert!(result.is_err());
    let failed = scans.current().unwrap().unwrap();
    assert_eq!(failed.status, ScanStatus::Failed);
    assert!(failed.error.unwrap().contains("Try scanning again"));
    scans.wait_for_shutdown();
    let terminal = scans
        .0
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .terminal_at
        .unwrap();
    assert!(
        scans
            .current_at(terminal + Duration::from_secs(599))
            .unwrap()
            .is_some()
    );
    assert!(
        scans
            .current_at(terminal + Duration::from_secs(600))
            .unwrap()
            .is_none()
    );
    let mut pending = None;
    let retry = scans
        .start_with(config, Vec::new(), BTreeSet::new(), log, |work| {
            pending = Some(work);
            Ok(())
        })
        .unwrap();
    assert_ne!(failed.id, retry.id);
    pending.unwrap()();
    assert_eq!(
        scans.current().unwrap().unwrap().status,
        ScanStatus::Finished
    );
}
