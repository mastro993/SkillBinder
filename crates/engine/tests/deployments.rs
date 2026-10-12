//! Adding skills to Global and workspaces, and removing them, in isolated roots.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
mod support;

use skillbinder_engine::Engine;
use skillbinder_proto::{
    AppSnapshot, Deployment, DeploymentTarget, ErrorCategory, Removal, SkillId,
};
use std::{fs, path::Path};
use support::{Fixture, category, write_skill};

/// Imports `alpha` with an executable script and binary bytes, then deletes the source.
async fn library_skill(fixture: &Fixture, engine: &Engine) -> SkillId {
    let source = write_skill(&fixture.home.join(".claude/skills"), "alpha", "Alpha");
    fs::write(source.join("run.sh"), "#!/bin/sh\necho alpha\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(source.join("run.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    }
    fs::create_dir(source.join("assets")).unwrap();
    fs::write(source.join("assets/data.bin"), [0u8, 159, 146, 150, 255]).unwrap();
    let scan = fixture.scan(engine).await;
    let skill = fixture.import_one(engine, &scan.candidates[0], &scan).await;
    fs::remove_dir_all(source).unwrap();
    skill
}

async fn workspace(fixture: &Fixture, engine: &Engine) -> DeploymentTarget {
    fixture.register(engine).await;
    let snapshot = engine.snapshot().await.unwrap();
    DeploymentTarget::Workspace(snapshot.workspaces[0].id.clone())
}

fn deployment<'a>(snapshot: &'a AppSnapshot, target: &DeploymentTarget) -> Option<&'a Deployment> {
    snapshot
        .deployments
        .iter()
        .find(|deployment| &deployment.target == target)
}

/// Every file in `copy` has the library payload's bytes and mode, and nothing is a link.
fn assert_identical(library: &Path, copy: &Path) {
    for relative in ["SKILL.md", "run.sh", "assets/data.bin"] {
        let (expected, actual) = (library.join(relative), copy.join(relative));
        assert_eq!(fs::read(&expected).unwrap(), fs::read(&actual).unwrap());
        let metadata = fs::symlink_metadata(&actual).unwrap();
        assert!(metadata.is_file(), "{relative} is a copy");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o111;
            assert_eq!(mode(&expected), mode(&actual), "{relative}");
        }
    }
    assert!(!fs::symlink_metadata(copy).unwrap().file_type().is_symlink());
}

