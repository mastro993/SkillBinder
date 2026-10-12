//! Global and workspaces: registration, deployment recording, and target scans in isolated roots.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
mod support;

use skillbinder_engine::Engine;
use skillbinder_proto::{AppSnapshot, Deployment, DeploymentTarget, ErrorCategory, SkillId};
use std::{fs, path::Path};
use support::{Fixture, add_workspace, category, write_skill};

/// Imports a skill from the fixture home's Claude Code directory, which Global also covers.
async fn library_skill(fixture: &Fixture, engine: &Engine, slug: &str, body: &str) -> SkillId {
    write_skill(&fixture.home.join(".claude/skills"), slug, body);
    let scan = fixture.scan(engine).await;
    let candidate = scan
        .candidates
        .iter()
        .find(|candidate| candidate.slug == slug)
        .unwrap();
    fixture.import_one(engine, candidate, &scan).await
}

fn deployments_of<'a>(snapshot: &'a AppSnapshot, target: &DeploymentTarget) -> Vec<&'a Deployment> {
    snapshot
        .deployments
        .iter()
        .filter(|deployment| &deployment.target == target)
        .collect()
}

fn workspace_target(snapshot: &AppSnapshot, name: &str) -> DeploymentTarget {
    let workspace = snapshot
        .workspaces
        .iter()
        .find(|workspace| workspace.name == name)
        .unwrap();
    DeploymentTarget::Workspace(workspace.id.clone())
}

