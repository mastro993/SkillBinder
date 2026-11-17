//! Onboarding's steps derived from saved facts: which apply, where a relaunch resumes, and data
//! saved at the stored position onboarding used before.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[allow(dead_code, reason = "Not every test crate uses every helper.")]
#[path = "support/fixture.rs"]
mod support;

use diesel::{connection::SimpleConnection, prelude::*};
use skillbinder_proto::{
    Bootstrap, ErrorCategory,
    OnboardingStep::{self, DataLocation, Import},
};
use std::{fs, path::Path};
use support::{Fixture, category};

/// Each step that applies, with whether it is done.
fn steps(boot: &Bootstrap) -> Vec<(OnboardingStep, bool)> {
    boot.onboarding
        .steps
        .iter()
        .map(|status| (status.step, status.done))
        .collect()
}

fn is_library(path: &str) -> bool {
    let library = Path::new(path);
    library.join("skills").is_dir() && library.join(".git").is_dir()
}

/// The legacy `onboarding` device setting, if it is still saved.
fn legacy_position(fixture: &Fixture) -> Option<String> {
    diesel::sql_query("SELECT value FROM device_settings WHERE key = 'onboarding'")
        .load::<Value>(&mut fixture.database())
        .unwrap()
        .pop()
        .map(|row| row.value)
}

#[derive(QueryableByName)]
struct Value {
    #[diesel(sql_type = diesel::sql_types::Text)]
    value: String,
}

#[tokio::test]
async fn with_git_and_writable_data_a_first_launch_opens_import_with_the_library_created() {
    let fixture = Fixture::new();
    let engine = fixture.open();

    let boot = engine.bootstrap().await.unwrap();

    assert_eq!(steps(&boot), [(Import, false)]);
    assert!(!boot.onboarding.finished);
    assert!(is_library(&boot.library_path));
    engine.shutdown().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn unwritable_data_holds_onboarding_until_it_is_writable() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    let engine = fixture.open();
    fs::set_permissions(&fixture.data, fs::Permissions::from_mode(0o555)).unwrap();

    let boot = engine.bootstrap().await.unwrap();
    assert_eq!(steps(&boot), [(DataLocation, false), (Import, false)]);
    assert_eq!(
        category(engine.finish_onboarding(None).await),
        ErrorCategory::Validation
    );

    fs::set_permissions(&fixture.data, fs::Permissions::from_mode(0o755)).unwrap();
    let boot = engine.bootstrap().await.unwrap();
    assert_eq!(steps(&boot), [(Import, false)]);
    assert!(is_library(&boot.library_path));
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_relaunch_resumes_at_the_first_unsettled_step_and_repeating_a_step_is_harmless() {
    let fixture = Fixture::new();
    let engine = fixture.open();
    let first = engine.bootstrap().await.unwrap();
    let skills = Path::new(&first.library_path).join("skills");
    fs::write(skills.join("marker"), "kept").unwrap();
    assert_eq!(steps(&engine.bootstrap().await.unwrap()), [(Import, false)]);
    engine.shutdown().await.unwrap();

    let engine = fixture.open();
    assert_eq!(steps(&engine.bootstrap().await.unwrap()), [(Import, false)]);
    assert!(skills.join("marker").is_file());
    assert!(
        engine
            .finish_onboarding(None)
            .await
            .unwrap()
            .onboarding
            .finished
    );
    let again = engine.finish_onboarding(None).await.unwrap();
    assert_eq!(steps(&again), [(Import, true)]);
    engine.shutdown().await.unwrap();

    let engine = fixture.open();
    let boot = engine.bootstrap().await.unwrap();
    assert!(boot.onboarding.finished);
    assert_eq!(boot.onboarding.current(), None);
    assert!(engine.snapshot().await.unwrap().library.is_some());
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn data_saved_at_the_old_stored_position_is_read_once_then_forgotten() {
    for (position, finished) in [("Complete", true), ("Ready", false), ("Import", false)] {
        let fixture = Fixture::new();
        let engine = fixture.open();
        let library = engine.bootstrap().await.unwrap().library_path;
        engine.shutdown().await.unwrap();
        fixture
            .database()
            .batch_execute(&format!(
                "INSERT INTO device_settings (key, value) VALUES ('onboarding', '\"{position}\"')"
            ))
            .unwrap();

        let engine = fixture.open();
        let boot = engine.bootstrap().await.unwrap();

        assert_eq!(boot.onboarding.finished, finished, "{position}");
        assert_eq!(legacy_position(&fixture), None, "{position}");
        assert!(is_library(&library), "{position}");
        engine.shutdown().await.unwrap();
    }
}
