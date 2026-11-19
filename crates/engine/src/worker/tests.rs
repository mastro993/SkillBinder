use super::*;

#[tokio::test]
#[expect(
    clippy::unwrap_used,
    reason = "Fixture failures must abort the regression test."
)]
async fn shutdown_releases_lock_while_a_child_retains_the_descriptor() {
    let temp = tempfile::tempdir().unwrap();
    let config = EngineConfig::isolated(temp.path().join("data"), temp.path().join("home"));
    let engine = Engine::open(config.clone()).unwrap();
    let inherited_descriptor = engine
        .call(|state| state.lock.try_clone().map_err(|_| AppError::storage()))
        .await
        .unwrap();
    let mut child = std::process::Command::new("/bin/cat")
        .stdin(std::process::Stdio::piped())
        .stdout(inherited_descriptor)
        .spawn()
        .unwrap();
    assert!(
        matches!(Engine::open(config.clone()), Err(error) if error.category == ErrorCategory::Busy)
    );

    engine.shutdown().await.unwrap();
    assert!(child.try_wait().unwrap().is_none());
    let reopened = Engine::open(config);
    drop(child.stdin.take());
    assert!(child.wait().unwrap().success());
    reopened.unwrap().shutdown().await.unwrap();
}
