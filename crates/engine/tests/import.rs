//! Onboarding, scanning, and installing skills end to end with isolated storage and unchanged
//! source fixtures.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/expiry.rs"]
mod expiry;
#[path = "support/fixture.rs"]
mod support;

use skillbinder_engine::{Engine, EngineConfig};
use skillbinder_proto::{ErrorCategory, ImportDecision, OnboardingStep, SkillId};
use std::{fs, path::Path};
use support::{Fixture, category};

#[tokio::test]
async fn onboarding_requires_each_step_and_survives_restart() {
    let fixture = Fixture::new();
    let engine = fixture.open();
    assert_eq!(
        category(engine.complete_onboarding().await),
        ErrorCategory::Validation
    );
    assert_eq!(
        category(engine.set_onboarding(OnboardingStep::Ready).await),
        ErrorCategory::Validation
    );
    engine
        .set_onboarding(OnboardingStep::Boundaries)
        .await
        .unwrap();
    engine.shutdown().await.unwrap();
    let engine = fixture.open();
    assert_eq!(
        engine.bootstrap().await.unwrap().step,
        OnboardingStep::Boundaries
    );
    engine
        .set_onboarding(OnboardingStep::SyncChoice)
        .await
        .unwrap();
    engine.set_onboarding(OnboardingStep::Ready).await.unwrap();
    let complete = engine.complete_onboarding().await.unwrap();
    assert_eq!(complete.step, OnboardingStep::Complete);
    assert_eq!(
        engine.complete_onboarding().await.unwrap().library_path,
        complete.library_path
    );
    assert!(Path::new(&complete.library_path).join("skills").is_dir());
    engine.shutdown().await.unwrap();
    let engine = fixture.open();
    assert_eq!(
        engine.bootstrap().await.unwrap().step,
        OnboardingStep::Complete
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn scan_deduplicates_shared_readers_and_stays_inside_known_workspace_locations() {
    let fixture = Fixture::new();
    fixture.skill("known", "Expected source");
    let unrelated = fixture.workspace.join("docs/unknown");
    fs::create_dir_all(&unrelated).unwrap();
    fs::write(
        unrelated.join("SKILL.md"),
        b"---\nname: unknown\ndescription: Unrelated\n---\n",
    )
    .unwrap();
    let engine = fixture.ready().await;
    let workspace = fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    assert_eq!(scan.candidates.len(), 1);
    assert_eq!(scan.candidates[0].slug, "known");
    assert_eq!(scan.candidates[0].locations.len(), 1);
    let location = &scan.candidates[0].locations[0];
    assert!(location.readers.len() > 1);
    assert_eq!(location.agents.len(), location.readers.len());
    assert!(location.agents.iter().any(|agent| agent == "codex"));
    assert_eq!(location.workspaces, [workspace.id]);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_skill_the_last_scan_no_longer_finds_cannot_be_prepared() {
    let fixture = Fixture::new();
    let source = fixture.skill("alpha", "A");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    let found = scan.candidates[0].id.clone();
    fs::remove_dir_all(&source).unwrap();
    assert!(fixture.scan(&engine).await.candidates.is_empty());
    assert_eq!(
        category(engine.prepare_import(vec![found], false).await),
        ErrorCategory::Stale
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn import_preserves_source_bytes_and_replays_after_restart() {
    let fixture = Fixture::new();
    let source = fixture.skill("alpha", "Stable original source");
    let original = fs::read(source.join("SKILL.md")).unwrap();
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    let plan = engine
        .prepare_import(vec![scan.candidates[0].id.clone()], false)
        .await
        .unwrap();
    assert_eq!(plan.items[0].decision, ImportDecision::NewSkill);
    let first = engine.apply_import(plan.id.clone()).await.unwrap();
    assert_eq!(fs::read(source.join("SKILL.md")).unwrap(), original);
    assert_eq!(engine.apply_import(plan.id.clone()).await.unwrap(), first);
    engine.shutdown().await.unwrap();
    let engine = fixture.open();
    assert_eq!(engine.apply_import(plan.id).await.unwrap(), first);
    assert_eq!(
        engine.snapshot().await.unwrap().import_outcome,
        Some(first.clone())
    );
    let library = engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(library.library.skills.len(), 1);
    let skill = &library.library.skills[0];
    assert!(
        skill
            .imported_at
            .as_deref()
            .is_some_and(|time| time.ends_with('Z')),
        "imports record an RFC 3339 UTC time"
    );
    assert!(
        library.modified[&skill.id] > 0,
        "the managed payload's newest file time is reported"
    );
    engine.dismiss_import().await.unwrap();
    assert_eq!(engine.snapshot().await.unwrap().import_outcome, None);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn changed_source_refuses_whole_prepared_import() {
    let fixture = Fixture::new();
    let source = fixture.skill("alpha", "Original");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    let plan = engine
        .prepare_import(vec![scan.candidates[0].id.clone()], false)
        .await
        .unwrap();
    fs::write(
        source.join("SKILL.md"),
        b"---\nname: alpha\ndescription: Changed\n---\nChanged\n",
    )
    .unwrap();
    assert_eq!(
        category(engine.apply_import(plan.id).await),
        ErrorCategory::Stale
    );
    assert!(
        engine
            .snapshot()
            .await
            .unwrap()
            .library
            .unwrap()
            .library
            .skills
            .is_empty()
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn preview_rejects_escape_and_bounds_binary_content() {
    let fixture = Fixture::new();
    let source = fixture.skill("alpha", "Content");
    fs::write(source.join("binary.dat"), [0, 1, 2, 3]).unwrap();
    fs::write(source.join("large.txt"), vec![b'x'; 128 * 1024 + 1]).unwrap();
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    let skill = fixture.import_one(&engine, &scan.candidates[0]).await;
    assert_eq!(
        category(
            engine
                .preview_skill(skill.clone(), Some("../../skillbinder.db".into()))
                .await
        ),
        ErrorCategory::Validation
    );
    let binary = engine
        .preview_skill(skill.clone(), Some("binary.dat".into()))
        .await
        .unwrap();
    assert!(binary.text.is_none() && binary.unavailable.as_deref().unwrap().contains("Binary"));
    let large = engine
        .preview_skill(skill, Some("large.txt".into()))
        .await
        .unwrap();
    assert!(large.text.is_none() && large.unavailable.as_deref().unwrap().contains("128 KiB"));
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_different_skill_with_a_held_slug_is_refused() {
    let fixture = Fixture::new();
    let source = fixture.skill("shared", "First copy");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let first_scan = fixture.scan(&engine).await;
    let first = fixture.import_one(&engine, &first_scan.candidates[0]).await;
    assert_eq!(first, SkillId("shared".into()));
    fs::write(
        source.join("SKILL.md"),
        b"---\nname: shared\ndescription: Changed\n---\nSecond copy\n",
    )
    .unwrap();
    let second_scan = fixture.scan(&engine).await;
    assert_eq!(
        category(
            engine
                .prepare_import(vec![second_scan.candidates[0].id.clone()], false)
                .await
        ),
        ErrorCategory::Validation
    );
    let library = engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(library.library.skills.len(), 1);
    let managed =
        Path::new(&engine.bootstrap().await.unwrap().library_path).join("skills/shared/SKILL.md");
    assert!(fs::read_to_string(managed).unwrap().contains("First copy"));
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_skill_without_files_is_refused_even_when_acknowledged() {
    let fixture = Fixture::new();
    let source = fixture.skill("empty", "Soon gone");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    fs::remove_file(source.join("SKILL.md")).unwrap();
    assert_eq!(
        category(
            engine
                .prepare_import(vec![scan.candidates[0].id.clone()], true)
                .await
        ),
        ErrorCategory::Validation
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn an_unreconciled_skill_directory_blocks_an_install_of_its_slug() {
    let fixture = Fixture::new();
    fixture.skill("alpha", "Workspace copy");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    let ids = vec![scan.candidates[0].id.clone()];
    let plan = engine.prepare_import(ids.clone(), false).await.unwrap();
    let skills = fixture.data.join("library/skills");
    support::write_skill(&skills, "alpha", "Appeared outside the app");
    assert_eq!(
        category(engine.apply_import(plan.id).await),
        ErrorCategory::Stale
    );
    assert_eq!(
        category(engine.prepare_import(ids, false).await),
        ErrorCategory::Validation
    );
    engine.shutdown().await.unwrap();
    let engine = fixture.open();
    let library = engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(library.library.skills.len(), 1, "no import journal is left");
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn two_engine_instances_cannot_own_one_library() {
    let fixture = Fixture::new();
    let engine = fixture.open();
    assert_eq!(
        category(Engine::open(EngineConfig::isolated(
            fixture.data.clone(),
            fixture.home.clone()
        ))),
        ErrorCategory::Busy
    );
    engine.shutdown().await.unwrap();
    let reopened = fixture.open();
    reopened.shutdown().await.unwrap();
}
