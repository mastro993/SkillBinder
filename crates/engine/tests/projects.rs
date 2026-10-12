//! Global and projects: registration, deployment recording, and target scans in isolated roots.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
mod support;

use skillbinder_engine::Engine;
use skillbinder_proto::{AppSnapshot, Deployment, DeploymentTarget, ErrorCategory, SkillId};
use std::{fs, path::Path};
use support::{Fixture, add_project, category, write_skill};

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

fn project_target(snapshot: &AppSnapshot, name: &str) -> DeploymentTarget {
    let project = snapshot
        .projects
        .iter()
        .find(|project| project.name == name)
        .unwrap();
    DeploymentTarget::Project(project.id.clone())
}

#[tokio::test]
async fn adding_a_project_records_library_copies_and_lists_importables() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let alpha = library_skill(&fixture, &engine, "alpha", "Alpha").await;
    write_skill(&fixture.project.join(".agents/skills"), "alpha", "Alpha");
    write_skill(&fixture.project.join(".claude/skills"), "alpha", "Alpha");
    write_skill(&fixture.project.join(".claude/skills"), "beta", "Beta");
    write_skill(
        &fixture.project.join("docs"),
        "loose",
        "Outside skill directories",
    );

    let addition = fixture.register(&engine).await;
    assert_eq!(addition.name, "project");
    assert_eq!(addition.in_library, 1);
    assert_eq!(addition.importable, ["beta"]);
    assert!(addition.folders_checked > 0);

    let snapshot = engine.snapshot().await.unwrap();
    let project = &snapshot.projects[0];
    assert_eq!((project.name.as_str(), project.position), ("project", 0));
    assert!(!project.missing);
    for agent in ["claude-code", "codex"] {
        assert!(project.agents.iter().any(|id| id == agent), "{agent}");
    }
    let target = DeploymentTarget::Project(project.id.clone());
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
async fn grants_are_single_use_and_duplicate_projects_are_refused() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let grant = engine
        .grant_directory(fixture.project.clone())
        .await
        .unwrap();
    assert!(
        engine
            .add_project(grant.id.clone())
            .await
            .unwrap()
            .is_some()
    );
    engine.dismiss_project_addition().unwrap();
    assert_eq!(
        category(engine.add_project(grant.id).await),
        ErrorCategory::Stale
    );
    let target = project_target(&engine.snapshot().await.unwrap(), "project");
    let DeploymentTarget::Project(id) = target else {
        unreachable!()
    };
    engine.rename_project(id, "Renamed".into()).await.unwrap();
    let duplicate = engine
        .grant_directory(fixture.project.join("."))
        .await
        .unwrap();
    let error = engine.add_project(duplicate.id).await.unwrap_err();
    assert_eq!(error.category, ErrorCategory::Validation);
    assert!(error.message.contains("Renamed"), "{}", error.message);
    assert_eq!(engine.snapshot().await.unwrap().projects.len(), 1);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn projects_are_renamed_reordered_and_removed_without_touching_files() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    library_skill(&fixture, &engine, "alpha", "Alpha").await;
    let second = fixture.project.parent().unwrap().join("second");
    let copy = write_skill(&fixture.project.join(".agents/skills"), "alpha", "Alpha");
    write_skill(&second.join(".agents/skills"), "alpha", "Alpha");
    fixture.register(&engine).await;
    add_project(&engine, &second).await;

    let snapshot = engine.snapshot().await.unwrap();
    let names: Vec<_> = snapshot.projects.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["project", "second"]);
    let (first, other) = (
        snapshot.projects[0].id.clone(),
        snapshot.projects[1].id.clone(),
    );
    engine
        .rename_project(other.clone(), "project".into())
        .await
        .unwrap();
    assert_eq!(
        category(engine.rename_project(other.clone(), " ".into()).await),
        ErrorCategory::Validation
    );
    assert_eq!(
        category(engine.reorder_projects(vec![other.clone()]).await),
        ErrorCategory::Stale
    );
    engine
        .reorder_projects(vec![other.clone(), first.clone()])
        .await
        .unwrap();
    let snapshot = engine.snapshot().await.unwrap();
    let order: Vec<_> = snapshot.projects.iter().map(|p| &p.id).collect();
    assert_eq!(order, [&other, &first]);
    assert!(snapshot.projects.iter().all(|p| p.name == "project"));

    let bytes = fs::read(copy.join("SKILL.md")).unwrap();
    assert!(
        engine
            .remove_project(first.clone())
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(fs::read(copy.join("SKILL.md")).unwrap(), bytes);
    let snapshot = engine.snapshot().await.unwrap();
    assert_eq!(snapshot.projects.len(), 1);
    assert_eq!(snapshot.projects[0].position, 0);
    assert!(deployments_of(&snapshot, &DeploymentTarget::Project(first)).is_empty());
    assert_eq!(
        deployments_of(&snapshot, &DeploymentTarget::Project(other)).len(),
        1
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn the_deepest_project_holds_skills_inside_nested_projects() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let outer_skill = library_skill(&fixture, &engine, "outer", "Outer").await;
    let inner_skill = library_skill(&fixture, &engine, "inner", "Inner").await;
    let inner = fixture.project.join("packages/inner");
    write_skill(&fixture.project.join(".agents/skills"), "outer", "Outer");
    write_skill(&inner.join(".claude/skills"), "inner", "Inner");

    assert_eq!(fixture.register(&engine).await.in_library, 2);
    assert_eq!(add_project(&engine, &inner).await.in_library, 1);
    let skills = |snapshot: &AppSnapshot, name: &str| -> Vec<SkillId> {
        deployments_of(snapshot, &project_target(snapshot, name))
            .iter()
            .map(|deployment| deployment.skill_id.clone())
            .collect()
    };
    let snapshot = engine.snapshot().await.unwrap();
    assert_eq!(
        skills(&snapshot, "project"),
        std::slice::from_ref(&outer_skill)
    );
    assert_eq!(
        skills(&snapshot, "inner"),
        std::slice::from_ref(&inner_skill)
    );

    let outer = project_target(&snapshot, "project");
    engine.rescan_target(outer.clone()).await.unwrap();
    let snapshot = engine.snapshot().await.unwrap();
    assert_eq!(
        skills(&snapshot, "project"),
        std::slice::from_ref(&outer_skill)
    );

    let DeploymentTarget::Project(inner_id) = project_target(&snapshot, "inner") else {
        unreachable!()
    };
    let containing = engine.remove_project(inner_id).await.unwrap();
    assert_eq!(containing.len(), 1);
    engine.rescan_target(outer).await.unwrap();
    let mut expected = vec![outer_skill, inner_skill];
    expected.sort();
    assert_eq!(
        skills(&engine.snapshot().await.unwrap(), "project"),
        expected
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_missing_or_unreadable_project_folder_keeps_its_deployments() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    library_skill(&fixture, &engine, "alpha", "Alpha").await;
    fixture.skill("alpha", "Alpha");
    fixture.register(&engine).await;
    let moved = fixture.project.with_file_name("moved");
    fs::rename(&fixture.project, &moved).unwrap();

    let snapshot = engine.snapshot().await.unwrap();
    assert!(snapshot.projects[0].missing);
    let target = project_target(&snapshot, "project");
    engine.rescan_target(target.clone()).await.unwrap();
    let snapshot = engine.snapshot().await.unwrap();
    assert_eq!(deployments_of(&snapshot, &target).len(), 1);
    assert!(fixture.scan(&engine).await.candidates.is_empty());

    fs::rename(&moved, &fixture.project).unwrap();
    assert!(!engine.snapshot().await.unwrap().projects[0].missing);

    // An unreadable folder is treated like a missing one.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&fixture.project, fs::Permissions::from_mode(0o000)).unwrap();
        let rescanned = engine.rescan_target(target.clone()).await;
        fs::set_permissions(&fixture.project, fs::Permissions::from_mode(0o755)).unwrap();
        rescanned.unwrap();
        let snapshot = engine.snapshot().await.unwrap();
        assert_eq!(deployments_of(&snapshot, &target).len(), 1);
    }

    // A rescan of a removed project is a no-op rather than an error.
    let DeploymentTarget::Project(id) = target.clone() else {
        unreachable!()
    };
    engine.remove_project(id).await.unwrap();
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
