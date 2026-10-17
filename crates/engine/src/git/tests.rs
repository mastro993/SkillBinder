#![allow(clippy::unwrap_used)]
use super::*;
use std::fs;
use tempfile::TempDir;

struct Fixture {
    _root: TempDir,
    repo: PathBuf,
    peer: PathBuf,
    remote: RemoteConfig,
    git: Git,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("library");
        let bare = root.path().join("remote.git");
        let peer = root.path().join("peer");
        fs::create_dir(&bare).unwrap();
        let git = Git::discover().unwrap();
        git.output(&bare, &["init", "--bare", "--initial-branch=main"])
            .unwrap();
        git.initialize(&repo).unwrap();
        fs::create_dir_all(repo.join("skills/demo")).unwrap();
        fs::write(repo.join("skills/demo/SKILL.md"), "initial").unwrap();
        let remote = RemoteConfig {
            url: url::Url::from_file_path(&bare).unwrap().into(),
            branch: "main".into(),
        };
        Self {
            _root: root,
            repo,
            peer,
            remote,
            git,
        }
    }
    fn push(&self) {
        self.git
            .perform(&self.repo, Some(&self.remote), SyncAction::Push)
            .unwrap();
    }
    fn peer(&self) {
        self.git
            .output(self._root.path(), &["clone", &self.remote.url, "peer"])
            .unwrap();
    }
    fn peer_commit(&self, file: &str, value: &str) {
        let path = self.peer.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, value).unwrap();
        self.git.output(&self.peer, &["add", "."]).unwrap();
        self.git
            .output(
                &self.peer,
                &[
                    "-c",
                    "user.name=Test",
                    "-c",
                    "user.email=test@example.invalid",
                    "commit",
                    "-m",
                    "peer",
                ],
            )
            .unwrap();
        self.git
            .output(&self.peer, &["push", "origin", "main"])
            .unwrap();
    }
}
#[test]
fn empty_remote_connect_never_commits_then_push_and_pull_work() {
    let f = Fixture::new();
    assert_eq!(
        f.git.connect(&f.repo, &f.remote).unwrap().state,
        SyncState::NeedsPush
    );
    assert!(f.git.revision(&f.repo, "HEAD").unwrap().is_none());
    f.push();
    f.peer();
    f.peer_commit("skills/demo/SKILL.md", "remote");
    assert_eq!(
        f.git.status(&f.repo, Some(&f.remote), true).unwrap().state,
        SyncState::NeedsPull
    );
    f.git
        .perform(&f.repo, Some(&f.remote), SyncAction::Pull)
        .unwrap();
    assert_eq!(
        fs::read_to_string(f.repo.join("skills/demo/SKILL.md")).unwrap(),
        "remote"
    );
}
#[test]
fn a_library_without_skills_publishes_an_empty_first_commit() {
    let f = Fixture::new();
    fs::remove_dir_all(f.repo.join("skills/demo")).unwrap();
    let pushed = f
        .git
        .perform(&f.repo, Some(&f.remote), SyncAction::Push)
        .unwrap();
    assert_eq!(pushed.state, SyncState::Synced);
    assert!(f.git.revision(&f.repo, "HEAD").unwrap().is_some());
}
#[test]
fn a_stray_local_file_in_skills_is_never_committed_or_pushed() {
    let f = Fixture::new();
    f.push();
    let head = f.git.revision(&f.repo, "HEAD").unwrap();
    let remote = || {
        f.git
            .output(&f.repo, &["ls-remote", "--", &f.remote.url])
            .unwrap()
    };
    let published = remote();
    fs::write(f.repo.join("skills/.DS_Store"), "finder").unwrap();
    fs::write(f.repo.join("skills/demo/SKILL.md"), "local").unwrap();
    assert!(
        f.git
            .perform(&f.repo, Some(&f.remote), SyncAction::Push)
            .is_err()
    );
    assert_eq!(f.git.revision(&f.repo, "HEAD").unwrap(), head);
    assert_eq!(remote(), published);
}
#[test]
fn foreign_staged_file_refuses_without_changing_index_or_worktree() {
    let f = Fixture::new();
    f.push();
    fs::write(f.repo.join("foreign.txt"), "private").unwrap();
    f.git
        .output(&f.repo, &["add", "--sparse", "foreign.txt"])
        .unwrap();
    let index = fs::read(f.repo.join(".git/index")).unwrap();
    fs::write(f.repo.join("skills/demo/SKILL.md"), "local").unwrap();
    assert!(
        f.git
            .perform(&f.repo, Some(&f.remote), SyncAction::Push)
            .is_err()
    );
    assert_eq!(index, fs::read(f.repo.join(".git/index")).unwrap());
    assert_eq!(
        fs::read_to_string(f.repo.join("foreign.txt")).unwrap(),
        "private"
    );
    assert_eq!(
        fs::read_to_string(f.repo.join("skills/demo/SKILL.md")).unwrap(),
        "local"
    );
}
#[test]
fn incoming_unrelated_files_never_materialize_and_local_untracked_survives() {
    let f = Fixture::new();
    f.push();
    f.peer();
    fs::write(f.repo.join("notes.txt"), "private").unwrap();
    f.peer_commit("notes.txt", "remote foreign");
    f.git
        .perform(&f.repo, Some(&f.remote), SyncAction::Pull)
        .unwrap();
    assert_eq!(
        fs::read_to_string(f.repo.join("notes.txt")).unwrap(),
        "private"
    );
}
#[test]
fn new_remote_foreign_files_stay_sparse() {
    let f = Fixture::new();
    f.push();
    f.peer();
    f.peer_commit("unrelated.txt", "remote foreign");
    f.git
        .perform(&f.repo, Some(&f.remote), SyncAction::Pull)
        .unwrap();
    assert!(!f.repo.join("unrelated.txt").exists());
    fs::write(f.repo.join("skills/demo/SKILL.md"), "local").unwrap();
    f.push();
    assert_eq!(
        f.git
            .output(&f.peer, &["show", "HEAD:unrelated.txt"])
            .unwrap(),
        "remote foreign"
    );
}
#[test]
fn dirty_behind_and_diverged_histories_are_refused() {
    let f = Fixture::new();
    f.push();
    f.peer();
    f.peer_commit("skills/demo/SKILL.md", "remote");
    fs::write(f.repo.join("skills/demo/SKILL.md"), "local").unwrap();
    for action in [SyncAction::Pull, SyncAction::Push, SyncAction::Sync] {
        assert!(f.git.perform(&f.repo, Some(&f.remote), action).is_err());
    }
    f.git.commit(&f.repo).unwrap();
    assert!(
        f.git
            .perform(&f.repo, Some(&f.remote), SyncAction::Sync)
            .is_err()
    );
}
#[test]
fn non_portable_remote_skill_names_do_not_reach_worktree() {
    let f = Fixture::new();
    f.push();
    f.peer();
    f.peer_commit("skills/aux/SKILL.md", "reserved on Windows");
    assert!(
        f.git
            .perform(&f.repo, Some(&f.remote), SyncAction::Pull)
            .is_err()
    );
    assert!(!f.repo.join("skills/aux").exists());
}

