//! A library without Git (ADR 0011): created as a plain directory once the Git step is skipped,
//! it imports and activates skills, and Git found later, at launch or on a recheck, makes it a
//! repository whose skills stay uncommitted until Sync or Push.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/device.rs"]
mod device;
#[path = "support/fixture.rs"]
mod support;

use device::git;
use skillbinder_engine::Engine;
use skillbinder_proto::{
    OnboardingStep, OrganizationChange, PresetId, RemoteConfig, SkillId, SyncAction, SyncState,
};
use std::{fs, path::Path};
use support::Fixture;

/// An engine without Git, past onboarding, whose library holds the workspace skill `review`,
/// made active in Claude Code's global skill directory.
async fn active_without_git(fixture: &Fixture) -> Engine {
    let claude = fixture.home.join(".claude/skills");
    fs::create_dir_all(&claude).unwrap();
    let engine = fixture.open_without_git();
    engine.skip_git().await.unwrap();
    engine.finish_onboarding(None).await.unwrap();
    device::import(fixture, &engine, &["review"]).await;
    engine
        .change_organization(OrganizationChange::AddToPreset {
            id: PresetId("default".to_owned()),
            skills: vec![SkillId("review".to_owned())],
        })
        .await
        .unwrap();
    engine
}

fn is_repository(library: &Path) -> bool {
    library.join(".git").is_dir()
}

#[tokio::test]
async fn a_library_created_without_git_imports_and_activates_skills() {
    let fixture = Fixture::new();
    let engine = active_without_git(&fixture).await;

    let library = fixture.data.join("library");
    assert!(library.join("skills/review/SKILL.md").is_file());
    assert!(
        fixture
            .home
            .join(".claude/skills/review/SKILL.md")
            .is_file()
    );
    assert!(!is_repository(&library));
    assert!(engine.snapshot().await.unwrap().library.is_some());
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn git_found_at_launch_makes_the_library_a_repository_with_everything_uncommitted() {
    let fixture = Fixture::new();
    active_without_git(&fixture).await.shutdown().await.unwrap();

    let engine = fixture.open();
    let sync = engine.snapshot().await.unwrap().sync;

    let library = fixture.data.join("library");
    assert!(is_repository(&library));
    assert_eq!(git(&library, &["rev-list", "--all"]), "");
    assert!(sync.uncommitted);
    let bare = fixture.workspace.with_file_name("remote.git");
    fs::create_dir(&bare).unwrap();
    git(&bare, &["init", "--bare", "--initial-branch=main"]);
    let remote = RemoteConfig {
        url: url::Url::from_file_path(&bare).unwrap().into(),
        branch: "main".into(),
    };
    engine.connect_remote(remote).await.unwrap();
    let synced = engine.synchronize(SyncAction::Sync).await.unwrap();
    assert_eq!(synced.state, SyncState::Synced);
    assert!(
        git(&bare, &["ls-tree", "-r", "--name-only", "main"])
            .lines()
            .any(|path| path == "skills/review/SKILL.md")
    );
    engine.shutdown().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn git_installed_while_the_app_runs_is_found_on_a_recheck_and_makes_the_library_a_repository()
{
    let fixture = Fixture::new();
    let engine = active_without_git(&fixture).await;
    let system_git = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|directory| directory.join("git"))
        .find(|git| git.is_file())
        .unwrap();
    std::os::unix::fs::symlink(system_git, fixture.git_bin().join("git")).unwrap();

    let boot = engine.bootstrap().await.unwrap();

    assert!(boot.git_version.is_some());
    let library = fixture.data.join("library");
    assert!(is_repository(&library));
    assert_eq!(git(&library, &["rev-list", "--all"]), "");
    engine.shutdown().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn an_outdated_git_is_reported_until_an_update_replaces_it() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    let outdated = fixture.git_bin().join("git");
    let engine = fixture.open_without_git();
    fs::write(&outdated, "#!/bin/sh\necho 'git version 2.30.1'\n").unwrap();
    fs::set_permissions(&outdated, fs::Permissions::from_mode(0o755)).unwrap();

    let boot = engine.bootstrap().await.unwrap();

    assert_eq!(boot.git_version, None);
    assert_eq!(boot.outdated_git.as_deref(), Some("2.30.1"));
    assert_eq!(boot.onboarding.current(), Some(OnboardingStep::Git));

    fs::remove_file(&outdated).unwrap();
    let system_git = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|directory| directory.join("git"))
        .find(|git| git.is_file())
        .unwrap();
    std::os::unix::fs::symlink(system_git, &outdated).unwrap();

    let boot = engine.bootstrap().await.unwrap();

    assert!(boot.git_version.is_some());
    assert_eq!(boot.outdated_git, None);
    engine.shutdown().await.unwrap();
}
