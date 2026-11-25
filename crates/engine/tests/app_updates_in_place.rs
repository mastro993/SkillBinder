//! App updates in place (spec rules 5–8, 12, 14, 17): a fixture bundle in a fixture Applications
//! folder downloads a newer version, stages it beside itself, and installs it at quit. The
//! archive is the bundle's executable and nothing is signed, so these run on every platform.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/update_site.rs"]
mod update_site;

use skillbinder_engine::{Engine, Logger, QuitCause};
use skillbinder_proto::{AppUpdateState, CannotInstall, Channel, CheckFailure};
use std::fs;
use update_site::{Fixture, TARGET, UP_TO_DATE, ready, state};

const APP: &str = "SkillBinder.app";

#[tokio::test]
async fn a_newer_version_downloads_and_waits_beside_the_app() {
    let fixture = Fixture::installed("0.1.0");
    fixture.publish("stable", "0.2.0");
    let engine = fixture.open();
    engine.check_app_update(false).await.unwrap();
    assert_eq!(state(&engine), ready("0.2.0"));
    assert_eq!(
        fixture.applications_names(),
        [".skillbinder-update-0.2.0.app", APP]
    );
    assert_eq!(
        fixture.build_of(".skillbinder-update-0.2.0.app"),
        "build 0.2.0"
    );
    assert!(
        !fixture.data.join("updates/0.2.0").exists(),
        "the download is gone"
    );

    // It stays ready across launches, and a check that finds it again downloads nothing.
    engine.shutdown().await.unwrap();
    drop(engine);
    let engine = fixture.open();
    assert_eq!(state(&engine), ready("0.2.0"));
    fs::remove_dir_all(fixture.site.join("downloads")).unwrap();
    engine.check_app_update(true).await.unwrap();
    assert_eq!(state(&engine), ready("0.2.0"));
}

#[tokio::test]
async fn a_download_with_the_wrong_sha256_is_discarded() {
    let fixture = Fixture::installed("0.1.0");
    let wrong = "0".repeat(64);
    fixture.publish_with("stable", "0.2.0", TARGET, &fixture.key, Some(&wrong));
    let engine = fixture.open();
    engine.check_app_update(false).await.unwrap();
    assert_eq!(
        state(&engine),
        UP_TO_DATE,
        "an automatic check stays silent"
    );
    assert_eq!(
        engine.current_app_update().unwrap().last_checked,
        None,
        "nor does it read as checked"
    );
    assert_eq!(fixture.applications_names(), [APP]);
    engine.check_app_update(true).await.unwrap();
    assert_eq!(
        state(&engine),
        AppUpdateState::Idle {
            last_failure: Some(CheckFailure::Failed)
        },
        "Check now says it failed"
    );

    // The next check tries again.
    fixture.publish("stable", "0.2.0");
    engine.check_app_update(false).await.unwrap();
    assert_eq!(state(&engine), ready("0.2.0"));
    engine.shutdown().await.unwrap();
    assert!(fixture.log().contains("update discarded version=0.2.0"));
}

#[tokio::test]
async fn quit_installs_the_update_and_the_next_launch_cleans_up() {
    let fixture = Fixture::installed("0.1.0");
    fixture.publish("stable", "0.2.0");
    let engine = fixture.open();
    assert_eq!(engine.current_app_update().unwrap().updated, None);
    engine.check_app_update(false).await.unwrap();
    engine.request_restart().unwrap();
    engine.shutdown().await.unwrap();
    assert!(
        engine.finish_app_update(QuitCause::User).is_some(),
        "Restart opens it again"
    );
    assert_eq!(fixture.build_of(APP), "build 0.2.0");
    assert_eq!(
        fixture.build_of(".skillbinder-old-0.1.0.app"),
        "build 0.1.0"
    );
    drop(engine);

    let mut updated = fixture.config();
    updated.app_version = "0.2.0".parse().unwrap();
    let log = Logger::open_session(&updated).unwrap();
    let engine = Engine::open(updated.clone(), log).unwrap();
    assert_eq!(fixture.applications_names(), [APP]);
    let update = engine.current_app_update().unwrap();
    assert_eq!(update.state, UP_TO_DATE);
    assert_eq!(
        update.updated.as_deref(),
        Some("https://example.invalid/v0.2.0")
    );
    engine.shutdown().await.unwrap();
    assert!(
        engine.finish_app_update(QuitCause::User).is_none(),
        "nothing to install or open"
    );
    drop(engine);

    let log = Logger::open_session(&updated).unwrap();
    let engine = Engine::open(updated, log).unwrap();
    assert_eq!(engine.current_app_update().unwrap().updated, None, "once");
}

