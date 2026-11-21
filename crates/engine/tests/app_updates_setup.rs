//! Windows app updates (spec rules 7, 12, 13, 22): a copy the setup program installed downloads
//! the newer setup program into application data and starts it silently at quit, unless the
//! session is ending. The fixture's setup program is only text, so these run on every platform.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/update_site.rs"]
mod update_site;

use skillbinder_engine::QuitCause;
use skillbinder_proto::{AppUpdateState, CannotInstall};
use std::fs;
use update_site::{Fixture, ready, state};

const STAGED: &str = "updates/.skillbinder-update-0.2.0.exe";

#[tokio::test]
async fn a_copy_without_the_uninstaller_is_download_only() {
    let fixture = Fixture::windows("0.1.0", false);
    fixture.publish("stable", "0.2.0");
    let engine = fixture.open();
    engine.check_app_update(false).await.unwrap();
    assert_eq!(
        state(&engine),
        AppUpdateState::DownloadOnly {
            version: "0.2.0".parse().unwrap(),
            notes_url: "https://example.invalid/v0.2.0".to_owned(),
            reason: CannotInstall::NotInstalled,
        }
    );
    assert!(!fixture.data.join("updates").exists(), "nothing downloads");
}

#[tokio::test]
async fn the_setup_program_waits_in_application_data_across_launches() {
    let fixture = Fixture::windows("0.1.0", true);
    fixture.publish("stable", "0.2.0");
    let engine = fixture.open();
    engine.check_app_update(false).await.unwrap();
    assert_eq!(state(&engine), ready("0.2.0"));
    assert_eq!(
        fs::read_to_string(fixture.data.join(STAGED)).unwrap(),
        "build 0.2.0"
    );
    assert!(
        !fixture.data.join("updates/0.2.0").exists(),
        "the download is gone"
    );
    engine.shutdown().await.unwrap();
    drop(engine);

    let engine = fixture.open();
    assert_eq!(state(&engine), ready("0.2.0"));
    assert!(fixture.data.join(STAGED).exists());
}

#[tokio::test]
async fn a_quit_as_the_session_ends_starts_nothing() {
    let fixture = Fixture::windows("0.1.0", true);
    fixture.publish("stable", "0.2.0");
    let engine = fixture.open();
    engine.check_app_update(false).await.unwrap();
    engine.request_restart().unwrap();
    engine.shutdown().await.unwrap();
    assert!(engine.finish_app_update(QuitCause::SessionEnd).is_none());
    assert!(
        fixture.data.join(STAGED).exists(),
        "the next quit installs it"
    );
    assert!(fixture.log().contains("installs at the next quit"));
    drop(engine);

    let engine = fixture.open();
    assert_eq!(state(&engine), ready("0.2.0"));
}

#[tokio::test]
async fn a_setup_program_that_does_not_start_reopens_the_app_after_restart() {
    let fixture = Fixture::windows("0.1.0", true);
    fixture.publish("stable", "0.2.0");
    let engine = fixture.open();
    engine.check_app_update(false).await.unwrap();
    // The fixture's setup program is text, which no platform runs.
    engine.request_restart().unwrap();
    engine.shutdown().await.unwrap();
    assert!(engine.finish_app_update(QuitCause::User).is_some());
    assert!(
        fixture
            .log()
            .contains("Couldn't start the setup program of SkillBinder 0.2.0")
    );
}
