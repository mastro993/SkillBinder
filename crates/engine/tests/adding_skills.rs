//! Adding several library skills to one place at once: the active preset for your agents, or a
//! workspace. Each batch merges into what the place holds, saves once, and runs one pass. The
//! project folder is outside an isolated home.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/project.rs"]
mod project;
#[path = "support/fixture.rs"]
mod support;

use project::{LIBRARY, Project, dirs, row, skill};
use skillbinder_proto::{
    ActivationProgress, AddSkills, CopyState, OrganizationChange, PresetId, Targets, Workspace,
    WorkspaceTargets,
};
use std::{collections::BTreeMap, fs};
use support::write_skill;

fn default() -> PresetId {
    PresetId("default".to_owned())
}

/// Adding `skills` to `workspace`, joining `joining` to its agents.
fn to_workspace(
    workspace: &Workspace,
    joining: &[&str],
    skills: &[(&str, WorkspaceTargets)],
) -> AddSkills {
    AddSkills::Workspace {
        id: workspace.id.clone(),
        dirs: dirs(joining),
        skills: skills
            .iter()
            .map(|(slug, targets)| (skill(slug), targets.clone()))
            .collect(),
    }
}

/// Adding `slugs` for your agents `agents`.
fn for_agents(slugs: &[&str], agents: &[&str]) -> AddSkills {
    AddSkills::YourAgents {
        skills: slugs
            .iter()
            .map(|slug| (skill(slug), dirs(agents)))
            .collect(),
    }
}