#[tokio::test]
async fn adding_copies_bytes_into_every_agent_directory_and_records_them() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let skill = library_skill(&fixture, &engine).await;
    let target = workspace(&fixture, &engine).await;
    let skills = vec![skill.clone()];

    // A target without agent skill directories asks for agents first.
    assert!(
        engine.snapshot().await.unwrap().workspaces[0]
            .agents
            .is_empty()
    );
    assert_eq!(
        category(engine.add_deployments(target.clone(), skills.clone()).await),
        ErrorCategory::Validation
    );
    assert_eq!(
        category(
            engine
                .set_target_agents(target.clone(), vec!["unknown".into()])
                .await
        ),
        ErrorCategory::Validation
    );
    engine
        .set_target_agents(target.clone(), vec!["claude-code".into(), "codex".into()])
        .await
        .unwrap();
    engine
        .add_deployments(target.clone(), skills.clone())
        .await
        .unwrap();

    let snapshot = engine.snapshot().await.unwrap();
    let library = snapshot.library.as_ref().unwrap();
    let recorded = deployment(&snapshot, &target).unwrap();
    let copies = [
        fixture.workspace.join(".agents/skills/alpha"),
        fixture.workspace.join(".claude/skills/alpha"),
    ];
    let canonical = fs::canonicalize(&fixture.workspace).unwrap();
    let expected: Vec<_> = [".agents/skills/alpha", ".claude/skills/alpha"]
        .iter()
        .map(|relative| canonical.join(relative).display().to_string())
        .collect();
    assert_eq!(recorded.paths, expected);
    assert_eq!(recorded.digest, library.library.skills[0].digest);
    for copy in &copies {
        assert_identical(&fixture.data.join("library/skills/alpha"), copy);
    }

    // A rescan finds the same copies, and adding the skill again is refused.
    engine.rescan_target(target.clone()).await.unwrap();
    let snapshot = engine.snapshot().await.unwrap();
    assert_eq!(deployment(&snapshot, &target).unwrap().paths, expected);
    assert_eq!(
        category(engine.add_deployments(target, skills).await),
        ErrorCategory::Validation
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn adding_to_global_writes_the_global_skill_directory() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let skill = library_skill(&fixture, &engine).await;
    let target = DeploymentTarget::Global;
    engine
        .set_target_agents(target.clone(), vec!["claude-code".into()])
        .await
        .unwrap();
    engine
        .add_deployments(target.clone(), vec![skill.clone()])
        .await
        .unwrap();
    let copy = fixture.home.join(".claude/skills/alpha");
    assert_identical(&fixture.data.join("library/skills/alpha"), &copy);
    assert_eq!(
        engine
            .remove_deployment(target.clone(), skill, Vec::new())
            .await
            .unwrap(),
        Removal::Removed
    );
    assert!(!copy.exists());
    assert!(fixture.home.join(".claude/skills").is_dir());
    assert!(deployment(&engine.snapshot().await.unwrap(), &target).is_none());
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_different_copy_in_the_way_refuses_the_addition_before_any_write() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let skill = library_skill(&fixture, &engine).await;
    let theirs = fixture.skill("alpha", "Theirs");
    let target = workspace(&fixture, &engine).await;
    engine
        .set_target_agents(target.clone(), vec!["claude-code".into(), "codex".into()])
        .await
        .unwrap();
    let error = engine
        .add_deployments(target.clone(), vec![skill])
        .await
        .unwrap_err();
    assert_eq!(error.category, ErrorCategory::Validation);
    assert!(error.message.contains("already holds a different alpha"));
    assert!(!fixture.workspace.join(".claude/skills/alpha").exists());
    assert!(
        fs::read_to_string(theirs.join("SKILL.md"))
            .unwrap()
            .contains("Theirs")
    );
    assert!(deployment(&engine.snapshot().await.unwrap(), &target).is_none());
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn removing_deletes_only_the_recorded_copies_and_keeps_the_library_skill() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let skill = library_skill(&fixture, &engine).await;
    let target = workspace(&fixture, &engine).await;
    engine
        .set_target_agents(target.clone(), vec!["claude-code".into()])
        .await
        .unwrap();
    engine
        .add_deployments(target.clone(), vec![skill.clone()])
        .await
        .unwrap();
    // Neighbours of the recorded copy, and an identical copy nobody recorded.
    let neighbour = write_skill(&fixture.workspace.join(".claude/skills"), "beta", "Beta");
    let vendored = fixture.workspace.join("vendor/alpha");
    let source = fixture.data.join("library/skills/alpha");
    fs::create_dir_all(&vendored).unwrap();
    fs::copy(source.join("SKILL.md"), vendored.join("SKILL.md")).unwrap();

    let removal = engine
        .remove_deployment(target.clone(), skill.clone(), Vec::new())
        .await
        .unwrap();
    assert_eq!(removal, Removal::Removed);
    assert!(!fixture.workspace.join(".claude/skills/alpha").exists());
    assert!(neighbour.join("SKILL.md").exists());
    assert!(vendored.join("SKILL.md").exists());
    let snapshot = engine.snapshot().await.unwrap();
    assert!(deployment(&snapshot, &target).is_none());
    assert_eq!(snapshot.library.unwrap().library.skills[0].id, skill);
    assert!(source.join("SKILL.md").exists());
    assert_eq!(
        category(engine.remove_deployment(target, skill, Vec::new()).await),
        ErrorCategory::NotFound
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_changed_copy_is_removed_only_after_confirmation() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let skill = library_skill(&fixture, &engine).await;
    let target = workspace(&fixture, &engine).await;
    engine
        .set_target_agents(target.clone(), vec!["claude-code".into(), "codex".into()])
        .await
        .unwrap();
    engine
        .add_deployments(target.clone(), vec![skill.clone()])
        .await
        .unwrap();
    let edited = fixture.workspace.join(".claude/skills/alpha");
    fs::write(edited.join("SKILL.md"), "edited in the workspace").unwrap();

    let removal = engine
        .remove_deployment(target.clone(), skill.clone(), Vec::new())
        .await
        .unwrap();
    let canonical = fs::canonicalize(&edited).unwrap().display().to_string();
    assert_eq!(
        removal,
        Removal::Drifted {
            paths: vec![canonical.clone()]
        }
    );
    assert!(edited.join("SKILL.md").exists());
    assert!(fixture.workspace.join(".agents/skills/alpha").exists());
    assert!(deployment(&engine.snapshot().await.unwrap(), &target).is_some());

    // A rescan keeps the changed copy recorded, so removing it still asks first.
    engine.rescan_target(target.clone()).await.unwrap();
    let snapshot = engine.snapshot().await.unwrap();
    assert_eq!(deployment(&snapshot, &target).unwrap().paths.len(), 2);

    // Confirming one changed copy never discards another that changed after the prompt.
    let other = fixture.workspace.join(".agents/skills/alpha");
    fs::write(other.join("SKILL.md"), "edited by an agent").unwrap();
    let removal = engine
        .remove_deployment(target.clone(), skill.clone(), vec![canonical.clone()])
        .await
        .unwrap();
    let other_path = fs::canonicalize(&other).unwrap().display().to_string();
    assert_eq!(
        removal,
        Removal::Drifted {
            paths: vec![other_path.clone()]
        }
    );
    assert!(edited.exists() && other.exists());

    let removal = engine
        .remove_deployment(target.clone(), skill.clone(), vec![canonical, other_path])
        .await
        .unwrap();
    assert_eq!(removal, Removal::Removed);
    assert!(!edited.exists());
    assert!(!other.exists());
    let snapshot = engine.snapshot().await.unwrap();
    assert!(deployment(&snapshot, &target).is_none());
    assert_eq!(snapshot.library.unwrap().library.skills.len(), 1);
    engine.shutdown().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn agents_whose_directories_are_the_same_folder_share_one_copy() {
    let fixture = Fixture::new();
    let engine = fixture.ready().await;
    let skill = library_skill(&fixture, &engine).await;
    fs::create_dir_all(fixture.workspace.join(".agents/skills")).unwrap();
    fs::create_dir_all(fixture.workspace.join(".claude")).unwrap();
    std::os::unix::fs::symlink(
        "../.agents/skills",
        fixture.workspace.join(".claude/skills"),
    )
    .unwrap();
    let target = workspace(&fixture, &engine).await;
    engine
        .set_target_agents(target.clone(), vec!["claude-code".into(), "codex".into()])
        .await
        .unwrap();
    engine
        .add_deployments(target.clone(), vec![skill])
        .await
        .unwrap();
    assert_identical(
        &fixture.data.join("library/skills/alpha"),
        &fixture.workspace.join(".agents/skills/alpha"),
    );
    let snapshot = engine.snapshot().await.unwrap();
    assert_eq!(deployment(&snapshot, &target).unwrap().paths.len(), 1);
    engine.shutdown().await.unwrap();
}