#[test]
fn remote_files_outside_skill_directories_do_not_reach_worktree() {
    let f = Fixture::new();
    f.push();
    f.peer();
    f.peer_commit("skills/notes", "a file, not a skill directory");
    assert!(
        f.git
            .perform(&f.repo, Some(&f.remote), SyncAction::Pull)
            .is_err()
    );
    assert!(!f.repo.join("skills/notes").exists());
}

#[test]
fn branches_follow_git_grammar_and_refresh_preserves_index() {
    let f = Fixture::new();
    f.push();
    for branch in ["-bad", "main.lock", "a..b", "a/.b", "@{-1}", "main\n"] {
        assert!(
            f.git
                .validate_remote(&RemoteConfig {
                    url: f.remote.url.clone(),
                    branch: branch.into()
                })
                .is_err(),
            "{branch}"
        );
    }
    let index = fs::read(f.repo.join(".git/index")).unwrap();
    fs::write(f.repo.join("skills/demo/SKILL.md"), "dirty").unwrap();
    let head = f.git.revision(&f.repo, "HEAD").unwrap();
    assert_eq!(
        f.git
            .perform(&f.repo, Some(&f.remote), SyncAction::Refresh)
            .unwrap()
            .state,
        SyncState::NeedsPush
    );
    assert_eq!(head, f.git.revision(&f.repo, "HEAD").unwrap());
    assert_eq!(index, fs::read(f.repo.join(".git/index")).unwrap());
}

