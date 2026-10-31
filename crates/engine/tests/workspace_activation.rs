//! Each workspace holds its own skills, for all its agents or for specific skill directories, and
//! activation writes them there as managed copies. The project folder is outside an isolated home.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/fixture.rs"]
mod support;

use skillbinder_engine::Engine;
use skillbinder_proto::{
    CopyChoice, CopyState, ErrorCategory, LibrarySnapshot, OrganizationChange, PresetId, SkillId,
    Targets, Workspace, WorkspaceTargets,
};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};
use support::{Fixture, write_skill};

/// The body every library skill is imported with, so a copy written with it matches the library.
const LIBRARY: &str = "Library version";

/// A library holding `slugs`, imported from Claude Code's global skill directory, and a project
/// folder that is not a workspace yet.
struct Project {
    fixture: Fixture,
    engine: Engine,
    folder: PathBuf,
}
impl Project {
    async fn with(slugs: &[&str]) -> Self {
        let fixture = Fixture::new();
        let engine = fixture.ready().await;
        for slug in slugs {
            write_skill(&fixture.home.join(".claude/skills"), slug, LIBRARY);
        }
        let ids = fixture
            .scan(&engine)
            .await
            .candidates
            .into_iter()
            .map(|candidate| candidate.id)
            .collect();
        let plan = engine.prepare_import(ids, false).await.unwrap();
        engine.apply_import(plan.id).await.unwrap();
        let folder = fs::canonicalize(&fixture.workspace).unwrap();
        Self {
            fixture,
            engine,
            folder,
        }
    }

    /// One of the project's agent skill directories.
    fn dir(&self, dir: &str) -> PathBuf {
        self.folder.join(dir)
    }

    async fn add(&self) -> Workspace {
        self.fixture.register(&self.engine).await
    }

    async fn workspace(&self) -> Workspace {
        let mut workspaces = self.engine.snapshot().await.unwrap().workspaces;
        assert_eq!(workspaces.len(), 1);
        workspaces.remove(0)
    }

    async fn library(&self) -> LibrarySnapshot {
        self.engine.snapshot().await.unwrap().library.unwrap()
    }

    /// The project's copies: each directory relative to the project, slug, and state.
    async fn states(&self) -> Vec<(String, String, CopyState)> {
        self.library()
            .await
            .copies
            .into_iter()
            .filter_map(|copy| {
                let relative = Path::new(&copy.directory).strip_prefix(&self.folder).ok()?;
                let relative = relative.to_string_lossy().replace('\\', "/");
                Some((relative, copy.skill.to_string(), copy.state))
            })
            .collect()
    }
}

fn skill(slug: &str) -> SkillId {
    SkillId(slug.to_owned())
}

fn dirs(dirs: &[&str]) -> BTreeSet<String> {
    dirs.iter().map(|dir| (*dir).to_owned()).collect()
}

fn row(dir: &str, slug: &str, state: CopyState) -> (String, String, CopyState) {
    (dir.to_owned(), slug.to_owned(), state)
}

fn modified(path: &Path) -> SystemTime {
    fs::metadata(path.join("SKILL.md"))
        .unwrap()
        .modified()
        .unwrap()
}

