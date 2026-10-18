//! Workspaces: registration, folders of projects, renaming, reordering, and removal, and the
//! single scan of their agent skill directories, in isolated roots.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/folders.rs"]
mod folders;
#[path = "support/fixture.rs"]
mod support;

use folders::add_folder;
use skillbinder_engine::Engine;
use skillbinder_proto::{ErrorCategory, ScanSnapshot, SkillId};
use std::fs;
use support::{Fixture, add_workspace, category, write_skill};

/// Imports a skill from the fixture home's Claude Code global skill directory.
async fn library_skill(fixture: &Fixture, engine: &Engine, slug: &str, body: &str) -> SkillId {
    write_skill(&fixture.home.join(".claude/skills"), slug, body);
    let scan = fixture.scan(engine).await;
    let candidate = scan
        .candidates
        .iter()
        .find(|candidate| candidate.slug == slug)
        .unwrap();
    fixture.import_one(engine, candidate).await
}

/// Paths of every found skill folder, in order.
fn paths(scan: &ScanSnapshot) -> Vec<String> {
    let mut paths: Vec<_> = scan
        .candidates
        .iter()
        .flat_map(|candidate| &candidate.locations)
        .map(|location| location.display_path.clone())
        .collect();
    paths.sort();
    paths
}

#[tokio::test]
async fn a_workspace_scan_reads_only_its_top_level_agent_skill_directories() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let alpha = library_skill(&fixture, &engine, "alpha", "Alpha").await;
    write_skill(&fixture.workspace.join(".agents/skills"), "alpha", "Alpha");
    write_skill(&fixture.workspace.join(".claude/skills"), "beta", "Beta");
    write_skill(&fixture.workspace.join("docs"), "loose", "Outside");
    write_skill(
        &fixture.workspace.join("nested/inner/.claude/skills"),
        "deep",
        "Below the top",
    );

    let workspace = fixture.register(&engine).await;
    assert_eq!(
        (workspace.name.as_str(), workspace.position),
        ("workspace", 0)
    );
    assert!(!workspace.missing);
    assert_eq!(
        engine.snapshot().await.unwrap().workspaces,
        std::slice::from_ref(&workspace)
    );

    let scan = fixture.scan(&engine).await;
    let root = fs::canonicalize(&fixture.workspace).unwrap();
    let home = fs::canonicalize(&fixture.home).unwrap();
    assert_eq!(
        paths(&scan),
        [
            home.join(".claude/skills/alpha"),
            root.join(".agents/skills/alpha"),
            root.join(".claude/skills/beta"),
        ]
        .map(|path| path.display().to_string())
    );
    let found = |slug: &str| {
        scan.candidates
            .iter()
            .find(|candidate| candidate.slug == slug)
            .unwrap()
    };
    assert_eq!(found("alpha").skill_id, Some(alpha));
    assert_eq!(found("alpha").locations.len(), 2);
    assert_eq!(found("beta").skill_id, None);
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
    let workspace = engine
        .add_workspaces(grant.id.clone())
        .await
        .unwrap()
        .remove(0);
    assert_eq!(
        category(engine.add_workspaces(grant.id).await),
        ErrorCategory::Stale
    );
    engine
        .rename_workspace(workspace.id, "Renamed".into())
        .await
        .unwrap();
    let duplicate = engine
        .grant_directory(fixture.workspace.join("."))
        .await
        .unwrap();
    let error = engine.add_workspaces(duplicate.id).await.unwrap_err();
    assert_eq!(error.category, ErrorCategory::Validation);
    assert!(error.message.contains("Renamed"), "{}", error.message);
    assert_eq!(engine.snapshot().await.unwrap().workspaces.len(), 1);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_folder_of_projects_adds_each_project_once() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let projects = fixture.workspace.join("projects");
    write_skill(&projects.join("api/.claude/skills"), "alpha", "Alpha");
    fs::create_dir_all(projects.join("web/.agents/skills")).unwrap();
    fs::create_dir_all(projects.join("docs")).unwrap();
    let canonical = fs::canonicalize(&projects).unwrap();

    let added = add_folder(&engine, &projects).await.unwrap();
    let names: Vec<_> = added
        .iter()
        .map(|workspace| workspace.name.as_str())
        .collect();
    assert_eq!(names, ["api", "web"]);
    assert_eq!(added[0].path, canonical.join("api").display().to_string());
    assert_eq!(added[1].position, 1);
    let slugs: Vec<_> = fixture
        .scan(&engine)
        .await
        .candidates
        .into_iter()
        .map(|candidate| candidate.slug)
        .collect();
    assert_eq!(slugs, ["alpha"]);

    // Picking the folder again adds only projects that appeared since.
    fs::create_dir_all(projects.join("docs/.claude/skills")).unwrap();
    let added = add_folder(&engine, &projects).await.unwrap();
    assert_eq!(added.len(), 1);
    assert_eq!((added[0].name.as_str(), added[0].position), ("docs", 2));
    let error = add_folder(&engine, &projects).await.unwrap_err();
    assert_eq!(error.category, ErrorCategory::Validation);
    assert_eq!(error.message, "Every project in projects is already added.");
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_folder_of_thousands_of_projects_is_added() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let projects = fixture.workspace.join("projects");
    // More rows than one SQLite statement can bind at five values each.
    for index in 0..7000 {
        fs::create_dir_all(projects.join(format!("{index}/.agents/skills"))).unwrap();
    }
    let added = add_folder(&engine, &projects).await.unwrap();
    assert_eq!(added.len(), 7000);
    assert_eq!(engine.snapshot().await.unwrap().workspaces.len(), 7000);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_folder_without_projects_is_added_as_an_empty_workspace() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let empty = fixture.workspace.join("empty");
    fs::create_dir_all(empty.join("docs")).unwrap();
    let added = add_folder(&engine, &empty).await.unwrap();
    let names: Vec<_> = added
        .iter()
        .map(|workspace| workspace.name.as_str())
        .collect();
    assert_eq!(names, ["empty"]);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn workspaces_are_renamed_reordered_and_removed_without_touching_files() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let second = fixture.workspace.parent().unwrap().join("second");
    let copy = write_skill(&fixture.workspace.join(".agents/skills"), "alpha", "Alpha");
    write_skill(&second.join(".agents/skills"), "beta", "Beta");
    let first = fixture.register(&engine).await.id;
    let other = add_workspace(&engine, &second).await.id;

    let names: Vec<_> = engine
        .snapshot()
        .await
        .unwrap()
        .workspaces
        .into_iter()
        .map(|workspace| workspace.name)
        .collect();
    assert_eq!(names, ["workspace", "second"]);
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
    engine.remove_workspace(first.clone()).await.unwrap();
    assert_eq!(fs::read(copy.join("SKILL.md")).unwrap(), bytes);
    let snapshot = engine.snapshot().await.unwrap();
    assert_eq!(snapshot.workspaces.len(), 1);
    assert_eq!(snapshot.workspaces[0].position, 0);
    let slugs: Vec<_> = fixture
        .scan(&engine)
        .await
        .candidates
        .into_iter()
        .map(|candidate| candidate.slug)
        .collect();
    assert_eq!(slugs, ["beta"]);
    assert_eq!(
        category(engine.remove_workspace(first).await),
        ErrorCategory::NotFound
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_missing_workspace_folder_is_reported_and_skipped() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    fixture.skill("alpha", "Alpha");
    fixture.register(&engine).await;
    let moved = fixture.workspace.with_file_name("moved");
    fs::rename(&fixture.workspace, &moved).unwrap();

    assert!(engine.snapshot().await.unwrap().workspaces[0].missing);
    assert!(fixture.scan(&engine).await.candidates.is_empty());

    fs::rename(&moved, &fixture.workspace).unwrap();
    assert!(!engine.snapshot().await.unwrap().workspaces[0].missing);
    assert_eq!(fixture.scan(&engine).await.candidates.len(), 1);
    engine.shutdown().await.unwrap();
}
