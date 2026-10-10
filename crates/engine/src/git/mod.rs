//! Bounded Git operations for the portable library.
mod adoption;
mod process;
#[cfg(test)]
mod tests;
mod validation;

use skillbinder_proto::{
    AppError, AppResult, ErrorCategory, RemoteConfig, SyncAction, SyncSnapshot, SyncState,
};
use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const REMOTE_REF: &str = "refs/skillbinder/incoming";
const MANAGED: &[&str] = &[".skillbinder.json", "skills"];

/// A verified executable whose identity is fixed for this engine session.
#[derive(Clone, Debug)]
pub struct Git {
    executable: PathBuf,
    version: String,
}

impl Git {
    /// Finds Git only in absolute PATH entries and requires version 2.39 or newer.
    pub fn discover() -> AppResult<Self> {
        let path = std::env::var_os("PATH").ok_or_else(|| failure("Git was not found."))?;
        for directory in std::env::split_paths(&path).filter(|p| p.is_absolute()) {
            let executable = directory.join(if cfg!(windows) { "git.exe" } else { "git" });
            if !executable.is_file() {
                continue;
            }
            let executable = executable
                .canonicalize()
                .map_err(|_| failure("Git could not be resolved."))?;
            let output = process::run(&executable, None, &["--version"], Duration::from_secs(30))?;
            let raw = String::from_utf8(output.stdout)
                .map_err(|_| failure("Git version is unreadable."))?;
            let version = validation::version(&raw)?;
            if !output.status.success() {
                return Err(failure("Git verification failed."));
            }
            return Ok(Self {
                executable,
                version,
            });
        }
        Err(failure(
            "Install Git 2.39 or newer and restart SkillBinder.",
        ))
    }
    /// Reports the verified installed version.
    pub fn version(&self) -> &str {
        &self.version
    }
    /// Initializes an empty local library repository without committing files.
    pub fn initialize(&self, repo: &Path) -> AppResult<()> {
        if repo.join(".git").exists() {
            return Ok(());
        }
        std::fs::create_dir_all(repo).map_err(|_| AppError::storage())?;
        self.output(repo, &["init", "--initial-branch=main"])?;
        self.scope(repo)
    }
    /// Validates remote syntax and the complete Git branch grammar.
    pub fn validate_remote(&self, remote: &RemoteConfig) -> AppResult<()> {
        validation::remote(remote)?;
        let result = process::run(
            &self.executable,
            None,
            &["check-ref-format", "--branch", &remote.branch],
            Duration::from_secs(30),
        )?;
        if result.status.success() {
            Ok(())
        } else {
            Err(AppError::validation("Choose a valid Git branch name."))
        }
    }
    /// Fetches a prospective remote without modifying library files or committing.
    pub fn connect(&self, repo: &Path, remote: &RemoteConfig) -> AppResult<SyncSnapshot> {
        self.validate_remote(remote)?;
        self.preflight(repo)?;
        self.status(repo, Some(remote), true)
    }
    /// Executes an explicit operation. The engine owns persistence of remote selection.
    pub fn perform(
        &self,
        repo: &Path,
        remote: Option<&RemoteConfig>,
        action: SyncAction,
    ) -> AppResult<SyncSnapshot> {
        if matches!(action, SyncAction::Disconnect) {
            self.disconnect_origin(repo)?;
            return self.status(repo, None, false);
        }
        if matches!(action, SyncAction::Refresh) {
            return self.status(repo, remote, true);
        }
        let remote = remote.ok_or_else(|| AppError::validation("Connect a remote first."))?;
        self.recover(repo)?;
        self.preflight(repo)?;
        let before = self.status(repo, Some(remote), true)?;
        let adopt = before.behind > 0
            && matches!(action, SyncAction::Pull | SyncAction::Sync)
            && self.can_adopt(repo)?;
        if !adopt && before.behind > 0 && (before.ahead > 0 || before.uncommitted) {
            return Err(failure(
                "Local and remote changes require manual reconciliation.",
            ));
        }
        if !adopt && matches!(action, SyncAction::Pull) && before.uncommitted {
            return Err(failure("Commit local managed changes before pulling."));
        }
        if matches!(action, SyncAction::Push) && before.behind > 0 {
            return Err(failure("Pull remote changes before pushing."));
        }
        self.scope(repo)?;
        if adopt {
            self.adopt(repo)?;
        } else if before.behind > 0 {
            self.output(repo, &["merge", "--ff-only", "--no-edit", REMOTE_REF])?;
        }
        if matches!(action, SyncAction::Push | SyncAction::Sync) {
            self.commit(repo)?;
            if self.revision(repo, "HEAD")?.is_some() {
                self.output(
                    repo,
                    &[
                        "push",
                        "--",
                        &remote.url,
                        &format!("HEAD:refs/heads/{}", remote.branch),
                    ],
                )?;
            }
        }
        self.status(repo, Some(remote), true)
    }
    /// Classifies the state, optionally fetching. Never commits or checks out files.
    pub fn status(
        &self,
        repo: &Path,
        remote: Option<&RemoteConfig>,
        fetch: bool,
    ) -> AppResult<SyncSnapshot> {
        let changed_files = changed_files(&self.output(
            repo,
            &[
                "status",
                "--porcelain=v1",
                "-z",
                "--untracked-files=all",
                "--ignored=matching",
                "--",
                MANAGED[0],
                MANAGED[1],
            ],
        )?);
        let dirty = changed_files > 0;
        let Some(remote) = remote else {
            return Ok(SyncSnapshot {
                uncommitted: dirty,
                changed_files,
                ..Default::default()
            });
        };
        self.validate_remote(remote)?;
        if fetch {
            self.reconcile_origin(repo, remote)?;
            self.fetch(repo, remote)?;
        }
        let local = self.revision(repo, "HEAD")?;
        let incoming = self.revision(repo, REMOTE_REF)?;
        let (ahead, behind) = match (&local, &incoming) {
            (Some(local), Some(incoming)) => {
                let counts = self.output(
                    repo,
                    &[
                        "rev-list",
                        "--left-right",
                        "--count",
                        &format!("{local}...{incoming}"),
                    ],
                )?;
                let mut counts = counts.split_whitespace();
                (count(counts.next())?, count(counts.next())?)
            }
            (Some(_), None) => (1, 0),
            (None, Some(_)) => (0, 1),
            (None, None) => (0, 0),
        };
        let state = if behind > 0 && (ahead > 0 || dirty) {
            SyncState::NeedsSync
        } else if behind > 0 {
            SyncState::NeedsPull
        } else if ahead > 0 || dirty || incoming.is_none() {
            SyncState::NeedsPush
        } else {
            SyncState::Synced
        };
        Ok(SyncSnapshot {
            remote: Some(remote.clone()),
            state,
            ahead,
            behind,
            uncommitted: dirty,
            changed_files,
            refreshed_at: fetch.then(|| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            }),
        })
    }
    fn reconcile_origin(&self, repo: &Path, remote: &RemoteConfig) -> AppResult<()> {
        self.output(
            repo,
            &[
                "config",
                "--local",
                "--replace-all",
                "remote.origin.url",
                &remote.url,
            ],
        )?;
        Ok(())
    }
    fn disconnect_origin(&self, repo: &Path) -> AppResult<()> {
        let remotes = self.output(repo, &["remote"])?;
        if remotes.lines().any(|name| name == "origin") {
            self.output(repo, &["remote", "remove", "origin"])?;
        }
        Ok(())
    }
    fn fetch(&self, repo: &Path, remote: &RemoteConfig) -> AppResult<()> {
        let branch = format!("refs/heads/{}", remote.branch);
        let refs = self.output(repo, &["ls-remote", "--heads", "--", &remote.url, &branch])?;
        if refs.trim().is_empty() {
            self.output(repo, &["update-ref", "-d", REMOTE_REF])?;
            return Ok(());
        }
        self.output(
            repo,
            &[
                "fetch",
                "--no-tags",
                "--no-recurse-submodules",
                "--no-write-fetch-head",
                "--",
                &remote.url,
                &branch,
            ],
        )?;
        let expected = refs
            .split_whitespace()
            .next()
            .ok_or_else(|| failure("Remote revision is unavailable."))?;
        // Use the advertised immutable object, never FETCH_HEAD from an unrelated operation.
        self.validate_tree(repo, expected)?;
        self.output(repo, &["update-ref", REMOTE_REF, expected])?;
        Ok(())
    }
    fn validate_tree(&self, repo: &Path, revision: &str) -> AppResult<()> {
        let metadata = self.output(repo, &["show", &format!("{revision}:.skillbinder.json")])?;
        validation::metadata(metadata.as_bytes())?;
        let entries = self.output(
            repo,
            &["ls-tree", "-rz", revision, "--", MANAGED[0], MANAGED[1]],
        )?;
        for entry in entries.split('\0').filter(|p| !p.is_empty()) {
            let (mode, _) = entry
                .split_once(' ')
                .ok_or_else(|| failure("Remote tree is invalid."))?;
            if mode != "100644" && mode != "100755" {
                return Err(failure(
                    "Remote managed content contains unsupported entries.",
                ));
            }
        }
        Ok(())
    }
    fn revision(&self, repo: &Path, name: &str) -> AppResult<Option<String>> {
        let out = process::run(
            &self.executable,
            Some(repo),
            &[
                "rev-parse",
                "--verify",
                "--quiet",
                &format!("{name}^{{commit}}"),
            ],
            Duration::from_secs(30),
        )?;
        if out.status.success() {
            return String::from_utf8(out.stdout)
                .map(|s| Some(s.trim().to_owned()))
                .map_err(|_| failure("Git revision is unreadable."));
        }
        if out.status.code() == Some(1) {
            Ok(None)
        } else {
            Err(failure("Git revision is unavailable."))
        }
    }
    fn preflight(&self, repo: &Path) -> AppResult<()> {
        let staged = self.output(
            repo,
            &["diff", "--cached", "--no-renames", "--name-only", "-z"],
        )?;
        if staged
            .split('\0')
            .filter(|p| !p.is_empty())
            .any(|p| !managed(p))
        {
            return Err(failure(
                "Unrelated staged files block synchronization. Preserve or unstage them first.",
            ));
        }
        Ok(())
    }
    fn scope(&self, repo: &Path) -> AppResult<()> {
        // Never sparsify an existing foreign worktree file, even when Git considers it clean.
        let tracked = self.output(repo, &["ls-files", "-z"])?;
        for path in tracked.split('\0').filter(|p| !p.is_empty() && !managed(p)) {
            if repo.join(path).symlink_metadata().is_ok() {
                return Err(failure(
                    "Unrelated tracked files are present. Use a dedicated library repository.",
                ));
            }
        }
        self.output(
            repo,
            &[
                "sparse-checkout",
                "set",
                "--no-cone",
                "--no-sparse-index",
                "--",
                "/.skillbinder.json",
                "/skills/",
            ],
        )?;
        Ok(())
    }
    fn commit(&self, repo: &Path) -> AppResult<()> {
        let metadata =
            std::fs::read(repo.join(".skillbinder.json")).map_err(|_| AppError::storage())?;
        validation::metadata(&metadata)?;
        // Pathspecs which are absent both from HEAD and disk are not passed to `add`.
        for path in MANAGED {
            if repo.join(path).exists() || !self.output(repo, &["ls-files", "--", path])?.is_empty()
            {
                self.output(repo, &["add", "-f", "-A", "--", path])?;
            }
        }
        if !self
            .output(repo, &["diff", "--cached", "--name-only", "-z"])?
            .is_empty()
        {
            self.output(
                repo,
                &[
                    "-c",
                    "user.name=SkillBinder",
                    "-c",
                    "user.email=local@skillbinder.invalid",
                    "commit",
                    "-m",
                    "chore(skills): synchronize library",
                ],
            )?;
        }
        Ok(())
    }
    fn output(&self, repo: &Path, args: &[&str]) -> AppResult<String> {
        let output = process::run(&self.executable, Some(repo), args, Duration::from_secs(30))?;
        if !output.status.success() {
            return Err(failure(
                "Git operation failed. Check access and repository state.",
            ));
        }
        String::from_utf8(output.stdout).map_err(|_| failure("Git output is unreadable."))
    }
}
fn managed(path: &str) -> bool {
    path == ".skillbinder.json" || path.starts_with("skills/")
}
/// Counts `git status --porcelain=v1 -z` entries; renames and copies carry a second path.
fn changed_files(status: &str) -> u64 {
    let mut paths = status.split_terminator('\0');
    let mut count = 0;
    while let Some(entry) = paths.next() {
        if entry.get(..2).is_some_and(|xy| xy.contains(['R', 'C'])) {
            paths.next();
        }
        count += 1;
    }
    count
}
fn count(value: Option<&str>) -> AppResult<u64> {
    value
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| failure("Git returned invalid revision counts."))
}
fn failure(message: &str) -> AppError {
    AppError::new(
        ErrorCategory::Git,
        message,
        "Review synchronization settings and retry. See local diagnostics for details.",
    )
}
