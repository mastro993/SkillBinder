//! Undo on a workspace's page: removing a skill from a workspace and then saving its previous
//! choice again writes the same copies back and stores the same choice, for all its agents or a
//! specific list. The project folder is outside an isolated home.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/project.rs"]
mod project;
#[path = "support/fixture.rs"]
mod support;

use project::{LIBRARY, Project, dirs, skill};
use skillbinder_proto::WorkspaceTargets;
use std::fs;

/// Holds `review` for `targets`, removes it, then restores `targets`, checking the copies and the
/// stored choice come back as they were.
async fn removed_then_restored(targets: WorkspaceTargets) {
    let project = Project::with(&["review"]).await;
    fs::create_dir_all(project.dir(".claude/skills")).unwrap();
    let workspace = project.add().await;
    let set = |targets| {
        project
            .engine
            .set_workspace_skill(workspace.id.clone(), skill("review"), targets)
    };
    set(Some(targets.clone())).await.unwrap();
    let copies = project.states().await;
    assert!(!copies.is_empty(), "the choice wrote copies");
    let copy = project.dir(".claude/skills/review/SKILL.md");
    assert!(fs::read_to_string(&copy).unwrap().contains(LIBRARY));

    set(None).await.unwrap();
    assert!(project.states().await.is_empty());
    assert!(!copy.exists(), "the removal deleted the copy");
    assert!(project.workspace().await.skills.is_empty());

    set(Some(targets.clone())).await.unwrap();
    assert_eq!(project.states().await, copies, "the same copies are back");
    assert!(fs::read_to_string(&copy).unwrap().contains(LIBRARY));
    assert_eq!(
        project.workspace().await.skills.get(&skill("review")),
        Some(&targets),
        "the same choice is stored"
    );
    project.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn undo_restores_a_skill_held_for_all_agents() {
    removed_then_restored(WorkspaceTargets::AllAgents).await;
}

#[tokio::test]
async fn undo_restores_a_skill_held_for_specific_directories() {
    removed_then_restored(WorkspaceTargets::Dirs(dirs(&[".claude/skills"]))).await;
}
