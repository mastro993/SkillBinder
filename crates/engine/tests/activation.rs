//! Activation writes the active preset's skills into your agents' global skill directories as
//! managed copies, and removes only the copies it manages. Every agent directory is under an
//! isolated home.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/fixture.rs"]
mod support;

use diesel::{prelude::*, sql_types::Text};
use skillbinder_engine::Engine;
use skillbinder_proto::{
    CopyChoice, CopyState, ErrorCategory, LibrarySnapshot, OrganizationChange, PresetId,
    PresetSwitch, SkillId, Targets,
};
use std::{
    fs,
    path::{Path, PathBuf},
};
use support::{Fixture, category, scan, write_skill};

/// A library holding `slugs`, with Claude Code's directory and the universal one detected.
struct Agents {
    fixture: Fixture,
    engine: Engine,
    claude: PathBuf,
    universal: PathBuf,
}
impl Agents {
    async fn with(slugs: &[&str]) -> Self {
        let fixture = Fixture::new();
        let claude = fixture.home.join(".claude/skills");
        let universal = fixture.home.join(".agents/skills");
        for directory in [&claude, &universal] {
            fs::create_dir_all(directory).unwrap();
        }
        let engine = fixture.ready().await;
        for slug in slugs {
            fixture.skill(slug, "Library version");
        }
        fixture.register(&engine).await;
        let ids = fixture
            .scan(&engine)
            .await
            .candidates
            .into_iter()
            .map(|candidate| candidate.id)
            .collect();
        let plan = engine.prepare_import(ids, false).await.unwrap();
        engine.apply_import(plan.id).await.unwrap();
        Self {
            fixture,
            engine,
            claude,
            universal,
        }
    }

    async fn organize(&self, change: OrganizationChange) -> LibrarySnapshot {
        self.engine.change_organization(change).await.unwrap()
    }

    /// The directory as copies name it.
    fn named(&self, directory: &Path) -> String {
        fs::canonicalize(directory).unwrap().display().to_string()
    }

    /// Each copy's directory, relative to home, slug, and state.
    fn states(&self, library: &LibrarySnapshot) -> Vec<(String, String, CopyState)> {
        let home = fs::canonicalize(&self.fixture.home).unwrap();
        library
            .copies
            .iter()
            .map(|copy| {
                let relative = Path::new(&copy.directory).strip_prefix(&home).unwrap();
                let relative = relative.to_string_lossy().replace('\\', "/");
                (relative, copy.skill.to_string(), copy.state.clone())
            })
            .collect()
    }

    async fn resolve(&self, directory: &Path, slug: &str, choice: CopyChoice) -> LibrarySnapshot {
        self.engine
            .resolve_copy(self.named(directory), skill(slug), choice)
            .await
            .unwrap()
    }
}

fn skill(slug: &str) -> SkillId {
    SkillId(slug.to_owned())
}

fn default() -> PresetId {
    PresetId("default".to_owned())
}

fn activate(slug: &str) -> OrganizationChange {
    OrganizationChange::AddToPreset {
        id: default(),
        skills: vec![skill(slug)],
    }
}

fn deactivate(slug: &str) -> OrganizationChange {
    OrganizationChange::RemoveFromPreset {
        id: default(),
        skills: vec![skill(slug)],
    }
}

fn targets(slug: &str, targets: Targets) -> OrganizationChange {
    OrganizationChange::SetTargets {
        id: default(),
        skill: skill(slug),
        targets,
    }
}

fn text(directory: &Path, slug: &str) -> String {
    fs::read_to_string(directory.join(slug).join("SKILL.md")).unwrap()
}

fn row(directory: &str, slug: &str, state: CopyState) -> (String, String, CopyState) {
    (directory.to_owned(), slug.to_owned(), state)
}