#[tokio::test]
async fn skills_added_for_one_agent_are_held_in_the_active_preset_and_written() {
    let project = Project::with(&["pdf", "review"]).await;
    let cursor = project.fixture.home.join(".cursor/skills");
    fs::create_dir_all(&cursor).unwrap();
    project
        .engine
        .add_skills(for_agents(&["pdf", "review"], &["cursor"]))
        .await
        .unwrap();
    let library = project.library().await;
    for held in &library.library.skills {
        assert_eq!(
            held.presets.get(&default()),
            Some(&Targets::Agents(dirs(&["cursor"])))
        );
    }
    assert!(cursor.join("pdf/SKILL.md").exists());
    assert!(cursor.join("review/SKILL.md").exists());
    project.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_held_skill_keeps_its_agents_and_gains_the_added_ones() {
    let project = Project::with(&["pdf", "review"]).await;
    fs::create_dir_all(project.fixture.home.join(".cursor/skills")).unwrap();
    let set = |slug: &str, targets| OrganizationChange::SetTargets {
        id: default(),
        skill: skill(slug),
        targets,
    };
    let claude = Targets::Agents(dirs(&["claude-code"]));
    project
        .engine
        .change_organization(set("pdf", claude))
        .await
        .unwrap();
    project
        .engine
        .change_organization(set("review", Targets::AllAgents))
        .await
        .unwrap();
    project
        .engine
        .add_skills(for_agents(&["pdf", "review"], &["cursor"]))
        .await
        .unwrap();
    let held: BTreeMap<_, _> = project
        .library()
        .await
        .library
        .skills
        .into_iter()
        .map(|skill| (skill.id.to_string(), skill.presets[&default()].clone()))
        .collect();
    assert_eq!(
        held["pdf"],
        Targets::Agents(dirs(&["claude-code", "cursor"]))
    );
    assert_eq!(held["review"], Targets::AllAgents);
    project.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn all_agents_follows_a_directory_added_later_and_ticked_ones_stay_exact() {
    let project = Project::with(&["pdf", "review", "notes"]).await;
    fs::create_dir_all(project.dir(".claude/skills")).unwrap();
    // Cursor reads `.agents/skills` in a workspace.
    fs::create_dir_all(project.fixture.home.join(".cursor/skills")).unwrap();
    let workspace = project.add().await;
    let exact = WorkspaceTargets::Dirs(dirs(&[".claude/skills"]));
    project
        .engine
        .add_skills(to_workspace(
            &workspace,
            &[],
            &[
                ("pdf", WorkspaceTargets::AllAgents),
                ("review", exact.clone()),
            ],
        ))
        .await
        .unwrap();
    assert!(project.dir(".claude/skills/pdf").exists());
    assert!(project.dir(".claude/skills/review").exists());

    let universal = WorkspaceTargets::Dirs(dirs(&[".agents/skills"]));
    project
        .engine
        .add_skills(to_workspace(
            &workspace,
            &[".agents/skills"],
            &[("notes", universal)],
        ))
        .await
        .unwrap();
    let workspace = project.workspace().await;
    assert_eq!(workspace.dirs, dirs(&[".agents/skills", ".claude/skills"]));
    assert_eq!(workspace.skills[&skill("pdf")], WorkspaceTargets::AllAgents);
    assert_eq!(workspace.skills[&skill("review")], exact);
    assert_eq!(
        project.states().await,
        [
            row(".agents/skills", "notes", CopyState::Applied),
            row(".agents/skills", "pdf", CopyState::Applied),
            row(".claude/skills", "pdf", CopyState::Applied),
            row(".claude/skills", "review", CopyState::Applied),
        ]
    );
    project.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_workspace_without_agents_takes_the_ticked_directories() {
    let project = Project::with(&["review"]).await;
    let workspace = project.add().await;
    assert!(workspace.dirs.is_empty());
    assert!(!project.dir(".claude").exists());
    project
        .engine
        .add_skills(to_workspace(
            &workspace,
            &[".claude/skills"],
            &[("review", WorkspaceTargets::AllAgents)],
        ))
        .await
        .unwrap();
    let workspace = project.workspace().await;
    assert_eq!(workspace.dirs, dirs(&[".claude/skills"]));
    assert_eq!(
        workspace.skills,
        [(skill("review"), WorkspaceTargets::AllAgents)].into()
    );
    assert!(project.dir(".claude/skills/review/SKILL.md").exists());
    project.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_differing_copy_waits_while_the_rest_of_the_batch_applies() {
    let project = Project::with(&["pdf", "review"]).await;
    fs::create_dir_all(project.dir(".claude/skills")).unwrap();
    let workspace = project.add().await;
    let theirs = write_skill(&project.dir(".claude/skills"), "pdf", "Their version");
    let before = fs::read_to_string(theirs.join("SKILL.md")).unwrap();
    project
        .engine
        .add_skills(to_workspace(
            &workspace,
            &[],
            &[
                ("pdf", WorkspaceTargets::AllAgents),
                ("review", WorkspaceTargets::AllAgents),
            ],
        ))
        .await
        .unwrap();
    assert_eq!(
        project.states().await,
        [
            row(".claude/skills", "pdf", CopyState::Conflict),
            row(".claude/skills", "review", CopyState::Applied),
        ]
    );
    assert_eq!(fs::read_to_string(theirs.join("SKILL.md")).unwrap(), before);
    project.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_refused_batch_saves_nothing() {
    let project = Project::with(&["pdf"]).await;
    let workspace = project.add().await;
    let library = project.library().await.library;
    let refused =
        |change: AddSkills| async { project.engine.add_skills(change).await.unwrap_err().message };
    let all = || ("pdf", WorkspaceTargets::AllAgents);
    let none = WorkspaceTargets::Dirs(dirs(&[]));
    for (change, message) in [
        (for_agents(&[], &[]), "Select skills to add."),
        (to_workspace(&workspace, &[], &[]), "Select skills to add."),
        (for_agents(&["pdf"], &[]), "Pick an agent for pdf."),
        (
            to_workspace(&workspace, &[".claude/skills"], &[("pdf", none)]),
            "Pick an agent for pdf.",
        ),
        // All agents in a workspace that would still use none holds the skill nowhere.
        (
            to_workspace(&workspace, &[], &[all()]),
            "Pick an agent for pdf.",
        ),
        (
            for_agents(&["gone"], &["claude-code"]),
            "Skill no longer exists.",
        ),
        (
            to_workspace(
                &workspace,
                &[".claude/skills"],
                &[("gone", WorkspaceTargets::AllAgents)],
            ),
            "Skill no longer exists.",
        ),
        // Windsurf is not detected.
        (
            to_workspace(&workspace, &[".windsurf/skills"], &[all()]),
            "Choose the skill directory of a detected agent.",
        ),
    ] {
        assert_eq!(refused(change).await, message);
    }
    assert_eq!(project.library().await.library, library);
    assert!(project.workspace().await.skills.is_empty());
    assert!(!project.dir(".windsurf").exists());

    fs::rename(&project.folder, project.folder.with_extension("moved")).unwrap();
    let change = to_workspace(&workspace, &[".claude/skills"], &[all()]);
    assert_eq!(refused(change).await, "The workspace folder is missing.");
    assert!(project.workspace().await.skills.is_empty());
    project.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_batch_counts_every_pair_it_changes_as_one_operation() {
    let project = Project::with(&["pdf", "review"]).await;
    for dir in [".claude/skills", ".agents/skills"] {
        fs::create_dir_all(project.dir(dir)).unwrap();
    }
    fs::create_dir_all(project.fixture.home.join(".cursor/skills")).unwrap();
    let workspace = project.add().await;
    project
        .engine
        .add_skills(to_workspace(
            &workspace,
            &[],
            &[
                ("pdf", WorkspaceTargets::AllAgents),
                ("review", WorkspaceTargets::AllAgents),
            ],
        ))
        .await
        .unwrap();
    let progress = project.engine.activation_progress().unwrap().unwrap();
    assert_eq!(
        progress,
        ActivationProgress {
            done: 4,
            total: 4,
            step: None
        }
    );
    assert!(
        fs::read_to_string(project.dir(".agents/skills/pdf/SKILL.md"))
            .unwrap()
            .contains(LIBRARY)
    );
    project.engine.shutdown().await.unwrap();
}
