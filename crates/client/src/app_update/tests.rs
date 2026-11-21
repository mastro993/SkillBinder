use super::Trigger;
use crate::Client;
use skillbinder_engine::{CatalogSource, Engine, EngineConfig, UpdatePayload, UpdateSource};
use skillbinder_proto::{AppUpdateState, Channel, CheckFailure};
use std::time::Duration;
use tokio::runtime::Handle;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn with_automatic_checks_off_only_check_now_and_turning_them_on_check() {
    use Trigger::*;
    for trigger in [Launch, Hourly, ChannelChanged, AutoCheckTurnedOn, CheckNow] {
        assert!(trigger.checks(true), "{trigger:?}");
        let always = matches!(trigger, CheckNow | AutoCheckTurnedOn);
        assert_eq!(trigger.checks(false), always, "{trigger:?}");
        assert_eq!(trigger.manual(), trigger == CheckNow, "{trigger:?}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn check_now_checks_with_automatic_checks_off_and_shows_its_failure() -> TestResult {
    let directory = tempfile::tempdir()?;
    let mut config =
        EngineConfig::isolated(directory.path().join("data"), directory.path().join("home"));
    // A missing site answers as an unreachable host.
    config.app_update = Some(UpdateSource {
        site: CatalogSource::Directory(directory.path().join("site")),
        public_key: String::new(),
        payload: UpdatePayload::Executable,
    });
    let engine = Engine::open(config)?;
    engine.set_update_auto_check(false).await?;
    let client = Client::new(engine.clone(), Handle::current()).await?;
    client.set_update_channel(Channel::Beta).await?;
    assert!(!client.snapshot().app_update.auto_check);
    client.check_app_update();
    let mut snapshots = client.subscribe();
    let offline = AppUpdateState::Idle {
        last_failure: Some(CheckFailure::Offline),
    };
    tokio::time::timeout(Duration::from_secs(10), async {
        while client.snapshot().app_update.state != offline {
            snapshots.changed().await?;
        }
        Ok::<_, tokio::sync::watch::error::RecvError>(())
    })
    .await??;
    engine.shutdown().await?;
    Ok(())
}
