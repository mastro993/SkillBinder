//! Nested workspaces under an isolated home, and the steps the workspace removal journeys share.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#![allow(dead_code, reason = "Not every test crate uses every helper.")]

use crate::project::{Project, skill};
use skillbinder_engine::Engine;
use skillbinder_proto::{CopyState, Workspace, WorkspaceId, WorkspaceTargets};
use std::{fs, path::Path};

/// An edit of the `review` fixture, so its copy no longer matches the library.
pub const EDITED: &str = "---\nname: review\ndescription: Test fixture\n---\nMy edit\n";

pub async fn hold(engine: &Engine, workspace: &WorkspaceId, targets: Option<WorkspaceTargets>) {
    engine
        .set_workspace_skill(workspace.clone(), skill("review"), targets)
        .await
        .unwrap();
}

/// Removes `workspace` while `folder` is moved away, then restores it and scans.
pub async fn remove_while_missing(project: &Project, folder: &Path, workspace: WorkspaceId) {
    let moved = folder.with_file_name("moved");
    fs::rename(folder, &moved).unwrap();
    project.engine.remove_workspace(workspace).await.unwrap();
    fs::rename(&moved, folder).unwrap();
    project.fixture.scan(&project.engine).await;
}

/// `~/code` and `~/code/app`, each a workspace using `.claude/skills`.
pub async fn nested(project: &Project) -> (std::path::PathBuf, Workspace, Workspace) {
    let code = fs::canonicalize(&project.fixture.home)
        .unwrap()
        .join("code");
    for folder in [code.clone(), code.join("app")] {
        fs::create_dir_all(folder.join(".claude/skills")).unwrap();
    }
    let mut added = crate::folders::add_folder(&project.engine, &code)
        .await
        .unwrap();
    assert_eq!(added.len(), 2);
    let app = added.remove(1);
    (code, added.remove(0), app)
}

pub fn states(project_copies: &[skillbinder_proto::ManagedCopy]) -> Vec<(String, CopyState)> {
    project_copies
        .iter()
        .map(|copy| (copy.directory.clone(), copy.state.clone()))
        .collect()
}
