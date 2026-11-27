//! A merge interrupted at each step of its journal finishes at the next launch, and one that
//! fails after its journal row finishes at once, so HEAD never names the merge while the files
//! are still this device's.
#![allow(clippy::unwrap_used)]

use super::MERGE;
use crate::persistence::FAULTS;
use crate::schema::journals;
use crate::{git::Git, lifecycle::GitSearch, shutdown::Shutdown, worker::State};
use diesel::prelude::*;
use skillbinder_proto::{RemoteConfig, SyncAction};
use std::{
    collections::BTreeMap,
    fs,
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
    process::Command,
};

fn git(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.name=Peer",
            "-c",
            "user.email=peer@example.invalid",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn write(skills: &Path, slug: &str, body: &str) {
    let directory = skills.join(slug);
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("SKILL.md"),
        format!("---\nname: {slug}\ndescription: test\n---\n{body}\n"),
    )
    .unwrap();
}

fn body(state: &State, slug: &str) -> String {
    let path = state
        .library_dir()
        .join("skills")
        .join(slug)
        .join("SKILL.md");
    fs::read_to_string(path).unwrap()
}

/// A state on `root`'s device that can run Git, as one opened at launch.
fn open(root: &Path) -> State {
    let mut state = State::isolated_test_state(root.join("device"));
    state.git = Some(Git::discover(&GitSearch::System, None, Shutdown::default()).unwrap());
    state
}

/// A device that pushed `alpha` and `beta`, then changed `beta`, while a peer pushed a change
/// to `alpha`.
fn diverged(root: &Path) -> State {
    let remote = root.join("remote.git");
    fs::create_dir_all(&remote).unwrap();
    git(&remote, &["init", "--bare", "--initial-branch=main"]);
    let mut state = open(root);
    let repo = state.library_dir();
    state.git.as_ref().unwrap().initialize(&repo).unwrap();
    for slug in ["alpha", "beta"] {
        write(&repo.join("skills"), slug, "Base");
    }
    let url = url::Url::from_file_path(&remote).unwrap().to_string();
    let config = RemoteConfig {
        url: url.clone(),
        branch: "main".into(),
    };
    state.set_setting("remote", &Some(config)).unwrap();
    state
        .synchronize(SyncAction::Push, &BTreeMap::new())
        .unwrap();
    git(root, &["clone", &url, "peer"]);
    let peer = root.join("peer");
    write(&peer.join("skills"), "alpha", "Theirs");
    git(&peer, &["add", "-A"]);
    git(&peer, &["commit", "-m", "peer"]);
    git(&peer, &["push", "origin", "main"]);
    write(&repo.join("skills"), "beta", "Mine");
    state
}

fn journaled(state: &State) -> bool {
    journals::table
        .find(MERGE)
        .count()
        .get_result::<i64>(&mut *state.database.borrow_mut())
        .unwrap()
        > 0
}

/// The library holds both sides' changes as one merge, nothing waits on disk, and a Push
/// publishes it to `root`'s remote.
fn merged(state: &mut State, root: &Path) {
    let repo = state.library_dir();
    assert!(!journaled(state));
    assert!(!state.recovery_required);
    assert_eq!(
        git(&repo, &["rev-list", "--count", "--merges", "HEAD"]),
        "1"
    );
    assert!(body(state, "alpha").contains("Theirs"));
    assert!(body(state, "beta").contains("Mine"));
    assert_eq!(git(&repo, &["status", "--porcelain", "--", "skills"]), "");
    state
        .synchronize(SyncAction::Push, &BTreeMap::new())
        .unwrap();
    let published = git(&root.join("remote.git"), &["rev-parse", "main"]);
    assert_eq!(git(&repo, &["rev-parse", "HEAD"]), published);
}

#[test]
fn an_interrupted_merge_finishes_at_launch_from_each_step() {
    for point in ["merge_journaled", "merge_checked_out", "merge_moved"] {
        let directory = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(directory.path()).unwrap();
        let mut state = diverged(&root);
        FAULTS.with(|faults| *faults.borrow_mut() = [(point, true)].into());
        let sync = AssertUnwindSafe(|| state.synchronize(SyncAction::Sync, &BTreeMap::new()));
        assert!(catch_unwind(sync).is_err(), "{point}");
        assert!(journaled(&state), "{point}");
        drop(state);
        let mut reopened = open(&root);
        reopened.recover().unwrap();
        merged(&mut reopened, &root);
    }
}

#[test]
fn a_merge_that_fails_after_its_journal_row_finishes_at_once() {
    for point in ["merge_journaled", "merge_checked_out", "merge_moved"] {
        let directory = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(directory.path()).unwrap();
        let mut state = diverged(&root);
        FAULTS.with(|faults| *faults.borrow_mut() = [(point, false)].into());
        let failed = state.synchronize(SyncAction::Sync, &BTreeMap::new());
        assert!(failed.is_err(), "{point}");
        merged(&mut state, &root);
    }
}
