//! Installing from Explore: a skill's folder is fetched from its source into the library,
//! inactive, with a record holding its source. Every source is a local repository.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/publisher.rs"]
mod publisher;
#[path = "support/fixture.rs"]
mod support;

use publisher::{Publisher, catalog_skill};
use skillbinder_engine::Engine;
use skillbinder_proto::{
    AppSnapshot, ErrorCategory, InstallFailure, InstallState, SkillId, SkillSource, UpdateCheck,
    install_key,
};
use std::{fs, path::Path, time::Duration};
use support::{Fixture, category, write_skill};

/// A publishing repository beside an engine's fixture.
struct Published {
    fixture: Fixture,
    publisher: Publisher,
}

impl Published {
    fn new() -> Self {
        let fixture = Fixture::new();
        let publisher = Publisher::new(fixture.workspace.with_file_name("publisher"));
        Self { fixture, publisher }
    }
}

async fn snapshot(engine: &Engine) -> AppSnapshot {
    engine.snapshot().await.unwrap()
}

fn slugs(snapshot: &AppSnapshot) -> Vec<String> {
    let library = snapshot.library.as_ref().unwrap();
    library
        .library
        .skills
        .iter()
        .map(|skill| skill.id.0.clone())
        .collect()
}

/// Whether any directory under `root` is named `name`.
fn holds_directory(root: &Path, name: &str) -> bool {
    fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .any(|entry| {
            let path = entry.path();
            path.is_dir() && (entry.file_name() == name || holds_directory(&path, name))
        })
}

fn leftovers(data: &Path, folder: &str) -> Vec<String> {
    fs::read_dir(data.join(folder))
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("install") || folder.ends_with("install"))
        .collect()
}