#[test]
fn tracked_foreign_worktree_refusal_preserves_index_and_bytes() {
    let f = Fixture::new();
    f.push();
    f.peer();
    f.peer_commit("unrelated.txt", "remote");
    f.git
        .perform(&f.repo, Some(&f.remote), SyncAction::Pull)
        .unwrap();
    fs::write(f.repo.join("unrelated.txt"), "local foreign edit").unwrap();
    fs::write(f.repo.join("skills/demo/SKILL.md"), "managed edit").unwrap();
    let index = fs::read(f.repo.join(".git/index")).unwrap();
    assert!(
        f.git
            .perform(&f.repo, Some(&f.remote), SyncAction::Push)
            .is_err()
    );
    assert_eq!(index, fs::read(f.repo.join(".git/index")).unwrap());
    assert_eq!(
        fs::read_to_string(f.repo.join("unrelated.txt")).unwrap(),
        "local foreign edit"
    );
    assert_eq!(
        fs::read_to_string(f.repo.join("skills/demo/SKILL.md")).unwrap(),
        "managed edit"
    );
}

#[test]
fn ignored_foreign_files_survive_explicit_push() {
    let f = Fixture::new();
    f.push();
    fs::create_dir(f.repo.join("foreign")).unwrap();
    fs::write(f.repo.join("foreign/private"), "private bytes").unwrap();
    fs::write(f.repo.join(".git/info/exclude"), "foreign/\n").unwrap();
    fs::write(f.repo.join("skills/demo/SKILL.md"), "changed").unwrap();
    f.push();
    assert_eq!(
        fs::read_to_string(f.repo.join("foreign/private")).unwrap(),
        "private bytes"
    );
    assert!(
        !f.git
            .output(&f.repo, &["ls-tree", "-r", "--name-only", "HEAD"])
            .unwrap()
            .contains("foreign")
    );
}

#[test]
fn fresh_library_connect_is_fetch_only_then_sync_adopts_remote() {
    let f = Fixture::new();
    f.push();
    f.peer();
    f.peer_commit("unrelated.txt", "remote-only bytes");
    let fresh = f._root.path().join("fresh");
    f.git.initialize(&fresh).unwrap();
    fs::create_dir(fresh.join("skills")).unwrap();
    fs::write(fresh.join("notes.txt"), "local-only bytes").unwrap();
    let status = f.git.connect(&fresh, &f.remote).unwrap();
    assert_eq!(status.behind, 1);
    assert!(f.git.revision(&fresh, "HEAD").unwrap().is_none());
    f.git
        .perform(&fresh, Some(&f.remote), SyncAction::Sync)
        .unwrap();
    assert_eq!(
        fs::read_to_string(fresh.join("skills/demo/SKILL.md")).unwrap(),
        "initial"
    );
    assert!(!fresh.join("unrelated.txt").exists());
    assert_eq!(
        fs::read_to_string(fresh.join("notes.txt")).unwrap(),
        "local-only bytes"
    );
}

