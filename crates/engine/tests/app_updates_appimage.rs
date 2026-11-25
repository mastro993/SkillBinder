//! Linux app updates (spec rules 7, 12, 13, 23): an AppImage downloads the newer AppImage beside
//! itself and takes its place at quit, and offers once to add itself to the applications menu.
//! The fixture's AppImages are only text, so these run on every Unix.
#![cfg(unix)]
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/update_site.rs"]
mod update_site;

use skillbinder_engine::{Engine, Logger, QuitCause};
use skillbinder_proto::{AppUpdateState, CannotInstall};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use update_site::{Fixture, ready, state};

fn appimage(fixture: &Fixture) -> PathBuf {
    fixture.appimage.clone().unwrap()
}

fn offered(engine: &Engine) -> bool {
    engine.current_app_update().unwrap().offer_applications_menu
}

#[tokio::test]
async fn the_appimage_updates_in_place_at_restart() {
    let fixture = Fixture::appimage("0.1.0");
    fixture.publish("stable", "0.2.0");
    let engine = fixture.open();
    engine.check_app_update(false).await.unwrap();
    assert_eq!(state(&engine), ready("0.2.0"));
    let staged = appimage(&fixture).with_file_name(".skillbinder-update-0.2.0");
    assert_eq!(fs::read_to_string(&staged).unwrap(), "build 0.2.0");
    let mode = fs::metadata(&staged).unwrap().permissions().mode();
    assert_eq!(mode & 0o111, 0o111, "the staged AppImage runs");

    engine.request_restart().unwrap();
    engine.shutdown().await.unwrap();
    assert!(engine.finish_app_update(QuitCause::User).is_some());
    assert_eq!(
        fs::read_to_string(appimage(&fixture)).unwrap(),
        "build 0.2.0"
    );
    let old = appimage(&fixture).with_file_name(".skillbinder-old-0.1.0.AppImage");
    assert_eq!(fs::read_to_string(&old).unwrap(), "build 0.1.0");
    assert!(!staged.exists());
    drop(engine);

    let mut updated = fixture.config();
    updated.app_version = "0.2.0".parse().unwrap();
    let log = Logger::open_session(&updated).unwrap();
    let engine = Engine::open(updated, log).unwrap();
    assert_eq!(state(&engine), AppUpdateState::Idle { last_failure: None });
    assert!(!old.exists(), "the next launch removes the old AppImage");
}

#[tokio::test]
async fn without_appimage_a_newer_version_is_download_only() {
    let fixture = Fixture::new("0.1.0");
    fixture.publish("stable", "0.2.0");
    let engine = fixture.open();
    engine.check_app_update(false).await.unwrap();
    assert!(matches!(
        state(&engine),
        AppUpdateState::DownloadOnly {
            reason: CannotInstall::NotInstalled,
            ..
        }
    ));
    assert!(!offered(&engine), "only an AppImage offers the menu");
}

#[tokio::test]
async fn an_appimage_in_a_read_only_folder_is_download_only() {
    let fixture = Fixture::appimage("0.1.0");
    fixture.publish("stable", "0.2.0");
    let folder = appimage(&fixture).parent().unwrap().to_path_buf();
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o555)).unwrap();
    let probe = folder.join("probe");
    if fs::write(&probe, "").is_ok() {
        // Permissions don't bind this user, such as root in a container.
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    let engine = fixture.open();
    engine.check_app_update(false).await.unwrap();
    let found = state(&engine);
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(
        matches!(
            found,
            AppUpdateState::DownloadOnly {
                reason: CannotInstall::NotWritable,
                ..
            }
        ),
        "{found:?}"
    );
}

#[tokio::test]
async fn add_writes_the_menu_entry_and_is_never_asked_again() {
    let fixture = Fixture::appimage("0.1.0");
    let engine = fixture.open();
    assert!(offered(&engine));
    engine.answer_applications_menu(true).await.unwrap();
    assert!(!offered(&engine));
    let share = fixture.home.join(".local/share");
    let entry = fs::read_to_string(share.join("applications/skillbinder.desktop")).unwrap();
    let exec = format!("\nExec=\"{}\"\n", appimage(&fixture).display());
    assert!(entry.contains(&exec), "{entry}");
    assert!(
        share
            .join("icons/hicolor/256x256/apps/skillbinder.png")
            .is_file()
    );
    engine.shutdown().await.unwrap();
    drop(engine);

    let engine = fixture.open();
    assert!(!offered(&engine));
}

#[tokio::test]
async fn not_now_writes_nothing_and_is_never_asked_again() {
    let fixture = Fixture::appimage("0.1.0");
    let engine = fixture.open();
    engine.answer_applications_menu(false).await.unwrap();
    assert!(!offered(&engine));
    assert!(!fixture.home.join(".local/share/applications").exists());
    engine.shutdown().await.unwrap();
    drop(engine);

    let engine = fixture.open();
    assert!(!offered(&engine));
}
