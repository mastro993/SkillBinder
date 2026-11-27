//! Bounded Git operations for the library's skill directories and their records.
mod adoption;
mod commit;
mod discovery;
mod install;
mod note;
mod objects;
mod pending;
mod process;
mod source;
#[cfg(test)]
mod tests;
mod validation;

pub(crate) use commit::Committer;
pub(crate) use install::Installer;
pub(crate) use source::validate as validate_source;
pub(crate) use validation::url as validate_url;

use crate::{shutdown::Shutdown, storage};
use skillbinder_proto::{
    AppError, AppResult, ErrorCategory, Reason, RemoteConfig, SyncAction, SyncSnapshot, SyncState,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const REMOTE_REF: &str = "refs/skillbinder/incoming";
/// The managed paths, skill directories and their records: Sync and Push commit nothing else.
const MANAGED: [&str; 2] = ["skills", "records"];

/// A verified executable whose identity is fixed for this engine session.
#[derive(Clone, Debug)]
pub struct Git {
    program: process::Program,
    version: String,
    /// Stops a running command when the application quits, instead of waiting for its timeout.
    shutdown: Shutdown,
}

impl Git {
    /// Reports the verified installed version.
    pub fn version(&self) -> &str {
        &self.version
    }
    /// The executable's path.
    pub(crate) fn path(&self) -> &Path {
        &self.program.executable
    }
    /// Initializes an empty local library repository without committing files.
    pub fn initialize(&self, repo: &Path) -> AppResult<()> {
        if repo.join(".git").exists() {
            return Ok(());
        }
        std::fs::create_dir_all(repo).map_err(storage::error_at("creating", repo))?;
        // Records hold SHA-1 tree ids, whatever the user's default object format.
        self.output(
            repo,
            &["init", "--object-format=sha1", "--initial-branch=main"],
        )?;
        self.scope(repo)
    }
    /// Validates remote syntax and the complete Git branch grammar.
    pub fn validate_remote(&self, remote: &RemoteConfig) -> AppResult<()> {
        validation::remote(remote)?;
        let result = process::run(
            &self.program,
            None,
            &["check-ref-format", "--branch", &remote.branch],
            Duration::from_secs(30),
            None,
        )?;
        if result.status.success() {
            Ok(())
        } else {
            Err(AppError::validation("Choose a valid Git branch name."))
        }
    }
    /// Fetches a prospective remote without modifying library files or committing. `reasons`
    /// says why each skill changed, as [`Git::status`] takes them.
    pub fn connect(
        &self,
        repo: &Path,
        remote: &RemoteConfig,
        reasons: &BTreeMap<String, Reason>,
    ) -> AppResult<SyncSnapshot> {
        self.validate_remote(remote)?;
        self.preflight(repo)?;
        self.status(repo, Some(remote), true, reasons)
    }
    /// Executes an explicit operation. The engine owns persistence of remote selection.
    #[cfg(test)]
    pub fn perform(
        &self,
        repo: &Path,
        remote: Option<&RemoteConfig>,
        action: SyncAction,
    ) -> AppResult<SyncSnapshot> {
        self.perform_with(repo, remote, action, &tests::Unnamed)
    }
    /// Executes an explicit operation; a Push or Sync commits through `committer`.
    pub(crate) fn perform_with(
        &self,
        repo: &Path,
        remote: Option<&RemoteConfig>,
        action: SyncAction,
        committer: &dyn Committer,
    ) -> AppResult<SyncSnapshot> {
        if matches!(action, SyncAction::Disconnect) {
            self.disconnect_origin(repo)?;
            return self.status(repo, None, false, &BTreeMap::new());
        }
        if matches!(action, SyncAction::Refresh) {
            return self.status(repo, remote, true, &committer.reasons()?);
        }
        let remote = remote.ok_or_else(|| AppError::validation("Connect a remote first."))?;
        self.preflight(repo)?;
        let before = self.status(repo, Some(remote), true, &BTreeMap::new())?;
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
            if before.behind == 0 {
                committer.prepare()?;
            }
            self.commit(repo, committer)?;
            if self.revision(repo, "HEAD")?.is_some() {
                self.network_output(
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
        self.status(repo, Some(remote), true, &committer.reasons()?)
    }
    /// Classifies the state, optionally fetching, and lists what waits on each side. A skill
    /// changed on this device says why from `reasons`, by slug, else from the commits not pushed
    /// yet. Never commits or checks out files.
    pub fn status(
        &self,
        repo: &Path,
        remote: Option<&RemoteConfig>,
        fetch: bool,
        reasons: &BTreeMap<String, Reason>,
    ) -> AppResult<SyncSnapshot> {
        let porcelain = self.output(
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
        )?;
        let changed_files = changed_files(&porcelain);
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
        let (local_skills, incoming_skills) = if state == SyncState::Synced {
            (Vec::new(), None)
        } else {
            self.pending(
                repo,
                local.as_deref(),
                incoming.as_deref(),
                &porcelain,
                reasons,
            )?
        };
        Ok(SyncSnapshot {
            remote: Some(remote.clone()),
            state,
            ahead,
            behind,
            uncommitted: dirty,
            changed_files,
            local: local_skills,
            incoming: incoming_skills,
            refreshed_at: fetch.then(|| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            }),
            last_error: None,
            // The engine names the device.
            device: String::new(),
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
        let refs =
            self.network_output(repo, &["ls-remote", "--heads", "--", &remote.url, &branch])?;
        if refs.trim().is_empty() {
            self.output(repo, &["update-ref", "-d", REMOTE_REF])?;
            return Ok(());
        }
        self.network_output(
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
    /// The commit `name` resolves to, if any.
    pub fn revision(&self, repo: &Path, name: &str) -> AppResult<Option<String>> {
        let commit = format!("{name}^{{commit}}");
        let args = ["rev-parse", "--verify", "--quiet", &commit];
        let out = process::run(
            &self.program,
            Some(repo),
            &args,
            Duration::from_secs(30),
            None,
        )?;
        if out.status.success() {
            return String::from_utf8(out.stdout)
                .map(|s| Some(s.trim().to_owned()))
                .map_err(|error| {
                    failure("Git revision is unreadable.")
                        .because(process::unreadable(&args, error))
                });
        }
        if out.status.code() == Some(1) {
            Ok(None)
        } else {
            Err(failure("Git revision is unavailable.").because(process::exited(&args, &out)))
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
    /// Runs a local command. Quitting waits for it to finish, since it changes the library.
    fn output(&self, repo: &Path, args: &[&str]) -> AppResult<String> {
        self.run(repo, args, Duration::from_secs(30), None)
    }
    /// Runs a command that crosses the network. Quitting stops it instead of waiting for its
    /// timeout.
    fn network_output(&self, repo: &Path, args: &[&str]) -> AppResult<String> {
        self.run(repo, args, Duration::from_secs(30), Some(&self.shutdown))
    }
    fn run(
        &self,
        repo: &Path,
        args: &[&str],
        timeout: Duration,
        stop: Option<&Shutdown>,
    ) -> AppResult<String> {
        let output = process::run(&self.program, Some(repo), args, timeout, stop)?;
        if !output.status.success() {
            return Err(
                failure("Git operation failed. Check access and repository state.")
                    .because(process::exited(args, &output)),
            );
        }
        String::from_utf8(output.stdout).map_err(|error| {
            failure("Git output is unreadable.").because(process::unreadable(args, error))
        })
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
