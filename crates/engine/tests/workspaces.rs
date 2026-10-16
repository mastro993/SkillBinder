//! Workspaces and the agents' global skill directories: registration and the single scan of
//! their agent skill directories, in isolated roots.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
mod support;

use diesel::{prelude::*, sql_types::Text};
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
async fn detected_agents_are_those_whose_global_directory_exists() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.home.join(".claude/skills")).unwrap();
    let engine = fixture.ready().await;
    let snapshot = engine.snapshot().await.unwrap();
    let detected = |id: &str| {
        snapshot
            .agents
            .iter()
            .find(|agent| agent.id == id)
            .unwrap()
            .detected
    };
    assert!(detected("claude-code"));
    assert!(!detected("cursor"));
    engine.shutdown().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn an_absent_global_directory_under_a_linked_folder_is_still_an_agent_folder() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let home = fs::canonicalize(&fixture.home).unwrap();
    fs::create_dir_all(home.join("agent-data")).unwrap();
    std::os::unix::fs::symlink(home.join("agent-data"), home.join(".claude")).unwrap();
    let error = add_folder(&engine, &home.join("agent-data"))
        .await
        .unwrap_err();
    assert_eq!(
        error.message, "This folder's skills are shown under Agents.",
        "Claude Code would create its skills folder in the linked folder"
    );
    engine.shutdown().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn a_dangling_global_directory_link_points_at_an_agent_folder() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let home = fs::canonicalize(&fixture.home).unwrap();
    fs::create_dir_all(home.join("agent-data")).unwrap();
    fs::create_dir_all(home.join(".claude")).unwrap();
    std::os::unix::fs::symlink(home.join("agent-data/skills"), home.join(".claude/skills"))
        .unwrap();
    let error = add_folder(&engine, &home.join("agent-data"))
        .await
        .unwrap_err();
    assert_eq!(
        error.message, "This folder's skills are shown under Agents.",
        "the link's missing target is where Claude Code's skills will be"
    );
    engine.shutdown().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn a_project_whose_skill_directory_links_to_a_global_one_is_an_agent_folder() {
    let fixture = Fixture::new();
    write_skill(&fixture.home.join(".claude/skills"), "review", "Review");
    let engine = fixture.ready().await;
    let home = fs::canonicalize(&fixture.home).unwrap();
    let project = fs::canonicalize(&fixture.workspace).unwrap();
    fs::create_dir_all(project.join(".claude")).unwrap();
    std::os::unix::fs::symlink(home.join(".claude/skills"), project.join(".claude/skills"))
        .unwrap();
    let error = add_folder(&engine, &project).await.unwrap_err();
    assert_eq!(
        error.message,
        "This folder's skills are shown under Agents."
    );
    engine.shutdown().await.unwrap();
}

/// Opens an engine on `home` with `environment` and completes onboarding.
async fn ready_with(
    fixture: &Fixture,
    home: &std::path::Path,
    environment: &[(&str, &std::path::Path)],
) -> Engine {
    let mut config =
        skillbinder_engine::EngineConfig::isolated(fixture.data.clone(), home.to_owned());
    for (key, value) in environment {
        config
            .environment
            .insert((*key).to_owned(), value.display().to_string());
    }
    let engine = Engine::open(config).unwrap();
    for step in [
        skillbinder_proto::OnboardingStep::Boundaries,
        skillbinder_proto::OnboardingStep::SyncChoice,
        skillbinder_proto::OnboardingStep::Ready,
    ] {
        engine.set_onboarding(step).await.unwrap();
    }
    engine.complete_onboarding().await.unwrap();
    engine
}

