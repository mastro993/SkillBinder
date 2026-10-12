//! Importing importable skills from Global and workspace pages, in isolated roots.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
mod support;

use skillbinder_engine::Engine;
use skillbinder_proto::{
    AppSnapshot, DeploymentTarget, ErrorCategory, ImportDecision, Importable, ValidationStatus,
};
use std::{fs, path::Path};
use support::{Fixture, category, write_skill};

fn importables(snapshot: &AppSnapshot, target: &DeploymentTarget) -> Vec<Importable> {
    snapshot
        .importables
        .iter()
        .filter(|importable| &importable.target == target)
        .cloned()
        .collect()
}

async fn workspace(fixture: &Fixture, engine: &Engine) -> DeploymentTarget {
    fixture.register(engine).await;
    let snapshot = engine.snapshot().await.unwrap();
    DeploymentTarget::Workspace(snapshot.workspaces[0].id.clone())
}

#[cfg(unix)]
fn inode(path: &Path) -> u64 {
    use std::os::unix::fs::MetadataExt;
    fs::metadata(path.join("SKILL.md")).unwrap().ino()
}

#[tokio::test]
async fn importing_from_a_workspace_records_its_existing_copies_as_the_deployment() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let agents = fixture.skill("review", "Review");
    let claude = write_skill(
        &fixture.workspace.join(".claude/skills"),
        "review",
        "Review",
    );
    let target = workspace(&fixture, &engine).await;
    #[cfg(unix)]
    let inodes = (inode(&agents), inode(&claude));
    let before = (
        fs::read(agents.join("SKILL.md")).unwrap(),
        fs::read(claude.join("SKILL.md")).unwrap(),
    );

    // Both copies hold the same content, so the page lists one importable with both paths.
    let snapshot = engine.snapshot().await.unwrap();
    let shown = importables(&snapshot, &target);
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].slug, "review");
    assert_eq!(shown[0].paths.len(), 2);
    assert!(snapshot.deployments.is_empty());

    let plan = engine
        .prepare_target_import(shown[0].clone(), false)
        .await
        .unwrap();
    assert_eq!(plan.deployment, Some(target.clone()));
    assert_eq!(plan.items.len(), 1);
    assert_eq!(plan.items[0].decision, ImportDecision::NewSkill);
    let outcome = engine.apply_import(plan.id.clone()).await.unwrap();
    assert_eq!(outcome.imported.len(), 1);

    let snapshot = engine.snapshot().await.unwrap();
    let library = &snapshot.library.as_ref().unwrap().library;
    assert_eq!(library.skills.len(), 1);
    assert_eq!(library.skills[0].id, outcome.imported[0]);
    assert_eq!(snapshot.deployments.len(), 1);
    let deployment = &snapshot.deployments[0];
    assert_eq!(deployment.target, target);
    assert_eq!(deployment.skill_id, outcome.imported[0]);
    assert_eq!(deployment.digest, library.skills[0].digest);
    assert_eq!(deployment.paths, shown[0].paths);
    assert!(deployment.changes.is_empty());
    assert_eq!(deployment.drift(&library.skills[0].digest), None);
    assert!(importables(&snapshot, &target).is_empty());

    // The copies were recorded, not rewritten.
    assert_eq!(
        before,
        (
            fs::read(agents.join("SKILL.md")).unwrap(),
            fs::read(claude.join("SKILL.md")).unwrap(),
        )
    );
    #[cfg(unix)]
    assert_eq!(inodes, (inode(&agents), inode(&claude)));

    // A rescan keeps the same clean deployment, and replaying the plan records nothing new.
    engine.rescan_target(target.clone()).await.unwrap();
    assert_eq!(
        engine.apply_import(plan.id).await.unwrap().imported,
        outcome.imported
    );
    let rescanned = engine.snapshot().await.unwrap();
    assert_eq!(rescanned.deployments, snapshot.deployments);
}

