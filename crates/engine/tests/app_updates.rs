//! App update checks: the channel's manifest read from a fixture site laid out like the catalog's,
//! signed with a key made for the test, and compared with the configured running version. These
//! run from no bundle, so a newer version is download-only.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/update_site.rs"]
mod update_site;

use minisign::KeyPair;
use skillbinder_engine::{Engine, Logger};
use skillbinder_proto::{AppUpdateState, CannotInstall, Channel, CheckFailure};
use std::fs;
use update_site::{Fixture, TARGET, UP_TO_DATE, state};

/// A development build runs from no bundle, so it can't replace itself.
fn offered(version: &str) -> AppUpdateState {
    AppUpdateState::DownloadOnly {
        version: version.parse().unwrap(),
        notes_url: format!("https://example.invalid/v{version}"),
        reason: CannotInstall::NotInstalled,
    }
}

#[tokio::test]
async fn a_check_offers_only_a_newer_signed_version_for_this_platform() {
    let fixture = Fixture::new("0.1.1-beta.12");
    let engine = fixture.open();
    assert_eq!(state(&engine), UP_TO_DATE);
    assert_eq!(engine.current_app_update().unwrap().last_checked, None);
    for (version, expected) in [
        ("0.1.0", UP_TO_DATE),
        ("0.1.1-beta.12", UP_TO_DATE),
        ("0.1.1-beta.13", offered("0.1.1-beta.13")),
        ("0.1.1", offered("0.1.1")),
    ] {
        fixture.publish("beta", version);
        engine.check_app_update(false).await.unwrap();
        assert_eq!(state(&engine), expected, "{version}");
    }
    assert!(engine.current_app_update().unwrap().last_checked.is_some());

    fixture.publish_with("beta", "0.2.0", "sparc-sun-solaris", &fixture.key, None);
    engine.check_app_update(false).await.unwrap();
    assert_eq!(state(&engine), UP_TO_DATE, "no archive for this platform");

    let other = KeyPair::generate_unencrypted_keypair().unwrap();
    fixture.publish_with("beta", "0.2.0", TARGET, &other, None);
    engine.check_app_update(true).await.unwrap();
    assert_eq!(
        state(&engine),
        AppUpdateState::Idle {
            last_failure: Some(CheckFailure::Failed)
        },
        "a bad signature offers nothing"
    );
    engine.shutdown().await.unwrap();
    assert!(fixture.log().contains("signature does not verify"));
}

#[tokio::test]
async fn an_unchanged_manifest_changes_nothing() {
    let fixture = Fixture::new("0.1.0");
    fixture.publish("stable", "0.2.0");
    let engine = fixture.open();
    engine.check_app_update(false).await.unwrap();
    assert_eq!(state(&engine), offered("0.2.0"));
    // A 304 never reads the signature again, so a broken one shows it was not refetched.
    fs::write(
        fixture.site.join("updates/v1/stable.json.minisig"),
        "broken",
    )
    .unwrap();
    engine.check_app_update(true).await.unwrap();
    assert_eq!(state(&engine), offered("0.2.0"));
}

#[tokio::test]
async fn only_a_manual_check_shows_its_failure() {
    let fixture = Fixture::new("0.1.0");
    let engine = fixture.open();
    engine.check_app_update(false).await.unwrap();
    assert_eq!(
        state(&engine),
        UP_TO_DATE,
        "an automatic check stays silent"
    );
    engine.check_app_update(true).await.unwrap();
    assert_eq!(
        state(&engine),
        AppUpdateState::Idle {
            last_failure: Some(CheckFailure::Offline)
        }
    );
    fixture.publish("stable", "0.2.0");
    engine.check_app_update(false).await.unwrap();
    fs::remove_dir_all(&fixture.site).unwrap();
    engine.check_app_update(false).await.unwrap();
    assert_eq!(
        state(&engine),
        offered("0.2.0"),
        "a failure keeps the offer"
    );
}

#[tokio::test]
async fn a_build_without_a_source_is_off() {
    let fixture = Fixture::new("0.1.0");
    fixture.publish("stable", "0.2.0");
    let mut config = fixture.config();
    config.app_update = None;
    let log = Logger::open_session(&config).unwrap();
    let engine = Engine::open(config, log).unwrap();
    engine.check_app_update(true).await.unwrap();
    let update = engine.current_app_update().unwrap();
    assert_eq!(update.state, AppUpdateState::Off);
    assert_eq!(update.last_checked, None);
}

#[tokio::test]
async fn the_channel_starts_as_the_builds_and_settings_persist() {
    let fixture = Fixture::new("0.1.1-beta.12");
    fixture.publish("beta", "0.1.1-beta.13");
    fixture.publish("stable", "0.1.0");
    let engine = fixture.open();
    let update = engine.current_app_update().unwrap();
    assert_eq!((update.channel, update.auto_check), (Channel::Beta, true));
    engine.check_app_update(false).await.unwrap();
    assert_eq!(state(&engine), offered("0.1.1-beta.13"));

    engine.set_update_channel(Channel::Stable).await.unwrap();
    assert_eq!(
        state(&engine),
        UP_TO_DATE,
        "the beta offer is for the other channel"
    );
    assert_eq!(
        engine.current_app_update().unwrap().last_checked,
        None,
        "Stable was never checked"
    );
    engine.check_app_update(false).await.unwrap();
    assert_eq!(
        state(&engine),
        UP_TO_DATE,
        "no stable release passes the beta"
    );
    engine.set_update_auto_check(false).await.unwrap();
    let checked = engine.current_app_update().unwrap().last_checked;
    engine.shutdown().await.unwrap();
    drop(engine);

    let engine = fixture.open();
    let update = engine.current_app_update().unwrap();
    assert_eq!(update.channel, Channel::Stable);
    assert!(!update.auto_check);
    assert_eq!(update.last_checked, checked);
}

#[tokio::test]
async fn a_relaunch_shows_the_last_offer_until_the_next_check() {
    let fixture = Fixture::new("0.1.0");
    fixture.publish("stable", "0.2.0");
    let engine = fixture.open();
    engine.check_app_update(false).await.unwrap();
    engine.shutdown().await.unwrap();
    drop(engine);

    let engine = fixture.open();
    assert_eq!(state(&engine), offered("0.2.0"));
    engine.shutdown().await.unwrap();
    drop(engine);

    // A build that already passed the offer reads as up to date.
    let mut config = fixture.config();
    config.app_version = "0.2.0".parse().unwrap();
    let log = Logger::open_session(&config).unwrap();
    let engine = Engine::open(config, log).unwrap();
    assert_eq!(state(&engine), UP_TO_DATE);
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn a_channel_switched_during_a_check_is_checked_next() {
    let fixture = Fixture::new("0.1.0");
    fixture.publish("beta", "0.3.0-beta.1");
    fixture.publish("stable", "0.2.0");
    // A FIFO holds the stable check until it is written.
    let held = fixture.site.join("updates/v1/stable.json");
    let manifest = fs::read(&held).unwrap();
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
    while state(&engine) != AppUpdateState::Checking {
        tokio::task::yield_now().await;
    }
    engine.set_update_channel(Channel::Beta).await.unwrap();
    fs::write(&held, manifest).unwrap();
    check.await.unwrap().unwrap();
    assert_eq!(state(&engine), offered("0.3.0-beta.1"));
}
