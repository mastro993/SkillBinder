//! Steps and readings the import activation journeys share: a library at onboarding's import
//! review, aged fixtures, and how a library holds and places its skills.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#![allow(dead_code, reason = "Not every test crate uses every helper.")]

use crate::support::{Fixture, write_skill};
use skillbinder_engine::Engine;
use skillbinder_proto::{
    CopyState, ImportPlan, LibrarySnapshot, OnboardingStep, PresetId, SkillId, Targets,
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

/// An engine whose library was just created, so onboarding reviews its first import.
pub async fn created(fixture: &Fixture) -> Engine {
    let engine = fixture.open();
    for step in [
        OnboardingStep::Boundaries,
        OnboardingStep::SyncChoice,
        OnboardingStep::Ready,
    ] {
        engine.set_onboarding(step).await.unwrap();
    }
    let created = engine.create_library().await.unwrap();
    assert_eq!(created.step, OnboardingStep::Import);
    engine
}

/// Writes a skill whose files look edited long ago, so a copy rewritten since reads newer.
pub fn old_skill(directory: &Path, slug: &str, body: &str) -> PathBuf {
    let skill = write_skill(directory, slug, body);
    let old = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
    fs::File::options()
        .write(true)
        .open(skill.join("SKILL.md"))
        .unwrap()
        .set_modified(old)
        .unwrap();
    skill
}

pub fn modified(skill: &Path) -> SystemTime {
    fs::metadata(skill.join("SKILL.md"))
        .unwrap()
        .modified()
        .unwrap()
}

pub fn body(skill: &Path) -> String {
    fs::read_to_string(skill.join("SKILL.md")).unwrap()
}

pub fn named(directory: &Path) -> String {
    fs::canonicalize(directory).unwrap().display().to_string()
}

pub fn skill(slug: &str) -> SkillId {
    SkillId(slug.to_owned())
}

pub fn agents(ids: &[String]) -> Targets {
    Targets::Agents(ids.iter().cloned().collect())
}

/// The detected agents reading the universal directory, which share one copy there.
pub async fn universal(engine: &Engine, home: &Path) -> Vec<String> {
    let directory = named(&home.join(".agents/skills"));
    engine
        .snapshot()
        .await
        .unwrap()
        .agents
        .into_iter()
        .filter(|agent| agent.detected && agent.resolved_skills_dir.as_ref() == Some(&directory))
        .map(|agent| agent.id)
        .collect()
}

pub type Copy = (String, String, CopyState, bool);

/// Each copy's directory, slug, and state, with whether SkillBinder manages it, in path order.
pub fn copies(library: &LibrarySnapshot) -> Vec<Copy> {
    library
        .copies
        .iter()
        .map(|copy| {
            let (directory, slug) = (copy.directory.clone(), copy.skill.to_string());
            (directory, slug, copy.state.clone(), copy.managed)
        })
        .collect()
}

pub fn sorted<const N: usize>(mut expected: [Copy; N]) -> Vec<Copy> {
    expected.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
    expected.into()
}

pub fn applied(directory: &Path, slug: &str) -> Copy {
    (named(directory), slug.to_owned(), CopyState::Applied, true)
}

/// The held agents of each skill in preset `id`.
pub fn held_in(library: &LibrarySnapshot, id: &str) -> Vec<(SkillId, Targets)> {
    let id = PresetId(id.to_owned());
    library
        .library
        .skills
        .iter()
        .filter_map(|skill| Some((skill.id.clone(), skill.presets.get(&id)?.clone())))
        .collect()
}

pub fn replaces(plan: &ImportPlan) -> Vec<(String, Vec<String>)> {
    plan.items
        .iter()
        .map(|item| (item.slug.clone(), item.replaces.clone()))
        .collect()
}
