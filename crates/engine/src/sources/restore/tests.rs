//! Restore and Discard survive a crash or failure at each step of their journal: a crash rolls
//! forward at the next launch, and a failure before the database commits leaves the library as
//! it was.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::persistence::FAULTS;
use crate::schema::managed_copies;
use crate::{git::Git, lifecycle::GitSearch, shutdown::Shutdown, worker::State};
use diesel::prelude::*;
use skillbinder_proto::Reason;
use std::{
    fs,
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
    process::Command,
};

/// A copy of `beta` the library manages in an agent's directory.
const COPY: &str = "/home/.claude/skills";

fn git(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn write(state: &State, slug: &str, body: &str) {
    let directory = state.library_dir().join("skills").join(slug);
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("SKILL.md"),
        format!("---\nname: {slug}\ndescription: test\n---\n{body}\n"),
    )
    .unwrap();
}

fn body(state: &State, slug: &str) -> Option<String> {
    fs::read_to_string(
        state
            .library_dir()
            .join("skills")
            .join(slug)
            .join("SKILL.md"),
    )
    .ok()
}

/// A state on `root` that can run Git, as one opened at launch.
fn open(root: &Path) -> State {
    let mut state = State::isolated_test_state(root.to_path_buf());
    state.git = Some(Git::discover(&GitSearch::System, None, Shutdown::default()).unwrap());
    state
}

/// A library whose `alpha` was committed as "First", then as "Second", which the remote holds,
/// so the second commit is the base; `beta` was added since, unpushed, with a managed copy.
/// Returns the first commit.
fn library(root: &Path) -> (State, String) {
    let state = open(root);
    let repo = state.library_dir();
    state.git.as_ref().unwrap().initialize(&repo).unwrap();
    let commit = |state: &State, message: &str| {
        git(&repo, &["add", "-A", "-f"]);
        git(&repo, &["commit", "-q", "-m", message]);
        state.reconcile_head().unwrap();
        crate::pending_changes::clear(&mut state.database.borrow_mut()).unwrap();
        git(&repo, &["rev-parse", "HEAD"])
    };
    write(&state, "alpha", "First");
    state.reconcile_library().unwrap();
    state.write_pending_records().unwrap();
    let first = commit(&state, "first");
    write(&state, "alpha", "Second");
    // A changed skill's record follows its content.
    let tree = crate::payload::inspect(&repo.join("skills/alpha"))
        .unwrap()
        .tree;
    state
        .write_record("alpha", &crate::records::Record { tree, source: None })
        .unwrap();
    let second = commit(&state, "second");
    git(&repo, &["update-ref", "refs/skillbinder/incoming", &second]);
    write(&state, "beta", "Added");
    state.reconcile_library().unwrap();
    diesel::insert_into(managed_copies::table)
        .values((
            managed_copies::directory.eq(COPY),
            managed_copies::slug.eq("beta"),
            managed_copies::digest.eq("theirs"),
        ))
        .execute(&mut *state.database.borrow_mut())
        .unwrap();
    (state, first)
}

fn inject(points: &[(&'static str, bool)]) {
    FAULTS.with(|faults| *faults.borrow_mut() = points.iter().copied().collect());
}

fn reasons(state: &State) -> Vec<(String, Reason)> {
    crate::pending_changes::reasons(&mut state.database.borrow_mut())
        .unwrap()
        .into_iter()
        .collect()
}

fn holds(state: &State, slug: &str) -> bool {
    crate::library_store::holds(&mut state.database.borrow_mut(), slug).unwrap()
}

fn copies(state: &State) -> i64 {
    managed_copies::table
        .count()
        .get_result(&mut *state.database.borrow_mut())
        .unwrap()
}

/// What the library holds after an operation, applied or not, once reopened and recovered.
type Check = fn(&State, &str, bool);

fn restored(state: &State, first: &str, applied: bool) {
    let expected = if applied { "First" } else { "Second" };
    assert!(body(state, "alpha").unwrap().contains(expected));
    let tree = crate::payload::inspect(&state.library_dir().join("skills/alpha"))
        .unwrap()
        .tree;
    assert_eq!(state.read_record("alpha").unwrap().tree, tree);
    let at = git(
        &state.library_dir(),
        &["log", "-1", "--format=%ct", first, "--"],
    );
    let reason = Reason::Restored {
        at: at.parse().unwrap(),
    };
    let expected: Vec<_> = applied
        .then(|| ("alpha".to_owned(), reason))
        .into_iter()
        .collect();
    assert_eq!(reasons(state), expected);
}

fn discarded(state: &State, _: &str, applied: bool) {
    assert_eq!(body(state, "beta").is_some(), !applied);
    assert_eq!(holds(state, "beta"), !applied);
    // The copy stays where it is either way; only its record goes.
    assert_eq!(copies(state), i64::from(!applied));
}

/// Runs `operation` with a crash, then with a failure, at each step of its journal, and checks
/// the reopened library with `check`.
fn each_step(operation: fn(&mut State, &str) -> skillbinder_proto::AppResult<()>, check: Check) {
    let crashes = [
        "before_rename",
        "after_rename",
        "before_progress",
        "after_progress",
        "before_database",
        "after_database",
    ];
    for (point, crash) in crashes
        .iter()
        .map(|point| (*point, true))
        .chain(crashes[..5].iter().map(|point| (*point, false)))
    {
        let directory = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(directory.path()).unwrap();
        let (mut state, first) = library(&root);
        inject(&[(point, crash)]);
        let result = catch_unwind(AssertUnwindSafe(|| operation(&mut state, &first)));
        assert!(
            result.is_err() || result.as_ref().is_ok_and(|result| result.is_err()),
            "{point}"
        );
        drop(state);
        let mut reopened = open(&root);
        reopened.recover().unwrap();
        check(&reopened, &first, crash);
        assert!(!reopened.recovery_required);
    }
}

#[test]
fn an_interrupted_restore_finishes_or_rolls_back_at_launch() {
    each_step(
        |state, first| state.restore_version("alpha", first),
        restored,
    );
}

#[test]
fn an_interrupted_discard_of_an_added_skill_finishes_or_rolls_back_at_launch() {
    each_step(|state, _| state.discard_change("beta"), discarded);
}

#[test]
fn a_discarded_change_returns_to_the_base_and_forgets_why() {
    let directory = tempfile::tempdir().unwrap();
    let root: PathBuf = dunce::canonicalize(directory.path()).unwrap();
    let (mut state, first) = library(&root);
    state.restore_version("alpha", &first).unwrap();
    assert!(body(&state, "alpha").unwrap().contains("First"));
    state.discard_change("alpha").unwrap();
    assert!(body(&state, "alpha").unwrap().contains("Second"));
    assert!(reasons(&state).is_empty());
    // A name that is not a commit, or a skill no change holds, is refused.
    assert!(state.restore_version("alpha", "HEAD").is_err());
    assert!(state.restore_version("gamma", &first).is_err());
    assert!(state.discard_change("gamma").is_err());
}

#[test]
fn restore_and_discard_wait_for_an_interrupted_change_to_recover() {
    let directory = tempfile::tempdir().unwrap();
    let root: PathBuf = dunce::canonicalize(directory.path()).unwrap();
    let (mut state, first) = library(&root);
    state.recovery_required = true;
    assert!(state.restore_version("alpha", &first).is_err());
    assert!(state.discard_change("beta").is_err());
    assert!(body(&state, "alpha").unwrap().contains("Second"));
    assert!(holds(&state, "beta"));
}
