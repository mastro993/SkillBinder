#![expect(
    clippy::unwrap_used,
    reason = "Assertions describe isolated fixture failures."
)]
use super::*;

type Work = Box<dyn FnOnce() + Send>;

fn fixture() -> (tempfile::TempDir, EngineConfig, Logger) {
    let directory = tempfile::tempdir().unwrap();
    let config =
        EngineConfig::isolated(directory.path().join("data"), directory.path().join("home"));
    std::fs::create_dir_all(&config.home).unwrap();
    let log = Logger::start(config.data_dir.join("logs"), &config.home).unwrap();
    (directory, config, log)
}

/// Starts a scan whose work runs only when the returned closure is called.
fn deferred(scans: &Scans, config: &EngineConfig, log: &Logger) -> Work {
    let mut pending = None;
    scans
        .start_with(
            config.clone(),
            Vec::new(),
            BTreeMap::new(),
            log.clone(),
            |work| {
                pending = Some(work);
                Ok(())
            },
        )
        .unwrap();
    pending.unwrap()
}

#[test]
fn a_new_scan_replaces_the_running_one_and_keeps_the_last_results_meanwhile() {
    let (_directory, config, log) = fixture();
    let scans = Scans::default();
    let replaced = deferred(&scans, &config, &log);
    assert_eq!(scans.current().unwrap(), (None, true));
    let current = deferred(&scans, &config, &log);
    replaced();
    assert_eq!(
        scans.current().unwrap(),
        (None, true),
        "a replaced scan publishes nothing"
    );
    current();
    let (found, running) = scans.current().unwrap();
    assert!(!running);
    let found = found.unwrap();
    assert_eq!(found.error, None);
    let next = deferred(&scans, &config, &log);
    assert_eq!(scans.current().unwrap(), (Some(found), true));
    next();
    assert_eq!(scans.0.lock().unwrap().threads, 0);
}

#[test]
fn launch_failure_is_a_failed_scan_and_retryable_without_hanging_shutdown() {
    let (_directory, config, log) = fixture();
    let scans = Scans::default();
    let result = scans.start_with(
        config.clone(),
        Vec::new(),
        BTreeMap::new(),
        log.clone(),
        |_| Err(std::io::Error::other("injected spawn refusal")),
    );
    assert!(result.is_err());
    let (failed, running) = scans.current().unwrap();
    assert!(!running);
    assert!(
        failed
            .unwrap()
            .error
            .unwrap()
            .contains("Try scanning again")
    );
    assert!(scans.selected(&[CandidateId::new()]).is_err());
    scans.wait_for_shutdown();
    deferred(&scans, &config, &log)();
    assert_eq!(scans.current().unwrap().0.unwrap().error, None);
}
