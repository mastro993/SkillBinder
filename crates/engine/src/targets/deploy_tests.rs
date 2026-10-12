//! Crash and failure recovery of deployment writes, and containment of recorded paths.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::{
    payload,
    persistence::{FAULTS, write_json},
    worker::State,
};
use skillbinder_proto::*;
use std::{
    fs,
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
};

struct Fixture {
    _directory: tempfile::TempDir,
    root: PathBuf,
    skill: SkillId,
}

impl Fixture {
    /// A library holding `alpha`, with Global writing to Claude Code's global directory.
    fn new() -> (Self, State) {
        let directory = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(directory.path()).unwrap().join("data");
        let mut state = State::isolated_test_state(root.clone());
        let payload = state.library_dir().join("skills/alpha");
        fs::create_dir_all(&payload).unwrap();
        fs::write(
            payload.join("SKILL.md"),
            "---\nname: alpha\ndescription: test\n---\nAlpha\n",
        )
        .unwrap();
        let manifest = payload::inspect(&payload).unwrap().manifest;
        let skill = SkillId::new();
        let library = Library {
            schema_version: 2,
            library_id: "library".into(),
            created_at: "today".into(),
            content_policy_version: 1,
            folders: vec![],
            tags: vec![],
            skills: vec![Skill {
                id: skill.clone(),
                slug: "alpha".into(),
                display_name: None,
                folder_id: None,
                tag_ids: vec![],
                upstream_bindings: vec![],
                digest: manifest.digest.clone(),
                file_count: manifest.file_count(),
                total_bytes: manifest.total_bytes(),
            }],
        };
        write_json(&state.library_dir().join(".skillbinder.json"), &library).unwrap();
        state
            .set_target_agents(&DeploymentTarget::Global, &["claude-code".into()])
            .unwrap();
        (
            Self {
                _directory: directory,
                root,
                skill,
            },
            state,
        )
    }

    fn copy(&self) -> PathBuf {
        self.root.join("home/.claude/skills/alpha")
    }

    fn reopen(&self, state: State) -> State {
        drop(state);
        let mut state = State::isolated_test_state(self.root.clone());
        state.recover().unwrap();
        state
    }

    fn add(&self, state: &mut State) -> AppResult<()> {
        state.add_deployments(&DeploymentTarget::Global, std::slice::from_ref(&self.skill))
    }

    fn remove(&self, state: &mut State, discard: &[String]) -> AppResult<Removal> {
        state.remove_deployment(&DeploymentTarget::Global, &self.skill, discard)
    }
}

