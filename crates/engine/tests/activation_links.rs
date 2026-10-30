//! Links among agent directories never make activation touch what it does not manage: an agent
//! directory that leads into SkillBinder's own storage gets no copies, and a directory that became
//! a link to another one is forgotten rather than emptied.
#![cfg(unix)]
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/fixture.rs"]
#[allow(
    dead_code,
    reason = "These tests keep SkillBinder's data under home, unlike Fixture."
)]
mod support;

use skillbinder_engine::{Engine, EngineConfig};
use skillbinder_proto::{
    CopyState, LibrarySnapshot, OnboardingStep, OrganizationChange, PresetId, SkillId,
};
use std::{fs, os::unix::fs::symlink, path::Path};
use support::{scan, write_skill};

/// An engine whose data lives under `home`, as it does on every platform, with `review` imported
/// from the universal directory.
async fn imported(home: &Path, data: &Path) -> Engine {
    write_skill(&home.join(".agents/skills"), "review", "Library version");
    let engine = Engine::open(EngineConfig::isolated(data.to_owned(), home.to_owned())).unwrap();
    for step in [
        OnboardingStep::Boundaries,
        OnboardingStep::SyncChoice,
        OnboardingStep::Ready,
    ] {
        engine.set_onboarding(step).await.unwrap();
    }
    engine.complete_onboarding().await.unwrap();
    let ids = scan(&engine)
        .await
        .candidates
        .into_iter()
        .map(|candidate| candidate.id)
        .collect();
    let plan = engine.prepare_import(ids, false).await.unwrap();
    engine.apply_import(plan.id).await.unwrap();
    engine
}

fn membership(add: bool) -> OrganizationChange {
    let (id, skills) = (PresetId("default".into()), vec![SkillId("review".into())]);
    if add {
        OrganizationChange::AddToPreset { id, skills }
    } else {
        OrganizationChange::RemoveFromPreset { id, skills }
    }
}

fn states(library: &LibrarySnapshot) -> Vec<(String, CopyState)> {
    let named = |directory: &str| Path::new(directory).parent().unwrap().display().to_string();
    library
        .copies
        .iter()
        .map(|copy| (named(&copy.directory), copy.state.clone()))
        .collect()
}

#[tokio::test]
async fn an_agent_directory_inside_skillbinder_storage_never_receives_or_loses_a_copy() {
    let temp = tempfile::tempdir().unwrap();
    let home = fs::canonicalize(temp.path()).unwrap().join("home");
    let data = home.join("data/SkillBinder");
    let engine = imported(&home, &data).await;
    let library_skills = data.join("library/skills");
    fs::create_dir_all(home.join(".claude")).unwrap();
    symlink(&library_skills, home.join(".claude/skills")).unwrap();

    let library = engine.change_organization(membership(true)).await.unwrap();
    let protected = library_skills.parent().unwrap().display().to_string();
    assert!(states(&library).iter().any(|(directory, state)| {
        directory == &protected && matches!(state, CopyState::NotApplied(_))
    }));
    engine.change_organization(membership(false)).await.unwrap();
    assert!(
        library_skills.join("review/SKILL.md").exists(),
        "the library skill stays"
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_directory_that_became_a_link_is_forgotten_without_emptying_its_target() {
    let temp = tempfile::tempdir().unwrap();
    let home = fs::canonicalize(temp.path()).unwrap().join("home");
    let engine = imported(&home, &temp.path().join("data")).await;
    let claude = home.join(".claude/skills");
    fs::create_dir_all(&claude).unwrap();
    engine.change_organization(membership(true)).await.unwrap();
    assert!(claude.join("review").exists());

    // Claude Code now reads the universal directory through a link.
    fs::remove_dir_all(&claude).unwrap();
    symlink(home.join(".agents/skills"), &claude).unwrap();
    scan(&engine).await;
    assert!(home.join(".agents/skills/review/SKILL.md").exists());
    let library = engine.snapshot().await.unwrap().library.unwrap();
    assert_eq!(
        states(&library),
        [(
            home.join(".agents").display().to_string(),
            CopyState::Applied
        )]
    );
    engine.shutdown().await.unwrap();
}
