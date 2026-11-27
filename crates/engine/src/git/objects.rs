//! Reading the library's own Git objects: whether a recorded tree is still there, a skill's
//! directory and record in a past commit, and writing a tree out for Reinstall or Restore without
//! touching the library's index or working tree.

use super::{Git, failure, process};
use crate::storage;
use skillbinder_proto::{AppError, AppResult};
use std::{path::Path, time::Duration};

impl Git {
    /// Whether the library's object database holds the tree `tree`.
    pub fn has_tree(&self, repo: &Path, tree: &str) -> AppResult<bool> {
        let out = process::run(
            &self.program,
            Some(repo),
            &["cat-file", "-t", "--end-of-options", tree],
            Duration::from_secs(30),
            None,
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
        let args = [
            "init",
            "--bare",
            "--quiet",
            "--object-format=sha1",
            "--",
            path,
        ];
        let unprepared = || failure("Git could not prepare the restore.");
        match process::run(&self.program, None, &args, Duration::from_secs(30), None) {
            Ok(out) if out.status.success() => {}
            Ok(out) => return Err(unprepared().because(process::exited(&args, &out))),
            Err(error) => {
                return Err(AppError {
                    cause: error.cause,
                    ..unprepared()
                });
            }
        }
        let objects = repo.join(".git/objects");
        let objects =
            dunce::canonicalize(&objects).map_err(storage::error_at("resolving", &objects))?;
        let objects = objects.to_str().ok_or_else(AppError::storage)?;
        let alternates = scratch.join("objects/info/alternates");
        std::fs::write(&alternates, format!("{objects}\n"))
            .map_err(storage::error_at("writing", &alternates))?;
        self.checkout_tree(scratch, tree, into)
    }

    /// The tree id of `skills/<slug>` in the commit `commit`, if it holds that directory.
    pub fn tree_at(&self, repo: &Path, commit: &str, slug: &str) -> AppResult<Option<String>> {
        self.entry_at(repo, commit, &format!("skills/{slug}"), "tree")
    }

    /// The text of `records/<slug>.toml` in the commit `commit`; `None` when it holds no such
    /// file, or one over `max` bytes, which is no record.
    pub fn record_at(
        &self,
        repo: &Path,
        commit: &str,
        slug: &str,
        max: u64,
    ) -> AppResult<Option<String>> {
        let Some(blob) = self.entry_at(repo, commit, &format!("records/{slug}.toml"), "blob")?
        else {
            return Ok(None);
        };
        let size: u64 = self
            .output(repo, &["cat-file", "-s", &blob])?
            .trim()
            .parse()
            .map_err(|_| failure("Git returned an invalid object size."))?;
        if size > max {
            return Ok(None);
        }
        self.output(repo, &["cat-file", "blob", &blob]).map(Some)
    }

    /// When the commit `commit` was made, in Unix seconds.
    pub fn commit_time(&self, repo: &Path, commit: &str) -> AppResult<u64> {
        self.output(repo, &["log", "-1", "--format=%ct", commit, "--"])?
            .trim()
            .parse()
            .map_err(|_| failure("Git history is unreadable."))
    }

    /// The id of the `kind` entry at `path` in the commit `commit`, if it holds one.
    pub(super) fn entry_at(
        &self,
        repo: &Path,
        commit: &str,
        path: &str,
        kind: &str,
    ) -> AppResult<Option<String>> {
        let listed = self.output(repo, &["ls-tree", "-z", commit, "--", path])?;
        Ok(listed.split('\0').find_map(|entry| {
            let (meta, listed) = entry.split_once('\t')?;
            let mut fields = meta.split(' ');
            let (_, found, id) = (fields.next()?, fields.next()?, fields.next()?);
            (listed == path && found == kind).then(|| id.to_owned())
        }))
    }
}
