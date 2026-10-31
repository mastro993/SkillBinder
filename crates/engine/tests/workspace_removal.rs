//! Removing a workspace while its folder is missing forgets only the copy records it alone holds,
//! so no later scan deletes a copy once the folder returns: not the ones of a nested workspace
//! that is missing too, and not a global skill directory's. Every folder is inside an isolated
//! home, where a managed copy nothing holds any more would be removed.
#![cfg(unix)]
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/folders.rs"]
mod folders;
#[path = "support/project.rs"]
mod project;
#[path = "support/fixture.rs"]
mod support;

use project::{LIBRARY, Project, skill};
use skillbinder_engine::Engine;
use skillbinder_proto::{
    CopyState, OrganizationChange, PresetId, Workspace, WorkspaceId, WorkspaceTargets,
};
use std::{fs, os::unix::fs::symlink, path::Path};
use support::write_skill;

const EDITED: &str = "---\nname: review\ndescription: Test fixture\n---\nMy edit\n";

async fn hold(engine: &Engine, workspace: &WorkspaceId, targets: Option<WorkspaceTargets>) {
    engine
        .set_workspace_skill(workspace.clone(), skill("review"), targets)
        .await
        .unwrap();
}

/// Removes `workspace` while `folder` is moved away, then restores it and scans.
async fn remove_while_missing(project: &Project, folder: &Path, workspace: WorkspaceId) {
    let moved = folder.with_file_name("moved");
    fs::rename(folder, &moved).unwrap();
    project.engine.remove_workspace(workspace).await.unwrap();
    fs::rename(&moved, folder).unwrap();
    project.fixture.scan(&project.engine).await;
}

/// `~/code` and `~/code/app`, each a workspace using `.claude/skills`.
async fn nested(project: &Project) -> (std::path::PathBuf, Workspace, Workspace) {
    let code = fs::canonicalize(&project.fixture.home)
        .unwrap()
        .join("code");
    for folder in [code.clone(), code.join("app")] {
        fs::create_dir_all(folder.join(".claude/skills")).unwrap();
    }
    let mut added = folders::add_folder(&project.engine, &code).await.unwrap();
    assert_eq!(added.len(), 2);
    let app = added.remove(1);
    (code, added.remove(0), app)
}

fn states(project_copies: &[skillbinder_proto::ManagedCopy]) -> Vec<(String, CopyState)> {
    project_copies
        .iter()
        .map(|copy| (copy.directory.clone(), copy.state.clone()))
        .collect()
}