#[tokio::test]
async fn an_installed_skill_is_in_library_inactive_with_its_source() {
    let published = Published::new();
    published.publisher.publish("review", "review", "first");
    let engine = published.fixture.ready().await;
    // A trailing slash on the folder names the same source.
    let mut skill = published.publisher.skill("review", "review");
    skill.source.path.push('/');
    engine.install_skill(skill).await.unwrap();

    let snapshot = snapshot(&engine).await;
    assert_eq!(slugs(&snapshot), ["review"]);
    let library = snapshot.library.as_ref().unwrap();
    let installed = &library.library.skills[0];
    let source = SkillSource {
        url: published.publisher.url.clone(),
        reference: None,
        path: "skills/review".into(),
    };
    assert_eq!(installed.source, Some(source));
    assert!(installed.presets.is_empty(), "in no preset");
    assert!(installed.imported_at.is_some());
    assert!(snapshot.workspaces.iter().all(|w| w.skills.is_empty()));
    let record =
        fs::read_to_string(published.fixture.data.join("library/records/review.toml")).unwrap();
    assert!(record.contains(&format!("tree = \"{}\"", installed.recorded)));
    assert!(record.contains("skills/review"));
    assert!(!record.contains("reference"));
    let text = fs::read_to_string(
        published
            .fixture
            .data
            .join("library/skills/review/SKILL.md"),
    )
    .unwrap();
    assert!(text.contains("first"));
    assert!(!holds_directory(&published.fixture.home, "review"));
    assert!(!holds_directory(&published.fixture.workspace, "review"));
    assert!(snapshot.sync.uncommitted, "installs wait for Sync");
    assert!(snapshot.explore.installs.is_empty());
    assert_eq!(
        snapshot.updates[&SkillId("review".into())].check,
        UpdateCheck::UpToDate
    );
    // The install's staging directory and cache are gone.
    assert!(leftovers(&published.fixture.data, "staging").is_empty());
    assert!(leftovers(&published.fixture.data, "source-cache/install").is_empty());
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_newer_commit_gives_an_installed_skill_an_update() {
    let published = Published::new();
    published.publisher.publish("review", "review", "first");
    let engine = published.fixture.ready().await;
    engine
        .install_skill(published.publisher.skill("review", "review"))
        .await
        .unwrap();
    published.publisher.publish("review", "review", "second");
    engine.check_updates(Duration::ZERO).await.unwrap();
    let review = SkillId("review".into());
    assert_eq!(
        snapshot(&engine).await.updates[&review].check,
        UpdateCheck::Available
    );
    let outcome = engine.update_skills(vec![review.clone()]).await.unwrap();
    assert_eq!(outcome.updated, [review]);
    let text = fs::read_to_string(
        published
            .fixture
            .data
            .join("library/skills/review/SKILL.md"),
    )
    .unwrap();
    assert!(text.contains("second"));
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn install_refuses_a_slug_the_library_holds_even_one_held_while_it_ran() {
    let published = Published::new();
    published.publisher.publish("review", "review", "first");
    // Another folder publishes a skill with the same name, in another case.
    published.publisher.publish("review-copy", "Review", "copy");
    let engine = published.fixture.ready().await;

    // Both pass the first check before either is fetched; the commit that runs second finds
    // the slug held.
    let (first, second) = tokio::join!(
        engine.install_skill(published.publisher.skill("review", "review")),
        engine.install_skill(published.publisher.skill("review-copy", "Review")),
    );
    assert!(first.is_ok() != second.is_ok(), "exactly one lands");
    let refused = if first.is_ok() { second } else { first };
    assert_eq!(category(refused), ErrorCategory::Stale);
    let held = snapshot(&engine).await;
    assert_eq!(slugs(&held).len(), 1);
    // A refused install is idle: Explore shows the skill as in library or colliding instead.
    assert!(held.explore.installs.is_empty());

    for (folder, slug) in [("review", "review"), ("review-copy", "Review")] {
        let again = engine
            .install_skill(published.publisher.skill(folder, slug))
            .await;
        assert_eq!(category(again), ErrorCategory::Stale);
    }
    assert!(leftovers(&published.fixture.data, "staging").is_empty());
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn install_refuses_blocked_invalid_and_missing_folders() {
    let published = Published::new();
    published.publisher.publish("review", "review", "first");
    let blocked = published.publisher.dir.join("skills/blocked");
    write_skill(&published.publisher.dir.join("skills"), "blocked", "plugin");
    fs::create_dir(blocked.join(".codex-plugin")).unwrap();
    fs::write(blocked.join(".codex-plugin/plugin.json"), "{}").unwrap();
    let invalid = published.publisher.dir.join("skills/invalid");
    fs::create_dir_all(&invalid).unwrap();
    fs::write(invalid.join("SKILL.md"), "---\nname: [invalid\n---\n").unwrap();
    published.publisher.commit("refused");
    let engine = published.fixture.ready().await;

    for (folder, failure) in [
        ("blocked", InstallFailure::Blocked),
        ("invalid", InstallFailure::Invalid),
        ("gone", InstallFailure::NotFound),
    ] {
        let skill = published.publisher.skill(folder, folder);
        let key = install_key(&skill.source);
        assert_eq!(
            category(engine.install_skill(skill).await),
            ErrorCategory::Validation,
            "{folder}"
        );
        assert_eq!(
            engine.current_explore().installs.get(&key),
            Some(&InstallState::Failed(failure)),
            "{folder}"
        );
    }
    assert!(slugs(&snapshot(&engine).await).is_empty());
    assert!(leftovers(&published.fixture.data, "staging").is_empty());
    assert!(leftovers(&published.fixture.data, "source-cache/install").is_empty());

    // Installing again clears the failure; this time it succeeds.
    fs::remove_dir_all(&blocked).unwrap();
    write_skill(&published.publisher.dir.join("skills"), "blocked", "fixed");
    published.publisher.commit("fixed");
    let skill = published.publisher.skill("blocked", "blocked");
    let key = install_key(&skill.source);
    engine.install_skill(skill).await.unwrap();
    assert_eq!(engine.current_explore().installs.get(&key), None);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn an_unreadable_repository_fails_as_access_without_a_prompt() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let missing: String = url::Url::from_file_path(fixture.workspace.with_file_name("missing"))
        .unwrap()
        .into();
    let skill = catalog_skill(&missing, "skills/review", "review");
    let key = install_key(&skill.source);
    assert!(engine.install_skill(skill).await.is_err());
    assert_eq!(
        engine.current_explore().installs.get(&key),
        Some(&InstallState::Failed(InstallFailure::Access))
    );
    assert!(leftovers(&fixture.data, "staging").is_empty());
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn several_installs_from_one_repository_run_together_and_all_land() {
    let published = Published::new();
    let names = ["alpha", "beta", "gamma", "delta"];
    for name in names {
        write_skill(&published.publisher.dir.join("skills"), name, name);
    }
    published.publisher.commit("four");
    let engine = published.fixture.ready().await;
    let [a, b, c, d] =
        names.map(|name| engine.install_skill(published.publisher.skill(name, name)));
    let (a, b, c, d) = tokio::join!(a, b, c, d);
    for installed in [a, b, c, d] {
        installed.unwrap();
    }
    let mut held = slugs(&snapshot(&engine).await);
    held.sort();
    assert_eq!(held, ["alpha", "beta", "delta", "gamma"]);
    assert!(leftovers(&published.fixture.data, "source-cache/install").is_empty());
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn without_git_install_is_refused_and_says_so() {
    let fixture = Fixture::new();
    let engine = fixture.open_without_git();
    engine.skip_git().await.unwrap();
    support::choose_every_agent(&engine).await;
    engine.finish_onboarding(None).await.unwrap();
    let skill = catalog_skill("https://example.invalid/skills.git", "skills/a", "a");
    let key = install_key(&skill.source);
    assert!(engine.install_skill(skill).await.is_err());
    assert_eq!(
        engine.current_explore().installs.get(&key),
        Some(&InstallState::Failed(InstallFailure::Other(
            "Installing needs Git.".into()
        )))
    );
    engine.shutdown().await.unwrap();
}