#[tokio::test]
async fn a_newer_version_replaces_the_staged_one() {
    let fixture = Fixture::installed("0.1.0");
    fixture.publish("stable", "0.2.0");
    let engine = fixture.open();
    engine.check_app_update(false).await.unwrap();
    fixture.publish("stable", "0.2.1");
    engine.check_app_update(false).await.unwrap();
    assert_eq!(state(&engine), ready("0.2.1"));
    assert_eq!(
        fixture.applications_names(),
        [".skillbinder-update-0.2.1.app", APP]
    );
    // Quit without Restart installs it and opens nothing.
    engine.shutdown().await.unwrap();
    assert!(engine.finish_app_update(QuitCause::User).is_none());
    assert_eq!(fixture.build_of(APP), "build 0.2.1");
}

#[tokio::test]
async fn switching_to_stable_discards_a_staged_beta() {
    let fixture = Fixture::installed("0.1.1-beta.1");
    fixture.publish("beta", "0.1.1-beta.2");
    let engine = fixture.open();
    engine.check_app_update(false).await.unwrap();
    assert_eq!(state(&engine), ready("0.1.1-beta.2"));
    engine.set_update_channel(Channel::Stable).await.unwrap();
    assert_eq!(state(&engine), UP_TO_DATE);
    assert_eq!(fixture.applications_names(), [APP]);
    engine.shutdown().await.unwrap();
    assert!(engine.finish_app_update(QuitCause::User).is_none());
    assert_eq!(fixture.build_of(APP), "build 0.1.1-beta.1");
}

#[tokio::test]
async fn switching_to_stable_keeps_a_staged_release() {
    let fixture = Fixture::installed("0.1.1-beta.1");
    fixture.publish("beta", "0.1.1");
    let engine = fixture.open();
    engine.check_app_update(false).await.unwrap();
    engine.set_update_channel(Channel::Stable).await.unwrap();
    assert_eq!(state(&engine), ready("0.1.1"));
    assert_eq!(
        fixture.applications_names(),
        [".skillbinder-update-0.1.1.app", APP]
    );
}

#[tokio::test]
async fn outside_applications_a_newer_version_is_download_only() {
    let mut fixture = Fixture::new("0.1.0");
    let bundle = fixture.root.join("Downloads").join(APP);
    fs::create_dir_all(&bundle).unwrap();
    fixture.bundle = Some(bundle);
    fixture.publish("stable", "0.2.0");
    let engine = fixture.open();
    engine.check_app_update(false).await.unwrap();
    assert_eq!(
        state(&engine),
        AppUpdateState::DownloadOnly {
            version: "0.2.0".parse().unwrap(),
            notes_url: "https://example.invalid/v0.2.0".to_owned(),
            reason: CannotInstall::OutsideApplications,
        }
    );
    assert!(!fixture.data.join("updates").exists(), "nothing downloads");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn a_download_that_fails_after_a_channel_switch_checks_the_new_channel() {
    let fixture = Fixture::installed("0.1.0");
    let wrong = "0".repeat(64);
    fixture.publish_with("stable", "0.2.0", TARGET, &fixture.key, Some(&wrong));
    fixture.publish("beta", "0.3.0-beta.1");
    // A FIFO holds the stable download until it is written.
    let held = fixture.site.join("downloads/0.2.0");
    let archive = fs::read(&held).unwrap();
    fs::remove_file(&held).unwrap();
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&held)
            .status()
            .unwrap()
            .success()
    );
    let engine = fixture.open();
    let check = tokio::spawn({
        let engine = engine.clone();
        async move { engine.check_app_update(true).await }
    });
    while !matches!(state(&engine), AppUpdateState::Downloading { .. }) {
        tokio::task::yield_now().await;
    }
    engine.set_update_channel(Channel::Beta).await.unwrap();
    fs::write(&held, archive).unwrap();
    check.await.unwrap().unwrap();
    assert_eq!(state(&engine), ready("0.3.0-beta.1"));
}
