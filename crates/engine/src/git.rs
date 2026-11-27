//! Bounded Git operations for the library's skill directories and their records.
mod adoption;
mod commit;
mod discovery;
mod history;
mod install;
mod merge;
mod note;
mod objects;
mod pending;
mod process;
mod source;
mod status;
#[cfg(test)]
mod tests;
mod validation;

pub(crate) use commit::Committer;
pub(crate) use install::Installer;
pub(crate) use merge::Merge;
pub(crate) use source::validate as validate_source;
pub(crate) use validation::url as validate_url;

use crate::{shutdown::Shutdown, storage};
use skillbinder_proto::{
    AppError, AppResult, ErrorCategory, Reason, RemoteConfig, Side, SyncAction, SyncSnapshot,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    time::Duration,
};

const REMOTE_REF: &str = "refs/skillbinder/incoming";
/// The tree with no entries, the base of a library that shares no commit with its remote.
const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";
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
        self.perform_with(repo, remote, action, &BTreeMap::new(), &tests::Unnamed)
    }
    /// Executes an explicit operation; a Push or Sync commits through `committer`. A Sync with
    /// changes on both sides merges them, keeping the side `chosen` for each skill changed on
    /// both, and refuses unless every such skill has one.
    pub(crate) fn perform_with(
        &self,
        repo: &Path,
        remote: Option<&RemoteConfig>,
        action: SyncAction,
        chosen: &BTreeMap<String, Side>,
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
        let diverged = before.behind > 0 && (before.ahead > 0 || before.uncommitted);
        if !adopt && matches!(action, SyncAction::Pull) && (diverged || before.uncommitted) {
            return Err(failure("Push or Sync first: this device has changes."));
        }
        if matches!(action, SyncAction::Push) && before.behind > 0 {
            return Err(failure("Pull or Sync first: the remote has changes."));
        }
        self.scope(repo)?;
        if !adopt && diverged {
            let incoming = self
                .revision(repo, REMOTE_REF)?
                .ok_or_else(|| failure("Remote revision is unavailable."))?;
            self.merge(repo, &incoming, (&before.conflicts, chosen), committer)?;
            self.publish(repo, remote)?;
            return self.status(repo, Some(remote), true, &committer.reasons()?);
        }
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
                self.publish(repo, remote)?;
            }
        }
        self.status(repo, Some(remote), true, &committer.reasons()?)
    }
    /// Pushes HEAD to the remote's branch.
    fn publish(&self, repo: &Path, remote: &RemoteConfig) -> AppResult<()> {
        self.network_output(
            repo,
            &[
                "push",
                "--",
                &remote.url,
                &format!("HEAD:refs/heads/{}", remote.branch),
            ],
        )
        .map(drop)
    }
    fn disconnect_origin(&self, repo: &Path) -> AppResult<()> {
        let remotes = self.output(repo, &["remote"])?;
        if remotes.lines().any(|name| name == "origin") {
            self.output(repo, &["remote", "remove", "origin"])?;
        }
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
        let args: &[&str] = match from {
            Some(from) => &[
                "diff",
                "--no-ext-diff",
                "--no-renames",
                "--name-only",
                "-z",
                from,
                to,
                "--",
                MANAGED[0],
                MANAGED[1],
            ],
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
    /// Runs a local command against the index file `index` instead of the library's own.
    fn indexed_output(&self, repo: &Path, index: &Path, args: &[&str]) -> AppResult<String> {
        self.run_in(repo, Some(index), args, Duration::from_secs(30), None)
    }
    fn run(
        &self,
        repo: &Path,
        args: &[&str],
        timeout: Duration,
        stop: Option<&Shutdown>,
    ) -> AppResult<String> {
        self.run_in(repo, None, args, timeout, stop)
    }
    fn run_in(
        &self,
        repo: &Path,
        index: Option<&Path>,
        args: &[&str],
        timeout: Duration,
        stop: Option<&Shutdown>,
    ) -> AppResult<String> {
        let output = process::run_indexed(&self.program, Some(repo), index, args, timeout, stop)?;
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
fn failure(message: &str) -> AppError {
    AppError::new(
        ErrorCategory::Git,
        message,
        "Review synchronization settings and retry. See local diagnostics for details.",
    )
}