#[tokio::test]
async fn adding_a_workspace_records_library_copies_and_lists_importables() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let alpha = library_skill(&fixture, &engine, "alpha", "Alpha").await;
    write_skill(&fixture.workspace.join(".agents/skills"), "alpha", "Alpha");
    write_skill(&fixture.workspace.join(".claude/skills"), "alpha", "Alpha");
    write_skill(&fixture.workspace.join(".claude/skills"), "beta", "Beta");
    write_skill(
        &fixture.workspace.join("docs"),
        "loose",
        "Outside skill directories",
    );

    let addition = fixture.register(&engine).await;
    assert_eq!(addition.name, "workspace");
    assert_eq!(addition.in_library, 1);
    assert_eq!(addition.importable, ["beta"]);
    assert!(addition.folders_checked > 0);

    let snapshot = engine.snapshot().await.unwrap();
    let workspace = &snapshot.workspaces[0];
    assert_eq!(
        (workspace.name.as_str(), workspace.position),
        ("workspace", 0)
    );
    assert!(!workspace.missing);
    for agent in ["claude-code", "codex"] {
        assert!(workspace.agents.iter().any(|id| id == agent), "{agent}");
    }
    let target = DeploymentTarget::Workspace(workspace.id.clone());
    let recorded = deployments_of(&snapshot, &target);
    let library = snapshot.library.as_ref().unwrap();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].skill_id, alpha);
    assert_eq!(recorded[0].digest, library.library.skills[0].digest);
    assert_eq!(recorded[0].paths.len(), 2);
    assert!(deployments_of(&snapshot, &DeploymentTarget::Global).is_empty());

    engine
        .rescan_target(DeploymentTarget::Global)
        .await
        .unwrap();
    let snapshot = engine.snapshot().await.unwrap();
    assert!(snapshot.global_agents.iter().any(|id| id == "claude-code"));
    let home = fs::canonicalize(&fixture.home).unwrap();
    assert_eq!(snapshot.home, home.display().to_string());
    let global = deployments_of(&snapshot, &DeploymentTarget::Global);
    assert_eq!(global.len(), 1);
    assert_eq!(global[0].skill_id, alpha);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn grants_are_single_use_and_duplicate_workspaces_are_refused() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let grant = engine
        .grant_directory(fixture.workspace.clone())
        .await
        .unwrap();
    assert!(
        engine
            .add_workspace(grant.id.clone())
            .await
            .unwrap()
            .is_some()
    );
    engine.dismiss_workspace_addition().unwrap();
    assert_eq!(
        category(engine.add_workspace(grant.id).await),
        ErrorCategory::Stale
    );
    let target = workspace_target(&engine.snapshot().await.unwrap(), "workspace");
    let DeploymentTarget::Workspace(id) = target else {
        unreachable!()
    };
    engine.rename_workspace(id, "Renamed".into()).await.unwrap();
    let duplicate = engine
        .grant_directory(fixture.workspace.join("."))
        .await
        .unwrap();
    let error = engine.add_workspace(duplicate.id).await.unwrap_err();
    assert_eq!(error.category, ErrorCategory::Validation);
    assert!(error.message.contains("Renamed"), "{}", error.message);
    assert_eq!(engine.snapshot().await.unwrap().workspaces.len(), 1);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn workspaces_are_renamed_reordered_and_removed_without_touching_files() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    library_skill(&fixture, &engine, "alpha", "Alpha").await;
    let second = fixture.workspace.parent().unwrap().join("second");
    let copy = write_skill(&fixture.workspace.join(".agents/skills"), "alpha", "Alpha");
    write_skill(&second.join(".agents/skills"), "alpha", "Alpha");
    fixture.register(&engine).await;
    add_workspace(&engine, &second).await;

    let snapshot = engine.snapshot().await.unwrap();
    let names: Vec<_> = snapshot
        .workspaces
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    assert_eq!(names, ["workspace", "second"]);
    let (first, other) = (
        snapshot.workspaces[0].id.clone(),
        snapshot.workspaces[1].id.clone(),
    );
    engine
        .rename_workspace(other.clone(), "workspace".into())
        .await
        .unwrap();
    assert_eq!(
        category(engine.rename_workspace(other.clone(), " ".into()).await),
        ErrorCategory::Validation
    );
    assert_eq!(
        category(engine.reorder_workspaces(vec![other.clone()]).await),
        ErrorCategory::Stale
    );
    engine
        .reorder_workspaces(vec![other.clone(), first.clone()])
        .await
        .unwrap();
    let snapshot = engine.snapshot().await.unwrap();
    let order: Vec<_> = snapshot.workspaces.iter().map(|p| &p.id).collect();
    assert_eq!(order, [&other, &first]);
    assert!(snapshot.workspaces.iter().all(|p| p.name == "workspace"));

    let bytes = fs::read(copy.join("SKILL.md")).unwrap();
    assert!(
        engine
            .remove_workspace(first.clone())
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(fs::read(copy.join("SKILL.md")).unwrap(), bytes);
    let snapshot = engine.snapshot().await.unwrap();
    assert_eq!(snapshot.workspaces.len(), 1);
    assert_eq!(snapshot.workspaces[0].position, 0);
    assert!(deployments_of(&snapshot, &DeploymentTarget::Workspace(first)).is_empty());
    assert_eq!(
        deployments_of(&snapshot, &DeploymentTarget::Workspace(other)).len(),
        1
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn the_deepest_workspace_holds_skills_inside_nested_workspaces() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let outer_skill = library_skill(&fixture, &engine, "outer", "Outer").await;
    let inner_skill = library_skill(&fixture, &engine, "inner", "Inner").await;
    let inner = fixture.workspace.join("packages/inner");
    write_skill(&fixture.workspace.join(".agents/skills"), "outer", "Outer");
    write_skill(&inner.join(".claude/skills"), "inner", "Inner");

    assert_eq!(fixture.register(&engine).await.in_library, 2);
    assert_eq!(add_workspace(&engine, &inner).await.in_library, 1);
    let skills = |snapshot: &AppSnapshot, name: &str| -> Vec<SkillId> {
        deployments_of(snapshot, &workspace_target(snapshot, name))
            .iter()
            .map(|deployment| deployment.skill_id.clone())
            .collect()
    };
    let snapshot = engine.snapshot().await.unwrap();
    assert_eq!(
        skills(&snapshot, "workspace"),
        std::slice::from_ref(&outer_skill)
    );
    assert_eq!(
        skills(&snapshot, "inner"),
        std::slice::from_ref(&inner_skill)
    );

    let outer = workspace_target(&snapshot, "workspace");
    engine.rescan_target(outer.clone()).await.unwrap();
    let snapshot = engine.snapshot().await.unwrap();
    assert_eq!(
        skills(&snapshot, "workspace"),
        std::slice::from_ref(&outer_skill)
    );

    let DeploymentTarget::Workspace(inner_id) = workspace_target(&snapshot, "inner") else {
        unreachable!()
    };
    let containing = engine.remove_workspace(inner_id).await.unwrap();
    assert_eq!(containing.len(), 1);
    engine.rescan_target(outer).await.unwrap();
    let mut expected = vec![outer_skill, inner_skill];
    expected.sort();
    assert_eq!(
        skills(&engine.snapshot().await.unwrap(), "workspace"),
        expected
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_missing_or_unreadable_workspace_folder_keeps_its_deployments() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    library_skill(&fixture, &engine, "alpha", "Alpha").await;
    fixture.skill("alpha", "Alpha");
    fixture.register(&engine).await;
    let moved = fixture.workspace.with_file_name("moved");
    fs::rename(&fixture.workspace, &moved).unwrap();

    let snapshot = engine.snapshot().await.unwrap();
    assert!(snapshot.workspaces[0].missing);
    let target = workspace_target(&snapshot, "workspace");
    engine.rescan_target(target.clone()).await.unwrap();
    let snapshot = engine.snapshot().await.unwrap();
    assert_eq!(deployments_of(&snapshot, &target).len(), 1);
    assert!(fixture.scan(&engine).await.candidates.is_empty());

    fs::rename(&moved, &fixture.workspace).unwrap();
    assert!(!engine.snapshot().await.unwrap().workspaces[0].missing);

    // An unreadable folder is treated like a missing one.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&fixture.workspace, fs::Permissions::from_mode(0o000)).unwrap();
        let rescanned = engine.rescan_target(target.clone()).await;
        fs::set_permissions(&fixture.workspace, fs::Permissions::from_mode(0o755)).unwrap();
        rescanned.unwrap();
        let snapshot = engine.snapshot().await.unwrap();
        assert_eq!(deployments_of(&snapshot, &target).len(), 1);
    }

    // A rescan of a removed workspace is a no-op rather than an error.
    let DeploymentTarget::Workspace(id) = target.clone() else {
        unreachable!()
    };
    engine.remove_workspace(id).await.unwrap();
    engine.rescan_target(target.clone()).await.unwrap();
    assert!(
        engine
            .snapshot()
            .await
            .unwrap()
            .deployments
            .iter()
            .all(|d| d.target != target)
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_deployed_skill_cannot_be_deleted_from_the_library() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let first = library_skill(&fixture, &engine, "shared", "First copy").await;
    let source = fixture.home.join(".claude/skills/shared");
    write_skill(source.parent().unwrap(), "shared", "Second copy");
    let scan = fixture.scan(&engine).await;
    let plan = engine
        .prepare_import(scan.id, vec![scan.candidates[0].id.clone()], false)
        .await
        .unwrap();
    let second = engine.apply_import(plan.id).await.unwrap().imported[0].clone();
    engine
        .rescan_target(DeploymentTarget::Global)
        .await
        .unwrap();

    let library = engine.snapshot().await.unwrap().library.unwrap();
    let copies = vec![first.clone(), second.clone()];
    let error = engine
        .resolve_conflict(
            "shared".into(),
            first,
            copies.clone(),
            library.revision.clone(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.category, ErrorCategory::Validation);
    assert!(
        error.message.contains("shared is deployed to Global"),
        "{}",
        error.message
    );
    let unchanged = engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(unchanged.library.skills.len(), 2);
    assert!(Path::new(&fixture.data.join("library/skills/shared")).is_dir());

    let resolved = engine
        .resolve_conflict("shared".into(), second, copies, unchanged.revision)
        .await
        .unwrap();
    assert_eq!(resolved.library.skills.len(), 1);
    engine.shutdown().await.unwrap();
}
