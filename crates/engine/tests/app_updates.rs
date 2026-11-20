//! App update checks: the channel's manifest read from a fixture site laid out like the catalog's,
//! signed with a key made for the test, and compared with the configured running version.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
use minisign::KeyPair;
use skillbinder_engine::{CatalogSource, Engine, EngineConfig, UpdateSource};
use skillbinder_proto::{AppUpdateState, AppVersion, Channel, CheckFailure};
use std::{fs, io::Cursor, path::PathBuf};
use tempfile::TempDir;

/// The archive key of the platform the tests run on.
const TARGET: &str = if cfg!(target_os = "macos") {
    "universal-apple-darwin"
} else if cfg!(windows) {
    "x86_64-pc-windows-msvc"
} else {
    "x86_64-unknown-linux-gnu"
};

/// An isolated device running `current`, whose update site is `<temp>/site`.
struct Fixture {
    _temp: TempDir,
    data: PathBuf,
    home: PathBuf,
    site: PathBuf,
    key: KeyPair,
    current: AppVersion,
}

impl Fixture {
    fn new(current: &str) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().to_path_buf();
        Self {
            _temp: temp,
            data: path.join("data"),
            home: path.join("home"),
            site: path.join("site"),
            key: KeyPair::generate_unencrypted_keypair().unwrap(),
            current: current.parse().unwrap(),
        }
    }

    fn config(&self) -> EngineConfig {
        let mut config = EngineConfig::isolated(self.data.clone(), self.home.clone());
        config.app_version = self.current;
        config.app_update = Some(UpdateSource {
            site: CatalogSource::Directory(self.site.clone()),
            public_key: self.key.pk.to_base64(),
        });
        config
    }

    fn open(&self) -> Engine {
        Engine::open(self.config()).unwrap()
    }

    /// Publishes `version` on `channel`, with an archive for `target`, signed with `key`.
    fn publish_with(&self, channel: &str, version: &str, target: &str, key: &KeyPair) {
        let manifest = format!(
            r#"{{"version":"{version}","notes_url":"https://example.invalid/v{version}","published_at":"2026-12-01T10:00:00Z","assets":{{"{target}":{{"url":"https://example.invalid/a","sha256":"00","size":1}}}},"later":true}}"#
        );
        let signature = minisign::sign(None, &key.sk, Cursor::new(&manifest), None, None).unwrap();
        let directory = self.site.join("updates/v1");
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join(format!("{channel}.json")), &manifest).unwrap();
        fs::write(
            directory.join(format!("{channel}.json.minisig")),
            signature.into_string(),
        )
        .unwrap();
    }

    fn publish(&self, channel: &str, version: &str) {
        self.publish_with(channel, version, TARGET, &self.key);
    }

    fn log(&self) -> String {
        fs::read_to_string(self.data.join("logs/skillbinder.log")).unwrap_or_default()
    }
}

fn state(engine: &Engine) -> AppUpdateState {
    engine.current_app_update().unwrap().state
}

fn offered(version: &str) -> AppUpdateState {
    AppUpdateState::DownloadOnly {
        version: version.parse().unwrap(),
        notes_url: format!("https://example.invalid/v{version}"),
    }
}

const UP_TO_DATE: AppUpdateState = AppUpdateState::Idle { last_failure: None };

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

    fixture.publish_with("beta", "0.2.0", "sparc-sun-solaris", &fixture.key);
    engine.check_app_update(false).await.unwrap();
    assert_eq!(state(&engine), UP_TO_DATE, "no archive for this platform");

    let other = KeyPair::generate_unencrypted_keypair().unwrap();
    fixture.publish_with("beta", "0.2.0", TARGET, &other);
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
    let engine = Engine::open(config).unwrap();
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