fn names(directory: &Path) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[tokio::test]
async fn adding_a_workspace_adopts_the_library_skills_it_holds_without_writing() {
    let project = Project::with(&["pdf", "review"]).await;
    let claude = write_skill(&project.dir(".claude/skills"), "review", LIBRARY);
    write_skill(&project.dir(".agents/skills"), "review", LIBRARY);
    write_skill(&project.dir(".agents/skills"), "pdf", LIBRARY);
    write_skill(
        &project.dir(".agents/skills"),
        "notes",
        "Not in the library",
    );
    let before = modified(&claude);

    let added = project.add().await;
    let workspace = project.workspace().await;
    assert_eq!(added, workspace);
    assert_eq!(workspace.dirs, dirs(&[".agents/skills", ".claude/skills"]));
    // Each skill is held for exactly the directories that hold it, so a directory the workspace
    // gains later never receives it unasked.
    let held = |held: &[&str]| WorkspaceTargets::Dirs(dirs(held));
    assert_eq!(
        workspace.skills,
        [
            (skill("pdf"), held(&[".agents/skills"])),
            (skill("review"), held(&[".agents/skills", ".claude/skills"])),
        ]
        .into()
    );
    assert_eq!(
        workspace.paths.values().cloned().collect::<Vec<_>>(),
        [".agents/skills", ".claude/skills"].map(|dir| project.dir(dir).display().to_string())
    );
    assert_eq!(
        project.states().await,
        [
            row(".agents/skills", "pdf", CopyState::Applied),
            row(".agents/skills", "review", CopyState::Applied),
            row(".claude/skills", "review", CopyState::Applied),
        ]
    );
    assert_eq!(
        modified(&claude),
        before,
        "a matching copy is not rewritten"
    );
    assert_eq!(
        names(&project.dir(".agents/skills")),
        ["notes", "pdf", "review"]
    );
    let scan = project.fixture.scan(&project.engine).await;
    let notes = scan
        .candidates
        .iter()
        .find(|candidate| candidate.slug == "notes")
        .unwrap();
    assert_eq!(notes.skill_id, None, "slugs not in the library stay found");
    project.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_differing_copy_found_on_add_is_untouched_until_replaced() {
    let project = Project::with(&["review"]).await;
    let claude = write_skill(&project.dir(".claude/skills"), "review", "Their version");
    let theirs = fs::read_to_string(claude.join("SKILL.md")).unwrap();

    let workspace = project.add().await;
    assert_eq!(
        workspace.skills,
        [(
            skill("review"),
            WorkspaceTargets::Dirs(dirs(&[".claude/skills"]))
        )]
        .into()
    );
    assert_eq!(
        project.states().await,
        [row(".claude/skills", "review", CopyState::Conflict)]
    );
    assert_eq!(fs::read_to_string(claude.join("SKILL.md")).unwrap(), theirs);

    let directory = fs::canonicalize(project.dir(".claude/skills")).unwrap();
    project
        .engine
        .resolve_copy(
            directory.display().to_string(),
            skill("review"),
            CopyChoice::Replace,
        )
        .await
        .unwrap();
    assert_eq!(
        project.states().await,
        [row(".claude/skills", "review", CopyState::Applied)]
    );
    let library = project.fixture.data.join("library/skills/review/SKILL.md");
    assert_eq!(
        fs::read_to_string(claude.join("SKILL.md")).unwrap(),
        fs::read_to_string(library).unwrap()
    );
    project.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn activation_writes_each_directory_and_creates_one_a_detected_agent_adds() {
    let project = Project::with(&["pdf", "review"]).await;
    fs::create_dir_all(project.dir(".claude/skills")).unwrap();
    fs::create_dir_all(project.fixture.home.join(".cursor/skills")).unwrap();
    let workspace = project.add().await;
    assert_eq!(workspace.dirs, dirs(&[".claude/skills"]));
    let set = |slug: &str, targets: Option<WorkspaceTargets>| {
        project
            .engine
            .set_workspace_skill(workspace.id.clone(), skill(slug), targets)
    };

    set("review", Some(WorkspaceTargets::AllAgents))
        .await
        .unwrap();
    assert!(project.dir(".claude/skills/review").exists());
    assert!(!project.dir(".agents/skills").exists());

    // Cursor reads `.agents/skills` in a workspace; choosing it creates the directory and makes it
    // one of the workspace's agents, which a skill held for all of them follows.
    let universal = WorkspaceTargets::Dirs(dirs(&[".agents/skills"]));
    set("pdf", Some(universal.clone())).await.unwrap();
    let workspace = project.workspace().await;
    assert_eq!(workspace.dirs, dirs(&[".agents/skills", ".claude/skills"]));
    assert_eq!(workspace.skills[&skill("pdf")], universal);
    assert_eq!(
        project.states().await,
        [
            row(".agents/skills", "pdf", CopyState::Applied),
            row(".agents/skills", "review", CopyState::Applied),
            row(".claude/skills", "review", CopyState::Applied),
        ]
    );
    assert!(!project.dir(".claude/skills/pdf").exists());

    set("pdf", None).await.unwrap();
    assert!(!project.dir(".agents/skills/pdf").exists());
    assert!(project.dir(".agents/skills/review").exists());
    assert!(
        !project.workspace().await.skills.contains_key(&skill("pdf")),
        "deactivated"
    );

    // Windsurf is not detected, so its directory cannot join the workspace's agents.
    let windsurf = WorkspaceTargets::Dirs(dirs(&[".windsurf/skills"]));
    let refused = set("pdf", Some(windsurf)).await.unwrap_err();
    assert_eq!(refused.category, ErrorCategory::Validation);
    assert!(!project.dir(".windsurf").exists());
    project.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_workspace_without_agents_takes_the_ones_chosen_on_its_first_activation() {
    let project = Project::with(&["review"]).await;
    let workspace = project.add().await;
    assert!(workspace.dirs.is_empty());
    let set = |targets| {
        project
            .engine
            .set_workspace_skill(workspace.id.clone(), skill("review"), Some(targets))
    };
    let refused = set(WorkspaceTargets::AllAgents).await.unwrap_err();
    assert_eq!(refused.category, ErrorCategory::Validation);

    set(WorkspaceTargets::Dirs(dirs(&[".claude/skills"])))
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
async fn removing_a_workspace_keeps_its_copies_and_adding_it_again_adopts_them() {
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
    let copy = project.dir(".claude/skills/review");
    let written = modified(&copy);

    project
        .engine
        .remove_workspace(workspace.id.clone())
        .await
        .unwrap();
    assert!(copy.join("SKILL.md").exists(), "files stay");
    assert!(
        project.states().await.is_empty(),
        "its copies are no longer managed"
    );
    project.fixture.scan(&project.engine).await;
    assert!(copy.join("SKILL.md").exists(), "no pass removes them");

    let again = project.add().await;
    assert_eq!(
        again.skills,
        [(
            skill("review"),
            WorkspaceTargets::Dirs(dirs(&[".claude/skills"]))
        )]
        .into()
    );
    assert_eq!(
        project.states().await,
        [row(".claude/skills", "review", CopyState::Applied)]
    );
    assert_eq!(modified(&copy), written, "adopted without writing");
    project.engine.shutdown().await.unwrap();
}

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

#[tokio::test]
async fn a_skill_an_agent_reads_both_globally_and_in_the_workspace_is_marked() {
    let project = Project::with(&["lint", "review"]).await;
    fs::create_dir_all(project.fixture.home.join(".agents/skills")).unwrap();
    for dir in [".agents/skills", ".claude/skills"] {
        fs::create_dir_all(project.dir(dir)).unwrap();
    }
    let workspace = project.add().await;
    let default = PresetId("default".to_owned());
    let organize = |change| project.engine.change_organization(change);
    organize(OrganizationChange::AddToPreset {
        id: default.clone(),
        skills: vec![skill("review")],
    })
    .await
    .unwrap();
    organize(OrganizationChange::SetTargets {
        id: default,
        skill: skill("lint"),
        targets: Targets::Agents(["claude-code".to_owned()].into()),
    })
    .await
    .unwrap();
    let set = |slug: &str, dir: &str| {
        project.engine.set_workspace_skill(
            workspace.id.clone(),
            skill(slug),
            Some(WorkspaceTargets::Dirs(dirs(&[dir]))),
        )
    };
    // The universal agents read `.agents/skills` here and `~/.agents/skills` globally, which holds
    // review but not lint; Claude Code reads neither `.agents/skills` directory.
    set("review", ".agents/skills").await.unwrap();
    set("lint", ".agents/skills").await.unwrap();
    assert_eq!(project.workspace().await.overlaps, [skill("review")].into());
    set("lint", ".claude/skills").await.unwrap();
    assert_eq!(
        project.workspace().await.overlaps,
        [skill("lint"), skill("review")].into()
    );
    organize(OrganizationChange::RemoveFromPreset {
        id: PresetId("default".to_owned()),
        skills: vec![skill("review")],
    })
    .await
    .unwrap();
    assert_eq!(project.workspace().await.overlaps, [skill("lint")].into());
    project.engine.shutdown().await.unwrap();
}