#[tokio::test]
async fn activation_writes_one_copy_per_directory_and_deactivation_removes_only_it() {
    let agents = Agents::with(&["review"]).await;
    let theirs = write_skill(&agents.claude, "mine", "Not managed");
    let library = agents.organize(activate("review")).await;
    // Many agents read the universal directory, which still holds one copy.
    assert_eq!(
        agents.states(&library),
        [
            row(".agents/skills", "review", CopyState::Applied),
            row(".claude/skills", "review", CopyState::Applied),
        ]
    );
    let expected = text(&agents.fixture.data.join("library/skills"), "review");
    assert_eq!(text(&agents.claude, "review"), expected);
    assert_eq!(text(&agents.universal, "review"), expected);

    let library = agents.organize(deactivate("review")).await;
    assert!(agents.states(&library).is_empty());
    assert!(!agents.claude.join("review").exists());
    assert!(!agents.universal.join("review").exists());
    assert!(theirs.join("SKILL.md").exists(), "other skills stay");
    let names: Vec<_> = fs::read_dir(&agents.claude)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names, ["mine"], "nothing is left behind");
    agents.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn switching_and_deleting_the_active_preset_swap_its_copies() {
    let agents = Agents::with(&["alpha", "beta"]).await;
    agents.organize(activate("alpha")).await;
    let library = agents
        .organize(OrganizationChange::CreatePreset {
            name: "Work".into(),
        })
        .await;
    let work = library.library.presets[1].id.clone();
    agents
        .organize(OrganizationChange::AddToPreset {
            id: work.clone(),
            skills: vec![skill("beta")],
        })
        .await;
    assert!(
        !agents.universal.join("beta").exists(),
        "Work is not active"
    );
    // Claude Code's own beta stays theirs through every switch.
    write_skill(&agents.claude, "beta", "Their version");
    let theirs = text(&agents.claude, "beta");

    let library = agents
        .organize(OrganizationChange::ActivatePreset { id: work.clone() })
        .await;
    assert_eq!(library.library.active_preset, work);
    assert!(!agents.claude.join("alpha").exists());
    assert!(agents.universal.join("beta").exists());
    assert_eq!(text(&agents.claude, "beta"), theirs);

    let preview = agents.engine.preview_delete(work.clone()).await.unwrap();
    assert_eq!(
        preview.switch,
        Some(PresetSwitch {
            adds: vec![skill("alpha")],
            removes: vec![skill("beta")],
        })
    );
    let library = agents
        .organize(OrganizationChange::DeletePreset {
            id: work,
            revision: preview.revision,
        })
        .await;
    assert_eq!(library.library.active_preset, default());
    assert!(agents.claude.join("alpha").exists());
    assert!(!agents.universal.join("beta").exists());
    assert_eq!(text(&agents.claude, "beta"), theirs);
    agents.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_matching_copy_is_adopted_without_writing_and_then_managed() {
    let agents = Agents::with(&["review"]).await;
    let copy = write_skill(&agents.claude, "review", "Library version").join("SKILL.md");
    let before = fs::metadata(&copy).unwrap().modified().unwrap();
    let library = agents.organize(activate("review")).await;
    assert!(
        agents
            .states(&library)
            .contains(&row(".claude/skills", "review", CopyState::Applied))
    );
    assert_eq!(fs::metadata(&copy).unwrap().modified().unwrap(), before);
    agents.organize(deactivate("review")).await;
    assert!(!copy.exists(), "an adopted copy is managed");
    agents.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_differing_copy_is_untouched_until_the_user_replaces_it() {
    let agents = Agents::with(&["review"]).await;
    write_skill(&agents.claude, "review", "Their version");
    let theirs = text(&agents.claude, "review");
    let library = agents.organize(activate("review")).await;
    assert_eq!(
        agents.states(&library),
        [
            row(".agents/skills", "review", CopyState::Applied),
            row(".claude/skills", "review", CopyState::Conflict),
        ]
    );
    assert_eq!(text(&agents.claude, "review"), theirs);

    let library = agents
        .resolve(&agents.claude, "review", CopyChoice::Keep)
        .await;
    assert!(
        agents
            .states(&library)
            .contains(&row(".claude/skills", "review", CopyState::Skipped))
    );
    scan(&agents.engine).await;
    let library = agents.engine.retry_copies().await.unwrap();
    assert!(
        agents
            .states(&library)
            .contains(&row(".claude/skills", "review", CopyState::Skipped)),
        "a kept copy is not asked again"
    );
    assert_eq!(text(&agents.claude, "review"), theirs);

    let library = agents
        .resolve(&agents.claude, "review", CopyChoice::Replace)
        .await;
    assert!(
        agents
            .states(&library)
            .contains(&row(".claude/skills", "review", CopyState::Applied))
    );
    assert_eq!(
        text(&agents.claude, "review"),
        text(&agents.universal, "review")
    );
    agents.organize(deactivate("review")).await;
    assert!(
        !agents.claude.join("review").exists(),
        "the replacement is managed"
    );
    agents.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_stale_conflict_of_an_inactive_skill_never_replaces_its_copy() {
    let agents = Agents::with(&["review"]).await;
    write_skill(&agents.claude, "review", "Their version");
    let theirs = text(&agents.claude, "review");
    // A pass stopped before it forgot the conflict leaves its row behind.
    diesel::sql_query(
        "INSERT INTO managed_copies(directory, slug, digest, issue) \
         VALUES (?, 'review', NULL, '\"Conflict\"')",
    )
    .bind::<Text, _>(agents.named(&agents.claude))
    .execute(&mut agents.fixture.database())
    .unwrap();
    let refused = agents
        .engine
        .resolve_copy(
            agents.named(&agents.claude),
            skill("review"),
            CopyChoice::Replace,
        )
        .await;
    assert_eq!(category(refused), ErrorCategory::Stale);
    assert_eq!(text(&agents.claude, "review"), theirs);
    agents.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn an_edited_copy_is_never_removed_without_asking() {
    let agents = Agents::with(&["review"]).await;
    agents.organize(activate("review")).await;
    let edited = "---\nname: review\ndescription: Test fixture\n---\nMy edit\n";
    fs::write(agents.claude.join("review/SKILL.md"), edited).unwrap();
    let library = agents.organize(deactivate("review")).await;
    assert_eq!(
        agents.states(&library),
        [row(
            ".claude/skills",
            "review",
            CopyState::Edited { active: false }
        )]
    );
    assert_eq!(text(&agents.claude, "review"), edited);
    assert!(!agents.universal.join("review").exists());

    // Keeping it hands the copy to the user, so activating again finds a conflict.
    let library = agents
        .resolve(&agents.claude, "review", CopyChoice::Keep)
        .await;
    assert!(agents.states(&library).is_empty());
    assert_eq!(text(&agents.claude, "review"), edited);
    let library = agents.organize(activate("review")).await;
    assert!(agents.states(&library).contains(&row(
        ".claude/skills",
        "review",
        CopyState::Conflict
    )));
    agents.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn specific_agents_keep_their_list_and_all_agents_include_ones_detected_later() {
    let agents = Agents::with(&["review"]).await;
    let only_claude = Targets::Agents(["claude-code".to_owned()].into());
    agents
        .organize(targets("review", only_claude.clone()))
        .await;
    assert!(agents.claude.join("review").exists());
    assert!(!agents.universal.join("review").exists());
    let library = agents.organize(activate("review")).await;
    assert_eq!(
        library.library.skills[0].presets[&default()],
        only_claude,
        "adding a skill the preset holds keeps its agents"
    );

    agents.organize(targets("review", Targets::AllAgents)).await;
    assert!(agents.universal.join("review").exists());
    let cursor = agents.fixture.home.join(".cursor/skills");
    fs::create_dir_all(&cursor).unwrap();
    scan(&agents.engine).await;
    assert!(cursor.join("review").exists(), "a detected agent gains it");

    agents.organize(targets("review", only_claude)).await;
    assert!(!cursor.join("review").exists());
    assert!(!agents.universal.join("review").exists());
    let library = agents
        .organize(targets("review", Targets::Agents(Default::default())))
        .await;
    assert!(library.library.skills[0].presets.is_empty());
    assert!(!agents.claude.join("review").exists());
    agents.engine.shutdown().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn an_unwritable_directory_keeps_the_choice_and_shows_it_not_applied() {
    use std::os::unix::fs::PermissionsExt;
    let agents = Agents::with(&["review"]).await;
    fs::set_permissions(&agents.claude, fs::Permissions::from_mode(0o555)).unwrap();
    if fs::write(agents.claude.join("probe"), "").is_ok() {
        // Permissions do not bind this user, such as root.
        return;
    }
    let library = agents.organize(activate("review")).await;
    assert!(library.library.skills[0].presets.contains_key(&default()));
    let states = agents.states(&library);
    assert!(states.contains(&row(".agents/skills", "review", CopyState::Applied)));
    assert!(matches!(
        &states[1],
        (directory, _, CopyState::NotApplied(_)) if directory == ".claude/skills"
    ));
    fs::set_permissions(&agents.claude, fs::Permissions::from_mode(0o755)).unwrap();
    let library = agents.engine.retry_copies().await.unwrap();
    assert!(
        agents
            .states(&library)
            .contains(&row(".claude/skills", "review", CopyState::Applied))
    );

    // A copy that cannot be removed stays managed until a retry removes it.
    fs::set_permissions(&agents.claude, fs::Permissions::from_mode(0o555)).unwrap();
    let library = agents.organize(deactivate("review")).await;
    assert!(library.library.skills[0].presets.is_empty());
    assert!(matches!(
        &agents.states(&library)[..],
        [(directory, _, CopyState::NotRemoved(_))] if directory == ".claude/skills"
    ));
    assert!(library.copies[0].managed);
    assert!(agents.claude.join("review").exists());
    fs::set_permissions(&agents.claude, fs::Permissions::from_mode(0o755)).unwrap();
    let library = agents.engine.retry_copies().await.unwrap();
    assert!(library.copies.is_empty());
    assert!(!agents.claude.join("review").exists());
    agents.engine.shutdown().await.unwrap();
}