fn inject(points: &[(&'static str, bool)]) {
    FAULTS.with(|faults| *faults.borrow_mut() = points.iter().copied().collect());
}

/// Nothing but the expected copy, and no journal, remains.
fn assert_settled(state: &State, copy: &Path, deployed: bool) {
    assert_eq!(copy.exists(), deployed);
    let siblings: Vec<_> = fs::read_dir(copy.parent().unwrap())
        .map(|entries| entries.map(|entry| entry.unwrap().file_name()).collect())
        .unwrap_or_default();
    assert_eq!(siblings.len(), usize::from(deployed), "{siblings:?}");
    assert_eq!(state.deployments().unwrap().len(), usize::from(deployed));
    let journals = state.config.data_dir.join("journals/deployments");
    assert_eq!(
        fs::read_dir(journals).map_or(0, |entries| entries.count()),
        0
    );
    assert!(state.require_recovered().is_ok());
}

#[test]
fn a_crash_while_adding_undoes_the_unrecorded_copy() {
    for point in ["deploy_staged", "deploy_renamed", "deploy_before_database"] {
        let (fixture, mut state) = Fixture::new();
        inject(&[(point, true)]);
        assert!(catch_unwind(AssertUnwindSafe(|| fixture.add(&mut state))).is_err());
        let state = fixture.reopen(state);
        assert_settled(&state, &fixture.copy(), false);
    }
}

#[test]
fn a_crash_after_recording_keeps_the_deployment() {
    let (fixture, mut state) = Fixture::new();
    inject(&[("deploy_after_database", true)]);
    assert!(catch_unwind(AssertUnwindSafe(|| fixture.add(&mut state))).is_err());
    let state = fixture.reopen(state);
    assert_settled(&state, &fixture.copy(), true);
}

#[test]
fn a_failed_addition_is_undone_at_once() {
    for point in ["deploy_staged", "deploy_renamed", "deploy_before_database"] {
        let (fixture, mut state) = Fixture::new();
        inject(&[(point, false)]);
        assert!(fixture.add(&mut state).is_err(), "{point}");
        assert_settled(&state, &fixture.copy(), false);
    }
}

#[test]
fn a_crash_while_removing_finishes_the_removal() {
    for point in ["remove_renamed", "remove_deleted", "remove_before_database"] {
        let (fixture, mut state) = Fixture::new();
        fixture.add(&mut state).unwrap();
        inject(&[(point, true)]);
        assert!(catch_unwind(AssertUnwindSafe(|| fixture.remove(&mut state, &[]))).is_err());
        let state = fixture.reopen(state);
        assert_settled(&state, &fixture.copy(), false);
        assert_eq!(state.read_library().unwrap().skills.len(), 1);
    }
}

#[test]
fn recovery_keeps_a_copy_that_changed_before_it_was_confirmed() {
    let (fixture, mut state) = Fixture::new();
    fixture.add(&mut state).unwrap();
    inject(&[("remove_renamed", true)]);
    assert!(catch_unwind(AssertUnwindSafe(|| fixture.remove(&mut state, &[]))).is_err());
    let parent = fixture.copy().parent().unwrap().to_owned();
    let trash = fs::read_dir(&parent)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::write(trash.join("SKILL.md"), "edited in the project").unwrap();
    let state = fixture.reopen(state);
    assert_eq!(
        fs::read_to_string(fixture.copy().join("SKILL.md")).unwrap(),
        "edited in the project"
    );
    assert_settled(&state, &fixture.copy(), true);
}

#[test]
fn removal_refuses_a_recorded_path_outside_the_target() {
    let (fixture, mut state) = Fixture::new();
    let outside = fixture.root.join("home/elsewhere/alpha");
    fs::create_dir_all(&outside).unwrap();
    fs::copy(
        state.library_dir().join("skills/alpha/SKILL.md"),
        outside.join("SKILL.md"),
    )
    .unwrap();
    let digest = state.read_library().unwrap().skills[0].digest.clone();
    let confirmed = [outside.display().to_string()];
    state
        .database
        .execute(
            "INSERT INTO deployments(target,skill_id,digest,paths,recorded_at) VALUES('global',?1,?2,?3,0)",
            rusqlite::params![
                fixture.skill.as_str(),
                digest,
                serde_json::to_string(&[outside.join("../alpha").display().to_string()]).unwrap()
            ],
        )
        .unwrap();
    let error = fixture.remove(&mut state, &confirmed).unwrap_err();
    assert_eq!(error.category, ErrorCategory::Validation);
    state
        .database
        .execute(
            "UPDATE deployments SET paths=?1",
            [serde_json::to_string(&[outside.display().to_string()]).unwrap()],
        )
        .unwrap();
    let error = fixture.remove(&mut state, &confirmed).unwrap_err();
    assert_eq!(error.category, ErrorCategory::Validation);
    assert!(outside.join("SKILL.md").exists());
    assert_eq!(state.deployments().unwrap().len(), 1);
}

#[test]
fn recovery_sets_aside_a_changed_copy_whose_path_was_restored() {
    let (fixture, mut state) = Fixture::new();
    fixture.add(&mut state).unwrap();
    inject(&[("remove_renamed", true)]);
    assert!(catch_unwind(AssertUnwindSafe(|| fixture.remove(&mut state, &[]))).is_err());
    let parent = fixture.copy().parent().unwrap().to_owned();
    let trash = fs::read_dir(&parent)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::write(trash.join("SKILL.md"), "edited in the project").unwrap();
    // The project restored the original copy, for example from Git.
    fs::create_dir(fixture.copy()).unwrap();
    fs::copy(
        state.library_dir().join("skills/alpha/SKILL.md"),
        fixture.copy().join("SKILL.md"),
    )
    .unwrap();
    let state = fixture.reopen(state);
    assert!(state.require_recovered().is_ok());
    assert!(
        !fixture.copy().exists(),
        "the restored clean copy is removed"
    );
    let kept: Vec<_> = fs::read_dir(&parent)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(kept.len(), 1);
    assert!(
        kept[0]
            .to_string_lossy()
            .contains("alpha.skillbinder-kept-")
    );
    assert_eq!(
        fs::read_to_string(kept[0].join("SKILL.md")).unwrap(),
        "edited in the project"
    );
}

#[test]
fn a_second_crash_never_deletes_a_copy_that_replaced_a_deleted_one() {
    let (fixture, mut state) = Fixture::new();
    fixture.add(&mut state).unwrap();
    inject(&[("remove_deleted", true)]);
    assert!(catch_unwind(AssertUnwindSafe(|| fixture.remove(&mut state, &[]))).is_err());
    // An agent writes an edited copy at the same path before SkillBinder restarts.
    fs::create_dir(fixture.copy()).unwrap();
    fs::write(fixture.copy().join("SKILL.md"), "edited in the project").unwrap();
    drop(state);
    let mut state = State::isolated_test_state(fixture.root.clone());
    inject(&[("remove_renamed", true)]);
    assert!(catch_unwind(AssertUnwindSafe(|| state.recover())).is_err());
    let state = fixture.reopen(state);
    assert_eq!(
        fs::read_to_string(fixture.copy().join("SKILL.md")).unwrap(),
        "edited in the project"
    );
    assert_settled(&state, &fixture.copy(), true);
}