#[test]
fn fresh_library_with_managed_payload_refuses_adoption() {
    let f = Fixture::new();
    f.push();
    let fresh = f._root.path().join("fresh");
    f.git.initialize(&fresh).unwrap();
    fs::create_dir_all(fresh.join("skills/local")).unwrap();
    fs::write(fresh.join("skills/local/SKILL.md"), "local").unwrap();
    assert!(
        f.git
            .perform(&fresh, Some(&f.remote), SyncAction::Sync)
            .is_err()
    );
    assert_eq!(
        fs::read_to_string(fresh.join("skills/local/SKILL.md")).unwrap(),
        "local"
    );
}

#[test]
fn connection_reconciles_origin_and_disconnect_removes_it() {
    let f = Fixture::new();
    f.git.connect(&f.repo, &f.remote).unwrap();
    assert_eq!(
        f.git
            .output(
                &f.repo,
                &["config", "--local", "--get", "remote.origin.url"]
            )
            .unwrap()
            .trim(),
        f.remote.url
    );
    f.git
        .perform(&f.repo, Some(&f.remote), SyncAction::Disconnect)
        .unwrap();
    assert!(
        !f.git
            .output(&f.repo, &["remote"])
            .unwrap()
            .lines()
            .any(|name| name == "origin")
    );
}

#[cfg(unix)]
#[test]
fn review_remote_attributes_do_not_execute_global_filters() {
    use std::os::unix::fs::PermissionsExt;
    let mut f = Fixture::new();
    f.push();
    f.peer();
    fs::write(
        f.peer.join("skills/demo/.gitattributes"),
        "SKILL.md filter=fixture\n",
    )
    .unwrap();
    f.peer_commit("skills/demo/SKILL.md", "remote original");
    let home = f._root.path().join("filter-home");
    fs::create_dir(&home).unwrap();
    fs::write(
        home.join(".gitconfig"),
        "[filter \"fixture\"]\n smudge = sed s/original/transformed/\n clean = sed s/original/transformed/\n",
    )
    .unwrap();
    let wrapper = home.join("git-wrapper");
    fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\nexport HOME='{}'\nexport XDG_CONFIG_HOME='{}'\nexec '{}' \"$@\"\n",
            home.display(),
            home.display(),
            f.git.executable.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700)).unwrap();
    f.git.executable = wrapper;
    f.git
        .perform(&f.repo, Some(&f.remote), SyncAction::Pull)
        .unwrap();
    assert_eq!(
        fs::read_to_string(f.repo.join("skills/demo/SKILL.md")).unwrap(),
        "remote original",
        "pull must preserve exact incoming payload bytes"
    );
    fs::write(f.repo.join("skills/demo/SKILL.md"), "local original").unwrap();
    f.push();
    assert_eq!(
        f.git
            .output(&f.repo, &["show", "HEAD:skills/demo/SKILL.md"])
            .unwrap(),
        "local original"
    );
    fs::write(
        home.join(".gitconfig"),
        "[filter \"fixture\"]\n process = false\n required = true\n",
    )
    .unwrap();
    fs::write(
        f.repo.join("skills/demo/SKILL.md"),
        "process filter must not run",
    )
    .unwrap();
    f.push();
    assert_eq!(
        f.git
            .output(&f.repo, &["show", "HEAD:skills/demo/SKILL.md"])
            .unwrap(),
        "process filter must not run"
    );
}

#[test]
fn incoming_conversion_attributes_preserve_exact_bytes() {
    let f = Fixture::new();
    f.push();
    f.peer();
    let content = "$Id$\nremote original\n";
    fs::write(
        f.peer.join("skills/demo/.gitattributes"),
        "SKILL.md text eol=crlf ident working-tree-encoding=UTF-16LE\n",
    )
    .unwrap();
    f.peer_commit("skills/demo/SKILL.md", content);
    f.git
        .perform(&f.repo, Some(&f.remote), SyncAction::Pull)
        .unwrap();
    assert_eq!(
        fs::read_to_string(f.repo.join("skills/demo/SKILL.md")).unwrap(),
        content
    );
    fs::write(f.repo.join("skills/demo/SKILL.md"), "changed $Id$\n").unwrap();
    f.push();
    assert_eq!(
        f.git
            .output(&f.repo, &["show", "HEAD:skills/demo/SKILL.md"])
            .unwrap(),
        "changed $Id$\n"
    );
}

