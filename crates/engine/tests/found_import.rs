//! Scan for skills: one plan prepared from a fresh scan for every offered skill, applied to the
//! ticked ones only (ADR 0009).
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/imports.rs"]
mod imports;
#[path = "support/fixture.rs"]
mod support;

use skillbinder_engine::Engine;
use skillbinder_proto::{AppError, CandidateId, ErrorCategory, ImportPlan, ValidationStatus};
use std::fs;
use support::{Fixture, write_skill};

/// The plan's skills by slug with their validation, sorted.
fn planned(plan: &ImportPlan) -> Vec<(String, ValidationStatus)> {
    let mut items: Vec<_> = plan
        .items
        .iter()
        .map(|item| (item.slug.clone(), item.validation))
        .collect();
    items.sort_by(|a, b| a.0.cmp(&b.0));
    items
}

/// The found skills the plan offers under `slugs`.
fn ticked(plan: &ImportPlan, slugs: &[&str]) -> Vec<CandidateId> {
    slugs
        .iter()
        .map(|slug| {
            let item = plan.items.iter().find(|item| item.slug == *slug).unwrap();
            item.candidate_id.clone()
        })
        .collect()
}

async fn library(engine: &Engine) -> Vec<String> {
    let snapshot = engine.snapshot().await.unwrap();
    let skills = snapshot.library.unwrap().library.skills;
    skills.iter().map(|skill| skill.id.to_string()).collect()
}

fn invalid(fixture: &Fixture, slug: &str) {
    let directory = fixture.workspace.join(".agents/skills").join(slug);
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("SKILL.md"),
        format!("---\nname: {slug}\n---\n"),
    )
    .unwrap();
}

fn refusal<T: std::fmt::Debug>(result: Result<T, AppError>) -> (ErrorCategory, String) {
    let error = result.unwrap_err();
    (error.category, error.message)
}

#[tokio::test]
async fn preparing_scans_afresh_and_plans_every_offered_skill_with_invalid_ones_marked() {
    let fixture = Fixture::new();
    fixture.skill("held", "Already imported");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    fixture.import_one(&engine, &scan.candidates[0]).await;
    // Written after the last scan, so only a fresh scan finds them.
    fixture.skill("alpha", "A");
    invalid(&fixture, "broken");

    let found = engine.prepare_found_import(Vec::new()).await.unwrap();

    assert_eq!(
        planned(&found.plan),
        [
            ("alpha".to_owned(), ValidationStatus::Valid),
            ("broken".to_owned(), ValidationStatus::Invalid),
        ]
    );
    assert_eq!(
        found.scan.candidates.len(),
        3,
        "the scan the plan came from"
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn import_takes_only_the_ticked_skills_and_the_others_are_offered_again() {
    let fixture = Fixture::new();
    fixture.skill("alpha", "A");
    fixture.skill("beta", "B");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let found = engine.prepare_found_import(Vec::new()).await.unwrap();

    let outcome = engine
        .apply_import(
            found.plan.id.clone(),
            ticked(&found.plan, &["alpha"]),
            false,
        )
        .await
        .unwrap();

    assert_eq!(outcome.imported.len(), 1);
    assert_eq!(library(&engine).await, ["alpha"]);
    let again = engine.prepare_found_import(Vec::new()).await.unwrap();
    assert_eq!(
        planned(&again.plan),
        [("beta".to_owned(), ValidationStatus::Valid)]
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_selection_outside_the_rules_is_refused_and_imports_nothing() {
    let fixture = Fixture::new();
    fixture.skill("alpha", "A");
    invalid(&fixture, "broken");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let found = engine.prepare_found_import(Vec::new()).await.unwrap();
    let plan = found.plan.id.clone();
    let alpha = ticked(&found.plan, &["alpha"]);
    let broken = ticked(&found.plan, &["broken"]);
    let too_many: Vec<CandidateId> = (0..501).map(|_| CandidateId::new()).collect();
    let cases = [
        (Vec::new(), "Choose skills to import."),
        (too_many, "Import at most 500 at a time."),
        ([alpha.clone(), alpha].concat(), "Choose each skill once."),
        (
            vec![CandidateId::new()],
            "A chosen skill is not in this import. Scan again.",
        ),
        (
            broken.clone(),
            "Acknowledge invalid skill metadata before importing.",
        ),
    ];
    for (selection, message) in cases {
        assert_eq!(
            refusal(engine.apply_import(plan.clone(), selection, false).await),
            (ErrorCategory::Validation, message.to_owned())
        );
    }
    assert!(library(&engine).await.is_empty());

    engine.apply_import(plan, broken, true).await.unwrap();
    assert_eq!(library(&engine).await, ["broken"]);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_changed_source_refuses_the_import_only_when_its_skill_is_ticked() {
    let fixture = Fixture::new();
    fixture.skill("alpha", "A");
    let beta = fixture.skill("beta", "B");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let found = engine.prepare_found_import(Vec::new()).await.unwrap();
    write_skill(beta.parent().unwrap(), "beta", "Changed after review");

    let plan = found.plan.id.clone();
    assert_eq!(
        refusal(
            engine
                .apply_import(plan.clone(), ticked(&found.plan, &["beta"]), false)
                .await
        )
        .0,
        ErrorCategory::Stale
    );
    engine
        .apply_import(plan, ticked(&found.plan, &["alpha"]), false)
        .await
        .unwrap();

    assert_eq!(library(&engine).await, ["alpha"]);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn another_version_replans_that_skill_alone_under_the_first_plans_revision_and_expiry() {
    let fixture = Fixture::new();
    imports::old_skill(&fixture.home.join(".claude/skills"), "review", "Older");
    fixture.skill("review", "Newer");
    fixture.skill("alpha", "A");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let found = engine.prepare_found_import(Vec::new()).await.unwrap();
    let older = found
        .scan
        .candidates
        .iter()
        .find(|candidate| candidate.slug == "review" && !candidate.preferred)
        .unwrap()
        .id
        .clone();

    let alpha = ticked(&found.plan, &["alpha"]);
    // The library changes after the first preparation.
    let plan = found.plan.id.clone();
    engine
        .apply_import(plan, alpha.clone(), false)
        .await
        .unwrap();

    let version = engine
        .prepare_import_version(found.plan.id.clone(), older.clone())
        .await
        .unwrap();

    assert_ne!(version.id, found.plan.id);
    assert_eq!(
        (&version.revision, version.expires_at),
        (&found.plan.revision, found.plan.expires_at)
    );
    assert_eq!(ticked(&version, &["review"]), std::slice::from_ref(&older));
    assert_eq!(ticked(&version, &["alpha"]), alpha);
    assert_eq!(
        refusal(engine.apply_import(version.id, vec![older], false).await).0,
        ErrorCategory::Stale
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn preparing_with_a_chosen_version_plans_it_instead_of_the_preferred_one() {
    let fixture = Fixture::new();
    imports::old_skill(&fixture.home.join(".claude/skills"), "review", "Older");
    fixture.skill("review", "Newer");
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    let versions: Vec<_> = scan.candidates.iter().map(|c| c.id.clone()).collect();
    let older = scan.candidates.iter().find(|c| !c.preferred).unwrap();

    let found = engine
        .prepare_found_import(vec![older.id.clone()])
        .await
        .unwrap();

    assert_eq!(
        ticked(&found.plan, &["review"]),
        std::slice::from_ref(&older.id)
    );
    assert_eq!(
        refusal(engine.prepare_found_import(versions).await),
        (
            ErrorCategory::Validation,
            "Choose one version of review to import.".to_owned()
        )
    );
    engine.shutdown().await.unwrap();
}
