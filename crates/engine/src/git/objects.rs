//! Reading the library's own Git objects: whether a recorded tree is still there, and writing
//! one out for Reinstall without touching the library's index or working tree.

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
}
