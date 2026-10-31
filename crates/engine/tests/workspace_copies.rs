//! Workspace copies stay safe: a missing project folder is never recreated and keeps its records,
//! a directory linked out of the project is never touched, and edited or unwritable copies ask or
//! wait for a retry as your agents' copies do. The project folder is outside an isolated home.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/project.rs"]
mod project;
#[path = "support/fixture.rs"]
mod support;

#[cfg(unix)]
use project::{LIBRARY, dirs, names};
use project::{Project, row, skill};
use skillbinder_proto::{CopyChoice, CopyState, ErrorCategory, WorkspaceTargets};
use std::fs;
#[cfg(unix)]
use support::write_skill;

#[tokio::test]
async fn a_missing_workspace_folder_is_never_recreated() {
    let project = Project::with(&["review"]).await;
    fs::create_dir_all(project.dir(".claude/skills")).unwrap();
    let workspace = project.add().await;
    let activate = || {
        project.engine.set_workspace_skill(
            workspace.id.clone(),
            skill("review"),
            Some(WorkspaceTargets::AllAgents),
        )
    };
    activate().await.unwrap();
    let moved = project.folder.with_file_name("moved");
    fs::rename(&project.folder, &moved).unwrap();

    let refused = activate().await.unwrap_err();
    assert_eq!(refused.category, ErrorCategory::Validation);
    project.fixture.scan(&project.engine).await;
    assert!(!project.folder.exists(), "no copy recreates the folder");
    assert!(project.states().await.is_empty());

    fs::rename(&moved, &project.folder).unwrap();
    assert_eq!(
        project.states().await,
        [row(".claude/skills", "review", CopyState::Applied)],
        "its copies stayed managed"
    );
    project
        .engine
        .set_workspace_skill(workspace.id.clone(), skill("review"), None)
        .await
        .unwrap();
    assert!(!project.dir(".claude/skills/review").exists());
    project.engine.shutdown().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn a_missing_workspace_keeps_the_copies_of_a_directory_linked_inside_it() {
    let project = Project::with(&["review"]).await;
    fs::create_dir_all(project.dir("tooling/agent-skills")).unwrap();
    fs::create_dir_all(project.dir(".claude")).unwrap();
    std::os::unix::fs::symlink("../tooling/agent-skills", project.dir(".claude/skills")).unwrap();
    let workspace = project.add().await;
    assert_eq!(workspace.dirs, dirs(&[".claude/skills"]));
    project
        .engine
        .set_workspace_skill(
            workspace.id.clone(),
            skill("review"),
            Some(WorkspaceTargets::AllAgents),
        )
        .await
        .unwrap();
    let linked = |state| row("tooling/agent-skills", "review", state);
    assert_eq!(project.states().await, [linked(CopyState::Applied)]);
    let edited = "---\nname: review\ndescription: Test fixture\n---\nMy edit\n";
    fs::write(project.dir("tooling/agent-skills/review/SKILL.md"), edited).unwrap();
    project
        .engine
        .set_workspace_skill(workspace.id.clone(), skill("review"), None)
        .await
        .unwrap();
    let asking = linked(CopyState::Edited { active: false });
    assert_eq!(project.states().await, std::slice::from_ref(&asking));
    let directory = project.dir("tooling/agent-skills").display().to_string();

    // While the folder is gone the link cannot be followed, so its directory is known only by the
    // path it resolved to.
    let moved = project.folder.with_file_name("moved");
    fs::rename(&project.folder, &moved).unwrap();
    assert!(
        project.states().await.is_empty(),
        "a missing workspace lists no copies"
    );
    let refused = project
        .engine
        .resolve_copy(directory.clone(), skill("review"), CopyChoice::Keep)
        .await
        .unwrap_err();
    assert_eq!(refused.category, ErrorCategory::Stale);
    project.fixture.scan(&project.engine).await;
    fs::rename(&moved, &project.folder).unwrap();
    assert_eq!(project.states().await, [asking], "its records stayed");
    project
        .engine
        .resolve_copy(directory, skill("review"), CopyChoice::Replace)
        .await
        .unwrap();
    assert!(!project.dir("tooling/agent-skills/review").exists());
    project.engine.shutdown().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn a_directory_linked_out_of_the_project_is_never_touched() {
    let project = Project::with(&["review"]).await;
    fs::create_dir_all(project.dir(".claude/skills")).unwrap();
    let workspace = project.add().await;
    project
        .engine
        .set_workspace_skill(
            workspace.id.clone(),
            skill("review"),
            Some(WorkspaceTargets::AllAgents),
        )
        .await
        .unwrap();
    // The project's directory becomes a link to one outside it, holding a matching copy and what
    // looks like SkillBinder's leftover staging directory.
    let outside = project.folder.with_file_name("outside");
    write_skill(&outside, "review", LIBRARY);
    fs::create_dir_all(outside.join(".skillbinder-abc123")).unwrap();
    fs::remove_dir_all(project.dir(".claude/skills")).unwrap();
    std::os::unix::fs::symlink(&outside, project.dir(".claude/skills")).unwrap();

    project.fixture.scan(&project.engine).await;
    assert!(project.states().await.is_empty(), "its record is forgotten");
    assert_eq!(names(&outside), [".skillbinder-abc123", "review"]);
    project
        .engine
        .set_workspace_skill(workspace.id.clone(), skill("review"), None)
        .await
        .unwrap();
    assert!(outside.join("review/SKILL.md").exists());
    project.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn an_edited_workspace_copy_asks_before_it_is_removed() {
    let project = Project::with(&["review"]).await;
    fs::create_dir_all(project.dir(".claude/skills")).unwrap();
    let workspace = project.add().await;
    let set = |targets| {
        project
            .engine
            .set_workspace_skill(workspace.id.clone(), skill("review"), targets)
    };
    set(Some(WorkspaceTargets::AllAgents)).await.unwrap();
    let edited = "---\nname: review\ndescription: Test fixture\n---\nMy edit\n";
    let copy = project.dir(".claude/skills/review/SKILL.md");
    fs::write(&copy, edited).unwrap();

    set(None).await.unwrap();
    assert_eq!(
        project.states().await,
        [row(
            ".claude/skills",
            "review",
            CopyState::Edited { active: false }
        )]
    );
    assert_eq!(fs::read_to_string(&copy).unwrap(), edited);
    let directory = fs::canonicalize(project.dir(".claude/skills")).unwrap();
    project
        .engine
        .resolve_copy(
            directory.display().to_string(),
            skill("review"),
            CopyChoice::Keep,
        )
        .await
        .unwrap();
    assert!(project.states().await.is_empty());
    assert_eq!(fs::read_to_string(&copy).unwrap(), edited, "kept as it is");
    project.engine.shutdown().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn an_unwritable_workspace_directory_keeps_the_choice_until_a_retry() {
    use std::os::unix::fs::PermissionsExt;
    let project = Project::with(&["review"]).await;
    let claude = project.dir(".claude/skills");
    fs::create_dir_all(&claude).unwrap();
    let workspace = project.add().await;
    fs::set_permissions(&claude, fs::Permissions::from_mode(0o555)).unwrap();
    if fs::write(claude.join("probe"), "").is_ok() {
        // Permissions do not bind this user, such as root.
        return;
    }
    project
        .engine
        .set_workspace_skill(
            workspace.id.clone(),
            skill("review"),
            Some(WorkspaceTargets::AllAgents),
        )
        .await
        .unwrap();
    assert!(
        project
            .workspace()
            .await
            .skills
            .contains_key(&skill("review"))
    );
    assert!(matches!(
        &project.states().await[..],
        [(dir, _, CopyState::NotApplied(_))] if dir == ".claude/skills"
    ));
    fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).unwrap();
    project.engine.retry_copies().await.unwrap();
    assert_eq!(
        project.states().await,
        [row(".claude/skills", "review", CopyState::Applied)]
    );
    project.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn an_answer_for_a_copy_in_a_missing_workspace_waits_for_its_folder() {
    let project = Project::with(&["review"]).await;
    fs::create_dir_all(project.dir(".claude/skills")).unwrap();
    let workspace = project.add().await;
    let set = |targets| {
        project
            .engine
            .set_workspace_skill(workspace.id.clone(), skill("review"), targets)
    };
    set(Some(WorkspaceTargets::AllAgents)).await.unwrap();
    let edited = "---\nname: review\ndescription: Test fixture\n---\nMy edit\n";
    fs::write(project.dir(".claude/skills/review/SKILL.md"), edited).unwrap();
    set(None).await.unwrap();
    let asking = row(
        ".claude/skills",
        "review",
        CopyState::Edited { active: false },
    );
    assert_eq!(project.states().await, std::slice::from_ref(&asking));

    let directory = project.dir(".claude/skills").display().to_string();
    let moved = project.folder.with_file_name("moved");
    fs::rename(&project.folder, &moved).unwrap();
    let refused = project
        .engine
        .resolve_copy(directory, skill("review"), CopyChoice::Keep)
        .await
        .unwrap_err();
    assert_eq!(refused.category, ErrorCategory::Stale);
    fs::rename(&moved, &project.folder).unwrap();
    assert_eq!(project.states().await, [asking], "it still asks");
    project.engine.shutdown().await.unwrap();
}