#[test]
fn unexpected_local_attributes_are_preserved_and_refused() {
    let f = Fixture::new();
    let attributes = f.repo.join(".git/info/attributes");
    let original = "* filter=unexpected\n";
    fs::write(&attributes, original).unwrap();
    assert!(
        f.git
            .perform(&f.repo, Some(&f.remote), SyncAction::Push)
            .is_err()
    );
    assert_eq!(fs::read_to_string(attributes).unwrap(), original);
}

#[test]
fn review_sync_keeps_ignored_managed_payload_files() {
    let f = Fixture::new();
    fs::write(f.repo.join("skills/demo/.gitignore"), "SKILL.md\n").unwrap();
    f.push();
    assert_eq!(
        f.git
            .output(&f.repo, &["show", "HEAD:skills/demo/SKILL.md"])
            .unwrap(),
        "initial"
    );
}

#[test]
fn ignored_managed_changes_are_dirty_and_foreign_ignored_files_stay_local() {
    let f = Fixture::new();
    f.push();
    fs::write(f.repo.join(".git/info/exclude"), "*.txt\n").unwrap();
    fs::write(f.repo.join("skills/demo/new.txt"), "managed bytes").unwrap();
    fs::write(f.repo.join("private.txt"), "private bytes").unwrap();
    let status = f.git.status(&f.repo, Some(&f.remote), false).unwrap();
    assert!(status.uncommitted);
    assert_eq!(status.changed_files, 1);
    f.push();
    assert_eq!(
        f.git
            .output(&f.repo, &["show", "HEAD:skills/demo/new.txt"])
            .unwrap(),
        "managed bytes"
    );
    assert!(
        !f.git
            .status(&f.repo, Some(&f.remote), false)
            .unwrap()
            .uncommitted
    );
    assert_eq!(
        fs::read_to_string(f.repo.join("private.txt")).unwrap(),
        "private bytes"
    );
    assert!(
        !f.git
            .output(&f.repo, &["ls-tree", "-r", "--name-only", "HEAD"])
            .unwrap()
            .contains("private.txt")
    );
}
#[test]
fn changed_files_counts_renames_once() {
    assert_eq!(changed_files(""), 0);
    assert_eq!(
        changed_files(" M skills/a/SKILL.md\0R  skills/b/new.md\0skills/b/old.md\0?? x\0"),
        3
    );
}
#[test]
fn sources_publish_beside_skills_and_nothing_else_under_them() {
    let f = Fixture::new();
    fs::create_dir_all(f.repo.join("sources")).unwrap();
    fs::write(f.repo.join("sources/demo.toml"), "url = \"x\"\n").unwrap();
    f.push();
    let published = f
        .git
        .output(&f.repo, &["ls-tree", "-r", "--name-only", "HEAD"])
        .unwrap();
    assert_eq!(published, "skills/demo/SKILL.md\nsources/demo.toml\n");
    let head = f.git.revision(&f.repo, "HEAD").unwrap();
    for stray in ["sources/nested/demo.toml", "sources/notes.txt"] {
        let path = f.repo.join(stray);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "stray").unwrap();
        assert!(
            f.git
                .perform(&f.repo, Some(&f.remote), SyncAction::Push)
                .is_err(),
            "{stray}"
        );
        assert_eq!(f.git.revision(&f.repo, "HEAD").unwrap(), head);
        fs::remove_file(&path).unwrap();
    }
}
