//! Drift between deployed copies and the library, and the remedies the user picks, in isolated
//! roots.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
mod support;

use skillbinder_engine::Engine;
use skillbinder_proto::{
    Deployment, DeploymentTarget, Drift, ErrorCategory, Remedy, Removal, SkillId,
};
use std::{fs, path::Path, process::Command};
use support::{Fixture, category, write_skill};

struct Setup {
    fixture: Fixture,
    engine: Engine,
    skill: SkillId,
    project: DeploymentTarget,
}

impl Setup {
    /// `alpha` in the library, deployed to Global for Claude Code and to a project for Claude
    /// Code and Codex.
    async fn new() -> Self {
        let fixture = Fixture::new();
        let engine = fixture.ready().await;
        let source = write_skill(&fixture.home.join(".claude/skills"), "alpha", "Alpha");
        let scan = fixture.scan(&engine).await;
        let skill = fixture
            .import_one(&engine, &scan.candidates[0], &scan)
            .await;
        fs::remove_dir_all(source).unwrap();
        fixture.register(&engine).await;
        let id = engine.snapshot().await.unwrap().projects[0].id.clone();
        let project = DeploymentTarget::Project(id);
        for (target, agents) in [
            (DeploymentTarget::Global, vec!["claude-code".into()]),
            (project.clone(), vec!["claude-code".into(), "codex".into()]),
        ] {
            engine
                .set_target_agents(target.clone(), agents)
                .await
                .unwrap();
            engine
                .add_deployments(target, vec![skill.clone()])
                .await
                .unwrap();
        }
        Self {
            fixture,
            engine,
            skill,
            project,
        }
    }

    fn library(&self) -> std::path::PathBuf {
        self.fixture.data.join("library/skills/alpha")
    }

    fn global_copy(&self) -> std::path::PathBuf {
        self.fixture.home.join(".claude/skills/alpha")
    }

    fn project_copy(&self) -> std::path::PathBuf {
        self.fixture.project.join(".claude/skills/alpha")
    }

    /// The project's Codex copy, which drift tests leave alone unless they say otherwise.
    fn codex_copy(&self) -> std::path::PathBuf {
        self.fixture.project.join(".agents/skills/alpha")
    }

    /// The target's deployment as a page shows it; drift never drops a deployment.
    async fn shown(&self, target: &DeploymentTarget) -> Deployment {
        let snapshot = self.engine.snapshot().await.unwrap();
        let deployment = snapshot.deployments.iter().find(|d| &d.target == target);
        deployment.unwrap().clone()
    }

    /// Rescans `target` and returns its drift.
    async fn drift(&self, target: &DeploymentTarget) -> Option<Drift> {
        self.engine.rescan_target(target.clone()).await.unwrap();
        let snapshot = self.engine.snapshot().await.unwrap();
        let digest = snapshot.library.unwrap().library.skills[0].digest.clone();
        self.shown(target).await.drift(&digest)
    }

    /// Picks `remedy` for the deployment as the target's page shows it now.
    async fn resolve(
        &self,
        target: &DeploymentTarget,
        expected: Drift,
        remedy: Remedy,
    ) -> skillbinder_proto::AppResult<()> {
        let shown = self.shown(target).await;
        self.engine.resolve_drift(shown, expected, remedy).await
    }
}

fn edit(copy: &Path, body: &str) {
    write_skill(copy.parent().unwrap(), "alpha", body);
}

fn body(copy: &Path) -> String {
    fs::read_to_string(copy.join("SKILL.md")).unwrap()
}

/// Commits the library when Git is available, so a later change shows as uncommitted.
fn commit_library(library: &Path) -> bool {
    let git = |args: &[&str]| {
        Command::new("git")
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "-C",
            ])
            .arg(library)
            .args(args)
            .status()
            .is_ok_and(|status| status.success())
    };
    library.join(".git").is_dir() && git(&["add", "-A"]) && git(&["commit", "-qm", "fixture"])
}