#[tokio::test]
async fn removing_a_missing_workspace_leaves_the_copies_of_a_directory_linked_inside_it() {
    let project = Project::with(&["review"]).await;
    // Inside home, a managed copy that nothing holds any more would be removed.
    let folder = fs::canonicalize(&project.fixture.home)
        .unwrap()
        .join("proj");
    fs::create_dir_all(folder.join("tooling/agent-skills")).unwrap();
    fs::create_dir_all(folder.join(".claude")).unwrap();
    std::os::unix::fs::symlink("../tooling/agent-skills", folder.join(".claude/skills")).unwrap();
    let workspace = support::add_workspace(&project.engine, &folder).await;
    project
        .engine
        .set_workspace_skill(
            workspace.id.clone(),
            skill("review"),
            Some(WorkspaceTargets::AllAgents),
        )
        .await
        .unwrap();
    let copy = folder.join("tooling/agent-skills/review/SKILL.md");
    assert!(copy.exists());

    let moved = folder.with_file_name("moved");
    fs::rename(&folder, &moved).unwrap();
    project.engine.remove_workspace(workspace.id).await.unwrap();
    fs::rename(&moved, &folder).unwrap();
    project.fixture.scan(&project.engine).await;
    // A record left behind would make this scan remove a copy nothing holds any more.
    assert!(
        copy.exists(),
        "removing the workspace forgets its copies and leaves them"
    );
    project.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn removing_a_workspace_inside_a_missing_one_leaves_its_copies() {
    let project = Project::with(&["review"]).await;
    let (code, _, app) = nested(&project).await;
    hold(&project.engine, &app.id, Some(WorkspaceTargets::AllAgents)).await;
    let copy = code.join("app/.claude/skills/review/SKILL.md");
    assert!(copy.exists());

    // Both folders are missing, and the outer workspace stays.
    remove_while_missing(&project, &code, app.id).await;
    assert!(copy.exists());
    project.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn removing_a_missing_workspace_keeps_a_nested_missing_workspace_records() {
    let project = Project::with(&["review"]).await;
    let (code, outer, app) = nested(&project).await;
    hold(&project.engine, &app.id, Some(WorkspaceTargets::AllAgents)).await;
    fs::write(code.join("app/.claude/skills/review/SKILL.md"), EDITED).unwrap();
    hold(&project.engine, &app.id, None).await;

    remove_while_missing(&project, &code, outer.id).await;
    let library = project.engine.snapshot().await.unwrap().library.unwrap();
    let directory = code.join("app/.claude/skills").display().to_string();
    assert_eq!(
        states(&library.copies),
        [(directory, CopyState::Edited { active: false })],
        "the inner workspace still asks about its edited copy"
    );
    project.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn removing_a_missing_workspace_keeps_a_global_skill_directory_linked_into_it() {
    let project = Project::with(&[]).await;
    let home = fs::canonicalize(&project.fixture.home).unwrap();
    let dotfiles = home.join("dotfiles");
    let skills = dotfiles.join("claude-skills");
    write_skill(&skills, "review", LIBRARY);
    fs::create_dir_all(home.join(".claude")).unwrap();
    symlink("../dotfiles/claude-skills", home.join(".claude/skills")).unwrap();
    project::import_found(&project.fixture, &project.engine).await;
    let organize = |change| project.engine.change_organization(change);
    let default = PresetId("default".to_owned());
    organize(OrganizationChange::AddToPreset {
        id: default.clone(),
        skills: vec![skill("review")],
    })
    .await
    .unwrap();
    fs::write(skills.join("review/SKILL.md"), EDITED).unwrap();
    let library = organize(OrganizationChange::RemoveFromPreset {
        id: default,
        skills: vec![skill("review")],
    })
    .await
    .unwrap();
    let asking = states(&library.copies);
    assert_eq!(
        asking,
        [(
            skills.display().to_string(),
            CopyState::Edited { active: false }
        )]
    );
    let workspace = support::add_workspace(&project.engine, &dotfiles).await;

    remove_while_missing(&project, &dotfiles, workspace.id).await;
    let library = project.engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(
        states(&library.copies),
        asking,
        "your agents still ask about it"
    );
    project.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn removing_a_workspace_keeps_the_records_of_a_missing_one_its_directory_links_into() {
    let project = Project::with(&["review"]).await;
    let (code, outer, app) = nested(&project).await;
    hold(&project.engine, &app.id, Some(WorkspaceTargets::AllAgents)).await;
    fs::write(code.join("app/.claude/skills/review/SKILL.md"), EDITED).unwrap();
    hold(&project.engine, &app.id, None).await;
    // The outer workspace's directory now leads into the inner one's.
    fs::remove_dir_all(code.join(".claude/skills")).unwrap();
    symlink("../app/.claude/skills", code.join(".claude/skills")).unwrap();

    // Only the inner folder is missing.
    remove_while_missing(&project, &code.join("app"), outer.id).await;
    let library = project.engine.snapshot().await.unwrap().library.unwrap();
    let directory = code.join("app/.claude/skills").display().to_string();
    assert_eq!(
        states(&library.copies),
        [(directory, CopyState::Edited { active: false })],
        "the inner workspace still asks about its edited copy"
    );
    project.engine.shutdown().await.unwrap();
}
