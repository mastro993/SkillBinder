//! Each copy record names who it is held for, so removing a workspace forgets exactly its
//! records and hands any another workspace or your agents still hold to them, however the
//! directories link and whichever workspace held a shared copy first. Every folder is inside an
//! isolated home, where a managed copy nothing holds any more would be removed.
#![cfg(unix)]
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/folders.rs"]
mod folders;
#[path = "support/nested.rs"]
mod nested;
#[path = "support/project.rs"]
mod project;
#[path = "support/fixture.rs"]
mod support;

use nested::{hold, nested, remove_while_missing, states};
use project::{Project, skill};
use skillbinder_proto::{CopyChoice, CopyState, OrganizationChange, PresetId, WorkspaceTargets};
use std::{fs, os::unix::fs::symlink};
use support::write_skill;

#[tokio::test]
async fn removing_a_workspace_whose_directory_became_your_agents_leaves_its_copies() {
    for at_launch in [false, true] {
        let mut project = Project::with(&["review"]).await;
        let home = fs::canonicalize(&project.fixture.home).unwrap();
        let dotfiles = home.join("dotfiles");
        fs::create_dir_all(dotfiles.join(".claude/skills")).unwrap();
        let workspace = support::add_workspace(&project.engine, &dotfiles).await;
        hold(
            &project.engine,
            &workspace.id,
            Some(WorkspaceTargets::AllAgents),
        )
        .await;
        let copy = dotfiles.join(".claude/skills/review/SKILL.md");
        assert!(copy.exists());
        // Like a dotfiles manager, `~/.claude` now links into the workspace.
        fs::remove_dir_all(home.join(".claude")).unwrap();
        symlink("dotfiles/.claude", home.join(".claude")).unwrap();

        if at_launch {
            // The engine forgets a workspace that became an agent folder when it opens.
            project.engine.shutdown().await.unwrap();
            project.engine = project.fixture.open();
            assert!(
                project
                    .engine
                    .snapshot()
                    .await
                    .unwrap()
                    .workspaces
                    .is_empty()
            );
        } else {
            project.engine.remove_workspace(workspace.id).await.unwrap();
        }
        project.fixture.scan(&project.engine).await;
        assert!(copy.exists(), "at launch: {at_launch}");
        project.engine.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn removing_a_workspace_forgets_its_copies_in_a_directory_shared_with_a_missing_one() {
    let project = Project::with(&["review"]).await;
    let (code, outer, _) = nested(&project).await;
    // The outer workspace's directory leads into the inner one's, which never holds the skill.
    fs::remove_dir_all(code.join(".claude/skills")).unwrap();
    symlink("../app/.claude/skills", code.join(".claude/skills")).unwrap();
    hold(
        &project.engine,
        &outer.id,
        Some(WorkspaceTargets::AllAgents),
    )
    .await;
    let copy = code.join("app/.claude/skills/review/SKILL.md");
    assert!(copy.exists());

    remove_while_missing(&project, &code.join("app"), outer.id).await;
    assert!(copy.exists(), "the copy stays, unmanaged");
    project.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn removing_a_workspace_under_home_leaves_its_copies() {
    let project = Project::with(&["review"]).await;
    let folder = fs::canonicalize(&project.fixture.home)
        .unwrap()
        .join("proj");
    fs::create_dir_all(folder.join(".claude/skills")).unwrap();
    let workspace = support::add_workspace(&project.engine, &folder).await;
    hold(
        &project.engine,
        &workspace.id,
        Some(WorkspaceTargets::AllAgents),
    )
    .await;
    let copy = folder.join(".claude/skills/review/SKILL.md");
    assert!(copy.exists());

    project.engine.remove_workspace(workspace.id).await.unwrap();
    project.fixture.scan(&project.engine).await;
    // A record left behind would make this scan remove a copy nothing holds any more.
    assert!(
        copy.exists(),
        "removing the workspace forgets its copies and leaves them"
    );
    project.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn removing_a_workspace_hands_a_shared_copy_to_the_other_holder() {
    let project = Project::with(&["review"]).await;
    let (code, outer, app) = nested(&project).await;
    fs::remove_dir_all(code.join(".claude/skills")).unwrap();
    symlink("../app/.claude/skills", code.join(".claude/skills")).unwrap();
    let directory = code.join("app/.claude/skills");
    write_skill(&directory, "review", "Their version");
    for workspace in [&outer, &app] {
        hold(
            &project.engine,
            &workspace.id,
            Some(WorkspaceTargets::AllAgents),
        )
        .await;
    }
    let shown = directory.display().to_string();
    project
        .engine
        .resolve_copy(shown.clone(), skill("review"), CopyChoice::Keep)
        .await
        .unwrap();
    let kept = vec![(shown, CopyState::Skipped)];
    let library = project.engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(states(&library.copies), kept);

    // Forgotten instead, the kept copy would ask again as a conflict.
    project.engine.remove_workspace(outer.id).await.unwrap();
    project.fixture.scan(&project.engine).await;
    let library = project.engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(
        states(&library.copies),
        kept,
        "the inner workspace takes it over"
    );
    project.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn removing_the_last_holder_of_a_shared_copy_leaves_it() {
    let project = Project::with(&["review"]).await;
    let (code, outer, app) = nested(&project).await;
    fs::remove_dir_all(code.join(".claude/skills")).unwrap();
    symlink("../app/.claude/skills", code.join(".claude/skills")).unwrap();
    // The outer workspace writes the copy, then lets go of it while the inner one holds it.
    hold(
        &project.engine,
        &outer.id,
        Some(WorkspaceTargets::AllAgents),
    )
    .await;
    hold(&project.engine, &app.id, Some(WorkspaceTargets::AllAgents)).await;
    hold(&project.engine, &outer.id, None).await;
    let copy = code.join("app/.claude/skills/review/SKILL.md");
    assert!(copy.exists());

    project.engine.remove_workspace(app.id).await.unwrap();
    project.fixture.scan(&project.engine).await;
    assert!(copy.exists(), "the copy stays, unmanaged");
    project.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn removing_a_workspace_your_agents_no_longer_share_leaves_its_copies() {
    let project = Project::with(&["review"]).await;
    let home = fs::canonicalize(&project.fixture.home).unwrap();
    let dotfiles = home.join("dotfiles");
    fs::create_dir_all(dotfiles.join(".claude/skills")).unwrap();
    let workspace = support::add_workspace(&project.engine, &dotfiles).await;
    hold(
        &project.engine,
        &workspace.id,
        Some(WorkspaceTargets::AllAgents),
    )
    .await;
    // For a while `~/.claude` links into the workspace, and Default holds the skill there too.
    let claude = home.join(".claude");
    fs::rename(&claude, home.join("claude-before")).unwrap();
    symlink("dotfiles/.claude", &claude).unwrap();
    project
        .engine
        .change_organization(OrganizationChange::AddToPreset {
            id: PresetId("default".to_owned()),
            skills: vec![skill("review")],
        })
        .await
        .unwrap();
    fs::remove_file(&claude).unwrap();
    fs::rename(home.join("claude-before"), &claude).unwrap();
    let copy = dotfiles.join(".claude/skills/review/SKILL.md");
    assert!(copy.exists());

    project.engine.remove_workspace(workspace.id).await.unwrap();
    project.fixture.scan(&project.engine).await;
    assert!(copy.exists(), "the copy stays, unmanaged");
    project.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn removing_a_workspace_hands_a_shared_copy_to_a_missing_holder() {
    let project = Project::with(&["review"]).await;
    let (code, outer, app) = nested(&project).await;
    fs::remove_dir_all(code.join(".claude/skills")).unwrap();
    symlink("../app/.claude/skills", code.join(".claude/skills")).unwrap();
    let directory = code.join("app/.claude/skills");
    write_skill(&directory, "review", "Their version");
    for workspace in [&outer, &app] {
        hold(
            &project.engine,
            &workspace.id,
            Some(WorkspaceTargets::AllAgents),
        )
        .await;
    }
    let shown = directory.display().to_string();
    project
        .engine
        .resolve_copy(shown.clone(), skill("review"), CopyChoice::Keep)
        .await
        .unwrap();

    // The inner folder is missing while the outer workspace is removed.
    remove_while_missing(&project, &code.join("app"), outer.id).await;
    let library = project.engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(
        states(&library.copies),
        [(shown, CopyState::Skipped)],
        "the inner workspace takes over the kept copy"
    );
    project.engine.shutdown().await.unwrap();
}
