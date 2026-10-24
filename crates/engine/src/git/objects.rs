//! Reading the library's own Git objects: whether a recorded tree is still there, and writing
//! one out for Reinstall without touching the library's index or working tree.

use super::{Git, failure, process};
use skillbinder_proto::{AppError, AppResult};
use std::{path::Path, time::Duration};

impl Git {
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
            &[
                "init",
                "--bare",
                "--quiet",
                "--object-format=sha1",
                "--",
                path,
            ],
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
}