#[cfg(unix)]
#[tokio::test]
async fn paths_spelled_through_a_linked_home_are_inside_home() {
    let fixture = Fixture::new();
    let real = fixture.home.with_file_name("real-home");
    fs::create_dir_all(&real).unwrap();
    std::os::unix::fs::symlink(&real, &fixture.home).unwrap();
    let alias = fixture.home.clone();
    write_skill(
        &real.join(".config/opencode/skills"),
        "configured",
        "Configured",
    );
    write_skill(&real.join("dotfiles/claude-skills"), "linked", "Linked");
    fs::create_dir_all(real.join(".claude")).unwrap();
    std::os::unix::fs::symlink(
        alias.join("dotfiles/claude-skills"),
        real.join(".claude/skills"),
    )
    .unwrap();
    let engine = ready_with(
        &fixture,
        &alias,
        &[("XDG_CONFIG_HOME", &alias.join(".config"))],
    )
    .await;

    let snapshot = engine.snapshot().await.unwrap();
    let opencode = snapshot
        .agents
        .iter()
        .find(|agent| agent.id == "opencode")
        .unwrap();
    assert!(opencode.detected);
    let scan = fixture.scan(&engine).await;
    let readers = |slug: &str| {
        let candidate = scan
            .candidates
            .iter()
            .find(|candidate| candidate.slug == slug);
        candidate.map(|candidate| candidate.locations[0].global_agents.clone())
    };
    assert!(
        readers("configured")
            .unwrap()
            .iter()
            .any(|id| id == "opencode")
    );
    assert!(
        readers("linked")
            .unwrap()
            .iter()
            .any(|id| id == "claude-code")
    );
    let real = fs::canonicalize(&real).unwrap();
    let error = add_folder(&engine, &real.join(".config/opencode"))
        .await
        .unwrap_err();
    assert_eq!(
        error.message,
        "This folder's skills are shown under Agents."
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn an_agent_whose_directory_is_outside_home_is_not_detected() {
    let fixture = Fixture::new();
    let outside = fixture.home.with_file_name("outside-claude");
    write_skill(&outside.join("skills"), "elsewhere", "Elsewhere");
    let engine = ready_with(&fixture, &fixture.home, &[("CLAUDE_CONFIG_DIR", &outside)]).await;
    let snapshot = engine.snapshot().await.unwrap();
    let claude = snapshot
        .agents
        .iter()
        .find(|agent| agent.id == "claude-code")
        .unwrap();
    assert!(
        !claude.detected,
        "the scan never reads a directory outside home"
    );
    engine.shutdown().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn a_project_skill_link_to_an_absent_global_directory_is_an_agent_folder() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let home = fs::canonicalize(&fixture.home).unwrap();
    fs::create_dir_all(home.join(".claude")).unwrap();
    let project = fs::canonicalize(&fixture.workspace).unwrap();
    fs::create_dir_all(project.join(".claude")).unwrap();
    std::os::unix::fs::symlink(home.join(".claude/skills"), project.join(".claude/skills"))
        .unwrap();
    let error = add_folder(&engine, &project).await.unwrap_err();
    assert_eq!(
        error.message, "This folder's skills are shown under Agents.",
        "picking and launching must agree before Claude Code creates its skills folder"
    );
    engine.shutdown().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn agents_whose_directories_are_one_folder_share_a_resolved_directory() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.home.join(".agents/skills")).unwrap();
    fs::create_dir_all(fixture.home.join(".claude")).unwrap();
    let home = fs::canonicalize(&fixture.home).unwrap();
    std::os::unix::fs::symlink(home.join(".agents/skills"), home.join(".claude/skills")).unwrap();
    let engine = fixture.ready().await;
    let snapshot = engine.snapshot().await.unwrap();
    let agent = |id: &str| {
        snapshot
            .agents
            .iter()
            .find(|agent| agent.id == id)
            .unwrap()
            .clone()
    };
    let (claude, codex) = (agent("claude-code"), agent("codex"));
    assert_ne!(claude.global_skills_dir, codex.global_skills_dir);
    assert_eq!(claude.resolved_skills_dir, codex.resolved_skills_dir);
    assert_eq!(
        codex.resolved_skills_dir,
        Some(home.join(".agents/skills").display().to_string())
    );
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
async fn the_home_folder_is_never_a_workspace() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.home.join(".claude/skills")).unwrap();
    let engine = fixture.ready().await;
    let error = add_folder(&engine, &fixture.home).await.unwrap_err();
    assert_eq!(error.category, ErrorCategory::Validation);
    assert_eq!(
        error.message,
        "Your home folder's skills are shown under Agents."
    );

    // Picking home still adds the projects in it, and picking a folder that holds home skips it.
    fs::create_dir_all(fixture.home.join("api/.claude/skills")).unwrap();
    fs::create_dir_all(fixture.workspace.join(".claude/skills")).unwrap();
    let names = |added: Vec<skillbinder_proto::Workspace>| -> Vec<String> {
        added.into_iter().map(|workspace| workspace.name).collect()
    };
    assert_eq!(
        names(add_folder(&engine, &fixture.home).await.unwrap()),
        ["api"]
    );
    let again = add_folder(&engine, &fixture.home).await.unwrap_err();
    assert_eq!(
        again.message, "Every project in home is already added.",
        "picking home again does not name its project"
    );
    let parent = fixture.home.parent().unwrap();
    assert_eq!(
        names(add_folder(&engine, parent).await.unwrap()),
        ["workspace"]
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_stored_home_workspace_is_forgotten_on_launch() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.home.join(".claude/skills")).unwrap();
    let engine = fixture.ready().await;
    let kept = fixture.register(&engine).await;
    engine.shutdown().await.unwrap();
    let home = fs::canonicalize(&fixture.home).unwrap();
    let path = fixture.data.join("skillbinder.db");
    let mut database = SqliteConnection::establish(path.to_str().unwrap()).unwrap();
    diesel::sql_query("UPDATE workspaces SET position=1")
        .execute(&mut database)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO workspaces(id,canonical_path,name,position,created_at) VALUES('home',?,'home',0,0)",
    )
    .bind::<Text, _>(home.display().to_string())
    .execute(&mut database)
    .unwrap();
    drop(database);

    let engine = fixture.open();
    let workspaces = engine.snapshot().await.unwrap().workspaces;
    assert_eq!(workspaces.len(), 1);
    assert_eq!((&workspaces[0].id, workspaces[0].position), (&kept.id, 0));
    assert!(
        home.join(".claude/skills").is_dir(),
        "files are not touched"
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_folder_whose_only_project_is_the_home_folder_is_refused() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.home.join(".claude/skills")).unwrap();
    let engine = fixture.ready().await;
    let error = add_folder(&engine, fixture.home.parent().unwrap())
        .await
        .unwrap_err();
    assert_eq!(
        error.message,
        "Your home folder's skills are shown under Agents."
    );
    assert!(engine.snapshot().await.unwrap().workspaces.is_empty());
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_global_skill_directory_itself_is_not_a_workspace() {
    let fixture = Fixture::new();
    let skills = fixture.home.join(".claude/skills");
    write_skill(&skills, "review", "Review");
    let engine = fixture.ready().await;
    let skills = fs::canonicalize(skills).unwrap();
    for picked in [skills.clone(), skills.join("review")] {
        let error = add_folder(&engine, &picked).await.unwrap_err();
        assert_eq!(
            error.message,
            "This folder's skills are shown under Agents.",
            "{}",
            picked.display()
        );
    }
    assert!(engine.snapshot().await.unwrap().workspaces.is_empty());
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn an_empty_home_folder_is_still_an_agent_folder() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let error = add_folder(&engine, &fixture.home).await.unwrap_err();
    assert_eq!(
        error.message,
        "Your home folder's skills are shown under Agents."
    );
    engine.shutdown().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn a_linked_global_directory_is_an_agent_folder_only_inside_home() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let link = |target: &std::path::Path, agent: &str| {
        fs::create_dir_all(target).unwrap();
        fs::create_dir_all(fixture.home.join(agent)).unwrap();
        std::os::unix::fs::symlink(target, fixture.home.join(agent).join("skills")).unwrap();
    };
    let home = fs::canonicalize(&fixture.home).unwrap();
    let shared = home.join("shared");
    link(&shared.join("skills"), ".claude");
    let outside = fs::canonicalize(&fixture.workspace)
        .unwrap()
        .join(".continue/skills");
    link(&outside, ".continue");

    let error = add_folder(&engine, &shared).await.unwrap_err();
    assert_eq!(
        error.message, "This folder's skills are shown under Agents.",
        "the folder a global skill directory resolves to inside home is an agent folder"
    );
    let error = add_folder(&engine, &home.join(".claude"))
        .await
        .unwrap_err();
    assert_eq!(
        error.message, "This folder's skills are shown under Agents.",
        "so is the folder holding the link"
    );
    let added = add_folder(&engine, &fixture.workspace).await.unwrap();
    assert_eq!(
        added.len(),
        1,
        "a link leaving home is not scanned, so its target stays a project"
    );
    engine.shutdown().await.unwrap();
}

async fn add_folder(
    engine: &Engine,
    folder: &std::path::Path,
) -> skillbinder_proto::AppResult<Vec<skillbinder_proto::Workspace>> {
    let grant = engine.grant_directory(folder.to_owned()).await.unwrap();
    engine.add_workspaces(grant.id).await
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