#[tokio::test]
async fn importing_from_global_records_the_global_copy() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    write_skill(&fixture.home.join(".claude/skills"), "notes", "Notes");
    engine
        .rescan_target(DeploymentTarget::Global)
        .await
        .unwrap();
    let shown = importables(&engine.snapshot().await.unwrap(), &DeploymentTarget::Global);
    assert_eq!(shown.len(), 1);

    let plan = engine
        .prepare_target_import(shown[0].clone(), false)
        .await
        .unwrap();
    engine.apply_import(plan.id).await.unwrap();

    let snapshot = engine.snapshot().await.unwrap();
    assert_eq!(snapshot.deployments.len(), 1);
    assert_eq!(snapshot.deployments[0].target, DeploymentTarget::Global);
    assert_eq!(snapshot.deployments[0].paths, shown[0].paths);
    assert!(snapshot.importables.is_empty());
}

#[tokio::test]
async fn an_importable_that_changed_since_it_was_shown_is_stale() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let copy = fixture.skill("review", "Review");
    let target = workspace(&fixture, &engine).await;
    let shown = importables(&engine.snapshot().await.unwrap(), &target).remove(0);

    write_skill(
        &fixture.workspace.join(".agents/skills"),
        "review",
        "Edited",
    );
    assert_eq!(
        category(engine.prepare_target_import(shown.clone(), false).await),
        ErrorCategory::Stale
    );

    // An importable the target's scan never listed is refused too.
    let mut unknown = shown;
    unknown.paths = vec![copy.display().to_string()];
    unknown.digest = "unknown".into();
    assert_eq!(
        category(engine.prepare_target_import(unknown, false).await),
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
}

#[tokio::test]
async fn invalid_metadata_needs_acknowledgement() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let invalid = fixture.workspace.join(".agents/skills/odd");
    fs::create_dir_all(&invalid).unwrap();
    fs::write(
        invalid.join("SKILL.md"),
        "---\nname: other\ndescription: Name differs\n---\n",
    )
    .unwrap();
    let target = workspace(&fixture, &engine).await;
    let shown = importables(&engine.snapshot().await.unwrap(), &target).remove(0);
    assert_eq!(shown.validation, ValidationStatus::Invalid);

    assert_eq!(
        category(engine.prepare_target_import(shown.clone(), false).await),
        ErrorCategory::Validation
    );
    let plan = engine.prepare_target_import(shown, true).await.unwrap();
    engine.apply_import(plan.id).await.unwrap();
    assert_eq!(engine.snapshot().await.unwrap().deployments.len(), 1);
}

#[cfg(unix)]
#[tokio::test]
async fn a_blocked_copy_stays_apart_and_cannot_be_imported() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let blocked = fixture.skill("review", "Shared");
    let outside = fixture.workspace.with_file_name("outside.txt");
    fs::write(&outside, b"outside").unwrap();
    std::os::unix::fs::symlink(&outside, blocked.join("extra")).unwrap();
    write_skill(
        &fixture.workspace.join(".claude/skills"),
        "review",
        "Shared",
    );
    let target = workspace(&fixture, &engine).await;

    // The digests match, but the validation results differ, so the copies stay apart.
    let shown = importables(&engine.snapshot().await.unwrap(), &target);
    assert_eq!(shown.len(), 2);
    let (refused, clean): (Vec<_>, Vec<_>) = shown
        .into_iter()
        .partition(|importable| importable.validation == ValidationStatus::Blocked);
    assert_eq!(
        category(engine.prepare_target_import(refused[0].clone(), true).await),
        ErrorCategory::Validation
    );
    let plan = engine
        .prepare_target_import(clean[0].clone(), false)
        .await
        .unwrap();
    engine.apply_import(plan.id).await.unwrap();
    let snapshot = engine.snapshot().await.unwrap();
    assert_eq!(snapshot.deployments[0].paths, clean[0].paths);
}

#[tokio::test]
async fn a_copy_changed_before_apply_is_left_out_of_the_deployment() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    fixture.skill("review", "Review");
    write_skill(
        &fixture.workspace.join(".claude/skills"),
        "review",
        "Review",
    );
    let target = workspace(&fixture, &engine).await;
    let shown = importables(&engine.snapshot().await.unwrap(), &target).remove(0);
    let plan = engine
        .prepare_target_import(shown.clone(), false)
        .await
        .unwrap();

    // The plan imports from the first copy; the second changes after review.
    let edited = write_skill(
        &fixture.workspace.join(".claude/skills"),
        "review",
        "Edited",
    );
    engine.apply_import(plan.id).await.unwrap();

    let snapshot = engine.snapshot().await.unwrap();
    assert_eq!(snapshot.deployments.len(), 1);
    let edited = fs::canonicalize(edited).unwrap().display().to_string();
    assert!(!snapshot.deployments[0].paths.contains(&edited));
    assert_eq!(snapshot.deployments[0].paths.len(), 1);

    // The changed copy is its own importable after the next rescan.
    engine.rescan_target(target.clone()).await.unwrap();
    let snapshot = engine.snapshot().await.unwrap();
    assert_eq!(importables(&snapshot, &target)[0].paths, vec![edited]);
    assert_eq!(snapshot.deployments[0].paths.len(), 1);
}