#[tokio::test]
async fn a_project_change_is_restored_or_accepted_into_the_library() {
    let setup = Setup::new().await;
    let project = setup.project.clone();
    edit(&setup.project_copy(), "Edited in the project");
    assert_eq!(setup.drift(&project).await, Some(Drift::ProjectChanged));

    // A remedy chosen for another state writes nothing.
    assert_eq!(
        category(
            setup
                .resolve(&project, Drift::Outdated, Remedy::UseLibrary)
                .await
        ),
        ErrorCategory::Stale
    );
    assert!(body(&setup.project_copy()).contains("Edited in the project"));

    // Restore from library.
    setup
        .resolve(&project, Drift::ProjectChanged, Remedy::UseLibrary)
        .await
        .unwrap();
    assert_eq!(body(&setup.project_copy()), body(&setup.library()));
    assert_eq!(setup.drift(&project).await, None);

    // Accept into library; the library change stays uncommitted until Sync.
    edit(&setup.project_copy(), "Accepted");
    assert_eq!(setup.drift(&project).await, Some(Drift::ProjectChanged));
    let committed = commit_library(&setup.fixture.data.join("library"));
    setup
        .resolve(&project, Drift::ProjectChanged, Remedy::UseProject)
        .await
        .unwrap();
    assert!(body(&setup.library()).contains("Accepted"));
    assert!(body(&setup.codex_copy()).contains("Accepted"));
    assert_eq!(setup.drift(&project).await, None);
    let snapshot = setup.engine.snapshot().await.unwrap();
    if committed {
        assert!(snapshot.sync.uncommitted);
    }
    let backups = setup.fixture.data.join("backups/drift");
    assert_eq!(fs::read_dir(backups).unwrap().count(), 1);

    // Global is now outdated, and a rescan never updates it on its own.
    let global = DeploymentTarget::Global;
    assert_eq!(setup.drift(&global).await, Some(Drift::Outdated));
    assert!(body(&setup.global_copy()).contains("Alpha"));
    assert_eq!(
        category(
            setup
                .resolve(&global, Drift::Outdated, Remedy::UseProject)
                .await
        ),
        ErrorCategory::Validation
    );

    // Update project.
    setup
        .resolve(&global, Drift::Outdated, Remedy::UseLibrary)
        .await
        .unwrap();
    assert!(body(&setup.global_copy()).contains("Accepted"));
    assert_eq!(setup.drift(&global).await, None);
    setup.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_conflict_keeps_the_side_the_user_picks() {
    let setup = Setup::new().await;
    let (global, project) = (DeploymentTarget::Global, setup.project.clone());
    let change_library = async |body: &str| {
        edit(&setup.global_copy(), body);
        assert_eq!(setup.drift(&global).await, Some(Drift::ProjectChanged));
        setup
            .resolve(&global, Drift::ProjectChanged, Remedy::UseProject)
            .await
            .unwrap();
    };

    // Keep the library side.
    change_library("Library side").await;
    edit(&setup.project_copy(), "Project side");
    assert_eq!(setup.drift(&project).await, Some(Drift::Conflict));
    setup
        .resolve(&project, Drift::Conflict, Remedy::UseLibrary)
        .await
        .unwrap();
    assert!(body(&setup.project_copy()).contains("Library side"));
    assert_eq!(setup.drift(&project).await, None);

    // Keep the project side.
    edit(&setup.project_copy(), "Project side");
    change_library("Library again").await;
    assert_eq!(setup.drift(&project).await, Some(Drift::Conflict));
    setup
        .resolve(&project, Drift::Conflict, Remedy::UseProject)
        .await
        .unwrap();
    assert!(body(&setup.library()).contains("Project side"));
    assert!(body(&setup.project_copy()).contains("Project side"));
    assert_eq!(setup.drift(&project).await, None);
    assert_eq!(setup.drift(&global).await, Some(Drift::Outdated));
    setup.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_missing_copy_is_redeployed_or_its_deployment_removed() {
    let setup = Setup::new().await;
    let project = setup.project.clone();
    fs::remove_dir_all(setup.project_copy()).unwrap();
    assert_eq!(setup.drift(&project).await, Some(Drift::Missing));

    // A missing project folder blocks every remedy.
    let moved = setup.fixture.project.with_file_name("moved");
    fs::rename(&setup.fixture.project, &moved).unwrap();
    assert_eq!(
        category(
            setup
                .resolve(&project, Drift::Missing, Remedy::UseLibrary)
                .await
        ),
        ErrorCategory::Validation
    );
    fs::rename(&moved, &setup.fixture.project).unwrap();

    // Redeploy.
    setup
        .resolve(&project, Drift::Missing, Remedy::UseLibrary)
        .await
        .unwrap();
    assert_eq!(body(&setup.project_copy()), body(&setup.library()));
    assert_eq!(setup.drift(&project).await, None);

    // Remove deployment.
    fs::remove_dir_all(setup.project_copy()).unwrap();
    assert_eq!(setup.drift(&project).await, Some(Drift::Missing));
    let removal = setup
        .engine
        .remove_deployment(project.clone(), setup.skill.clone(), Vec::new())
        .await
        .unwrap();
    assert_eq!(removal, Removal::Removed);
    let snapshot = setup.engine.snapshot().await.unwrap();
    assert!(!snapshot.deployments.iter().any(|d| d.target == project));
    setup.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_drift_that_changed_since_it_was_shown_is_stale() {
    let setup = Setup::new().await;
    let project = setup.project.clone();
    edit(&setup.project_copy(), "Edited in the project");
    assert_eq!(setup.drift(&project).await, Some(Drift::ProjectChanged));
    // The project puts the original back before the user chooses.
    fs::copy(
        setup.library().join("SKILL.md"),
        setup.project_copy().join("SKILL.md"),
    )
    .unwrap();
    assert_eq!(
        category(
            setup
                .resolve(&project, Drift::ProjectChanged, Remedy::UseProject)
                .await
        ),
        ErrorCategory::Stale
    );
    assert!(body(&setup.library()).contains("Alpha"));
    setup.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn copies_changed_in_different_ways_are_not_accepted() {
    let setup = Setup::new().await;
    let project = setup.project.clone();
    edit(&setup.project_copy(), "Claude edit");
    setup.fixture.skill("alpha", "Codex edit");
    assert_eq!(setup.drift(&project).await, Some(Drift::ProjectChanged));
    let error = setup
        .resolve(&project, Drift::ProjectChanged, Remedy::UseProject)
        .await
        .unwrap_err();
    assert_eq!(error.category, ErrorCategory::Validation);
    assert!(body(&setup.library()).contains("Alpha"));
    assert!(body(&setup.codex_copy()).contains("Codex edit"));
    setup.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_copy_changed_after_it_was_shown_is_never_overwritten() {
    let setup = Setup::new().await;
    let project = setup.project.clone();
    edit(&setup.project_copy(), "Shown edit");
    assert_eq!(setup.drift(&project).await, Some(Drift::ProjectChanged));
    let shown = setup.shown(&project).await;
    // Another copy changes while the confirmation is open; the drift is still ProjectChanged.
    setup.fixture.skill("alpha", "Unseen edit");
    let result = setup
        .engine
        .resolve_drift(shown, Drift::ProjectChanged, Remedy::UseLibrary)
        .await;
    assert_eq!(category(result), ErrorCategory::Stale);
    assert!(body(&setup.codex_copy()).contains("Unseen edit"));
    assert!(body(&setup.project_copy()).contains("Shown edit"));
    setup.engine.shutdown().await.unwrap();
}

/// A copy the scan reaches under another agent directory's name is checked, not assumed changed.
#[cfg(unix)]
#[tokio::test]
async fn a_copy_behind_a_linked_agent_directory_is_not_drift() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let source = write_skill(&fixture.home.join(".claude/skills"), "alpha", "Alpha");
    let scan = fixture.scan(&engine).await;
    let skill = fixture
        .import_one(&engine, &scan.candidates[0], &scan)
        .await;
    fs::remove_dir_all(source).unwrap();
    fs::create_dir_all(fixture.project.join(".agents/skills")).unwrap();
    fs::create_dir_all(fixture.project.join(".claude")).unwrap();
    std::os::unix::fs::symlink("../.agents/skills", fixture.project.join(".claude/skills"))
        .unwrap();
    fixture.register(&engine).await;
    let id = engine.snapshot().await.unwrap().projects[0].id.clone();
    let project = DeploymentTarget::Project(id);
    engine
        .set_target_agents(project.clone(), vec!["claude-code".into()])
        .await
        .unwrap();
    engine
        .add_deployments(project.clone(), vec![skill])
        .await
        .unwrap();
    engine.rescan_target(project.clone()).await.unwrap();
    let snapshot = engine.snapshot().await.unwrap();
    let deployment = snapshot.deployments.iter().find(|d| d.target == project);
    assert!(deployment.unwrap().changes.is_empty());
    engine.shutdown().await.unwrap();
}
