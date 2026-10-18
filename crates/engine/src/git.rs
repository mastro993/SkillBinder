//! Bounded Git operations for the library's skill directories and their records.
mod adoption;
mod process;
mod source;
#[cfg(test)]
mod tests;
mod validation;

pub(crate) use source::validate as validate_source;

use skillbinder_proto::{
    AppError, AppResult, ErrorCategory, RemoteConfig, SyncAction, SyncSnapshot, SyncState,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const REMOTE_REF: &str = "refs/skillbinder/incoming";
/// The managed paths, skill directories and their records: Sync and Push commit nothing else.
const MANAGED: [&str; 2] = ["skills", "records"];

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
    /// Refuses a fetched or about-to-be-committed tree whose managed entries are not regular
    /// files or whose skill directories and records can't be checked out on every platform.
    fn validate_tree(&self, repo: &Path, revision: &str) -> AppResult<()> {
        let entries = self.output(
            repo,
            &["ls-tree", "-rz", revision, "--", MANAGED[0], MANAGED[1]],
        )?;
        let mut names = BTreeMap::new();
        let mut records = BTreeMap::new();
        for entry in entries.split('\0').filter(|p| !p.is_empty()) {
            let (mode, path) = entry
                .split_once(' ')
                .zip(entry.split_once('\t'))
                .map(|((mode, _), (_, path))| (mode, path))
                .ok_or_else(|| failure("Managed tree is invalid."))?;
            if mode != "100644" && mode != "100755" {
                return Err(failure("Managed content contains unsupported entries."));
            }
            let (slug, names) = match (skill_of(path), record_of(path)) {
                (Some(slug), _) => (slug, &mut names),
                (None, Some(slug)) if mode == "100644" => (slug, &mut records),
                _ => {
                    return Err(failure(
                        "Managed files must be skill directories or their records.",
                    ));
                }
            };
            let unique = *names
                .entry(crate::library::normalized(slug))
                .or_insert(slug)
                == slug;
            if !crate::payload::portable(slug) || !unique {
                return Err(failure(
                    "Skill directory names collide or are not portable.",
                ));
            }
        }
        Ok(())
    }
    /// The slugs whose skill directories or records differ between `from` and `to`; every slug
    /// in `to` when there is no `from`.
    pub fn changed_skills(
        &self,
        repo: &Path,
        from: Option<&str>,
        to: &str,
    ) -> AppResult<BTreeSet<String>> {
        let range;
        let args: &[&str] = match from {
            Some(from) => {
                range = format!("{from}..{to}");
                &[
                    "diff",
                    "--no-ext-diff",
                    "--no-renames",
                    "--name-only",
                    "-z",
                    &range,
                    "--",
                    MANAGED[0],
                    MANAGED[1],
                ]
            }
            None => &[
                "ls-tree",
                "-r",
                "--name-only",
                "-z",
                to,
                "--",
                MANAGED[0],
                MANAGED[1],
            ],
        };
        Ok(self
            .output(repo, args)?
            .split('\0')
            .filter_map(|path| skill_of(path).or_else(|| record_of(path)))
            .map(str::to_owned)
            .collect())
    }
    /// Whether the library's object database holds the tree `tree`.
    pub fn has_tree(&self, repo: &Path, tree: &str) -> AppResult<bool> {
        let out = process::run(
            &self.executable,
            Some(repo),
            &["cat-file", "-t", "--end-of-options", tree],
            Duration::from_secs(30),
        )?;
        Ok(out.status.success() && out.stdout.trim_ascii() == b"tree")
    }
    /// Writes the files of the library's tree `tree` into the empty directory `into`, through a
    /// scratch repository at `scratch` that borrows the library's objects, so the library's index
    /// and working tree are never touched.
    pub fn restore_tree(
        &self,
        repo: &Path,
        tree: &str,
        scratch: &Path,
        into: &Path,
    ) -> AppResult<()> {
        let path = scratch.to_str().ok_or_else(AppError::storage)?;
        process::run(
            &self.executable,
            None,
            &["init", "--bare", "--quiet", "--", path],
            Duration::from_secs(30),
        )
        .ok()
        .filter(|out| out.status.success())
        .ok_or_else(|| failure("Git could not prepare the restore."))?;
        let objects =
            std::fs::canonicalize(repo.join(".git/objects")).map_err(|_| AppError::storage())?;
        let objects = objects.to_str().ok_or_else(AppError::storage)?;
        std::fs::write(
            scratch.join("objects/info/alternates"),
            format!("{objects}\n"),
        )
        .map_err(|_| AppError::storage())?;
        self.checkout_tree(scratch, tree, into)
    }
    /// The commit `name` resolves to, if any.
    pub fn revision(&self, repo: &Path, name: &str) -> AppResult<Option<String>> {
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
                "/skills/",
                "/records/",
            ],
        )?;
        Ok(())
    }
    fn commit(&self, repo: &Path) -> AppResult<()> {
        // A pathspec absent both from HEAD and disk is not passed to `add`.
        for path in MANAGED {
            if repo.join(path).exists() || !self.output(repo, &["ls-files", "--", path])?.is_empty()
            {
                self.output(repo, &["add", "-f", "-A", "--", path])?;
            }
        }
        // Publish only what a fetch would accept, so a pushed commit never blocks later fetches.
        let staged = self.output(repo, &["write-tree"])?;
        self.validate_tree(repo, staged.trim())?;
        // A library without skills still publishes a first, empty commit.
        if self.revision(repo, "HEAD")?.is_none()
            || !self
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
                    "--allow-empty",
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
/// The slug of the skill directory holding a managed path, `skills/<slug>/...`.
fn skill_of(path: &str) -> Option<&str> {
    Some(path.strip_prefix("skills/")?.split_once('/')?.0)
}
/// The slug a record describes, `records/<slug>.toml`.
fn record_of(path: &str) -> Option<&str> {
    path.strip_prefix("records/")?
        .strip_suffix(".toml")
        .filter(|slug| !slug.is_empty() && !slug.contains('/'))
}
fn managed(path: &str) -> bool {
    MANAGED.iter().any(|root| {
        path.strip_prefix(root)
            .is_some_and(|rest| rest.starts_with('/'))
    })
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