#[tokio::test]
async fn a_workspace_removed_before_apply_gets_no_deployment() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    fixture.skill("review", "Review");
    let target = workspace(&fixture, &engine).await;
    let shown = importables(&engine.snapshot().await.unwrap(), &target).remove(0);
    let plan = engine.prepare_target_import(shown, false).await.unwrap();
    let DeploymentTarget::Workspace(id) = target else {
        unreachable!()
    };
    engine.remove_workspace(id).await.unwrap();

    let outcome = engine.apply_import(plan.id).await.unwrap();
    assert_eq!(outcome.imported.len(), 1);
    let snapshot = engine.snapshot().await.unwrap();
    assert!(snapshot.deployments.is_empty());
    assert!(snapshot.importables.is_empty());
}

#[tokio::test]
async fn a_missing_workspace_folder_refuses_import() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    fixture.skill("review", "Review");
    let target = workspace(&fixture, &engine).await;
    let shown = importables(&engine.snapshot().await.unwrap(), &target).remove(0);
    let moved = fixture.workspace.with_file_name("moved");
    fs::rename(&fixture.workspace, &moved).unwrap();

    assert_eq!(
        category(engine.prepare_target_import(shown, false).await),
        ErrorCategory::Validation
    );
}

#[tokio::test]
async fn an_import_from_discovery_stops_listing_the_importable() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    fixture.skill("review", "Review");
    let target = workspace(&fixture, &engine).await;
    assert_eq!(
        importables(&engine.snapshot().await.unwrap(), &target).len(),
        1
    );

    let scan = fixture.scan(&engine).await;
    fixture
        .import_one(&engine, &scan.candidates[0], &scan)
        .await;
    let snapshot = engine.snapshot().await.unwrap();
    assert!(importables(&snapshot, &target).is_empty());
    // Discovery never changes deployments; the next rescan records the copy.
    assert!(snapshot.deployments.is_empty());
}

#[tokio::test]
async fn a_changed_deployed_copy_is_drift_and_not_importable() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    fixture.skill("review", "Review");
    let target = workspace(&fixture, &engine).await;
    let shown = importables(&engine.snapshot().await.unwrap(), &target).remove(0);
    let plan = engine.prepare_target_import(shown, false).await.unwrap();
    engine.apply_import(plan.id).await.unwrap();

    fixture.skill("review", "Edited");
    engine.rescan_target(target.clone()).await.unwrap();
    let snapshot = engine.snapshot().await.unwrap();
    assert_eq!(snapshot.deployments.len(), 1);
    assert_eq!(snapshot.deployments[0].changes.len(), 1);
    assert!(importables(&snapshot, &target).is_empty());
}

#[tokio::test]
async fn a_workspace_nested_before_apply_keeps_its_copies() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let outer = fixture.skill("review", "Review");
    let inner = fixture.workspace.join("packages/inner");
    write_skill(&inner.join(".agents/skills"), "review", "Review");
    let target = workspace(&fixture, &engine).await;
    let shown = importables(&engine.snapshot().await.unwrap(), &target).remove(0);
    assert_eq!(shown.paths.len(), 2);
    let plan = engine.prepare_target_import(shown, false).await.unwrap();

    support::add_workspace(&engine, &inner).await;
    engine.apply_import(plan.id).await.unwrap();
    let snapshot = engine.snapshot().await.unwrap();
    let outer_deployment = snapshot
        .deployments
        .iter()
        .find(|deployment| deployment.target == target)
        .unwrap();
    let outer = fs::canonicalize(outer).unwrap().display().to_string();
    assert_eq!(outer_deployment.paths, vec![outer]);
}
