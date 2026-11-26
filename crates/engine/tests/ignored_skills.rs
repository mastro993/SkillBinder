//! Ignored skills (ADR 0033): an ignore names a found skill's slug on this device, survives a
//! relaunch and changed content, changes no file, and ends when the slug enters the library by
//! import or Pull. Every remote is a local repository.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/device.rs"]
mod device;
#[path = "support/fixture.rs"]
mod support;

use device::Device;
use skillbinder_engine::Engine;
use skillbinder_proto::{AppResult, ErrorCategory, SyncAction};
use std::{collections::BTreeSet, fs};
use support::{Fixture, write_skill};

async fn ignored(engine: &Engine) -> BTreeSet<String> {
    engine.snapshot().await.unwrap().ignored
}

fn keys(keys: &[&str]) -> BTreeSet<String> {
    keys.iter().map(|key| (*key).to_owned()).collect()
}

fn refusal(result: AppResult<()>) -> (ErrorCategory, String) {
    let error = result.unwrap_err();
    (error.category, error.message)
}

fn slugs(slugs: &[&str]) -> Vec<String> {
    slugs.iter().map(|slug| (*slug).to_owned()).collect()
}

#[tokio::test]
async fn an_ignore_persists_whatever_the_content_and_refuses_unknown_and_library_slugs() {
    let fixture = Fixture::new();
    fixture.skill("held", "Already imported");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    fixture.import_one(&engine, &scan.candidates[0]).await;
    let storybook = fixture.skill("storybook", "Stories");
    fixture.skill("tdd", "Tests first");
    fixture.scan(&engine).await;

    // Nothing changes unless every slug is accepted.
    let unknown = engine
        .set_ignored(slugs(&["storybook", "nope"]), true)
        .await;
    assert_eq!(
        refusal(unknown),
        (
            ErrorCategory::NotFound,
            "No found skill named nope.".to_owned()
        )
    );
    let held = engine.set_ignored(slugs(&["held"]), true).await;
    assert_eq!(
        refusal(held),
        (
            ErrorCategory::Validation,
            "held is in your library.".to_owned()
        )
    );
    assert!(ignored(&engine).await.is_empty());

    // Any spelling of the slug ignores it, and ignoring it again changes nothing.
    engine
        .set_ignored(slugs(&["Storybook"]), true)
        .await
        .unwrap();
    engine
        .set_ignored(slugs(&["storybook"]), true)
        .await
        .unwrap();
    assert_eq!(ignored(&engine).await, keys(&["storybook"]));
    let manifest = storybook.join("SKILL.md");
    let before = fs::read_to_string(&manifest).unwrap();
    engine.shutdown().await.unwrap();

    let engine = fixture.open();
    assert_eq!(ignored(&engine).await, keys(&["storybook"]), "relaunched");
    assert_eq!(
        fs::read_to_string(&manifest).unwrap(),
        before,
        "no file changes"
    );
    write_skill(
        &fixture.workspace.join(".agents/skills"),
        "storybook",
        "Edited",
    );
    fixture.scan(&engine).await;
    assert_eq!(ignored(&engine).await, keys(&["storybook"]), "edited");

    let stop = engine.set_ignored(slugs(&["nope"]), false).await;
    assert_eq!(
        refusal(stop),
        (
            ErrorCategory::NotFound,
            "No ignored skill named nope.".to_owned()
        )
    );
    // A found skill that is not ignored is accepted and stays offered.
    engine.set_ignored(slugs(&["tdd"]), false).await.unwrap();
    engine
        .set_ignored(slugs(&["storybook"]), false)
        .await
        .unwrap();
    assert!(ignored(&engine).await.is_empty());
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn importing_an_ignored_slug_ends_its_ignore() {
    let fixture = Fixture::new();
    fixture.skill("storybook", "Stories");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    engine
        .set_ignored(slugs(&["storybook"]), true)
        .await
        .unwrap();

    fixture.import_one(&engine, &scan.candidates[0]).await;

    assert!(ignored(&engine).await.is_empty());
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn pulling_an_ignored_slug_ends_its_ignore() {
    let device = Device::published(&["review"]).await;
    let engine = &device.engine;
    device.fixture.skill("pulled", "Found here first");
    device.fixture.scan(engine).await;
    engine.set_ignored(slugs(&["pulled"]), true).await.unwrap();
    assert_eq!(ignored(engine).await, keys(&["pulled"]));

    device.peer_push(|skills| {
        write_skill(skills, "pulled", "From a peer");
    });
    engine.synchronize(SyncAction::Pull).await.unwrap();

    assert!(ignored(engine).await.is_empty());
}
