//! Fetching a skill's source into a bare, shallow, blobless cache repository, and checking out
//! one of its folders. The cache is application data, never the library.

use super::{Git, failure, process, validation};
use skillbinder_proto::{AppError, AppResult, SkillSource};
use std::{fs, path::Path, time::Duration};

/// Where the fetched source commit is kept in a cache.
const SOURCE_REF: &str = "refs/skillbinder/source";
/// Fetches and lazy blob reads cross the network.
const NETWORK_TIMEOUT: Duration = Duration::from_secs(120);

impl Git {
    /// Fetches the commit `source` follows into the cache `cache`, creating the cache first.
    /// Blobs are fetched later, only for the folders that are checked out.
    pub fn fetch_source(&self, cache: &Path, source: &SkillSource) -> AppResult<()> {
        validate(source)?;
        if !cache.join("HEAD").is_file() {
            let _ = fs::remove_dir_all(cache);
            let parent = cache.parent().ok_or_else(AppError::storage)?;
            fs::create_dir_all(parent).map_err(|_| AppError::storage())?;
            let path = cache.to_str().ok_or_else(AppError::storage)?;
            self.network(
                None,
                &[
                    "clone",
                    "--bare",
                    "--depth=1",
                    "--filter=blob:none",
                    "--no-tags",
                    "--single-branch",
                    "--",
                    &source.url,
                    path,
                ],
            )?;
        }
        let wanted = source.reference.as_deref().unwrap_or("HEAD");
        self.network(
            Some(cache),
            &[
                "fetch",
                "--depth=1",
                "--filter=blob:none",
                "--no-tags",
                "--no-recurse-submodules",
                "--no-write-fetch-head",
                // The clone made the source the cache's promisor remote, which lazy blob reads use.
                "origin",
                &format!("+{wanted}:{SOURCE_REF}"),
            ],
        )
        .map(drop)
    }

    /// The tree of `path` in the last fetched source commit, or `None` when the commit has no
    /// folder there.
    pub fn source_tree(&self, cache: &Path, path: &str) -> AppResult<Option<String>> {
        if path.is_empty() {
            return Ok(Some(
                self.output(cache, &["rev-parse", &format!("{SOURCE_REF}^{{tree}}")])?
                    .trim()
                    .to_owned(),
            ));
        }
        let listed = self.output(
            cache,
            &[
                "--literal-pathspecs",
                "ls-tree",
                "-dz",
                SOURCE_REF,
                "--",
                path,
            ],
        )?;
        // `<mode> <type> <object>\t<path>`; only the named folder itself is listed.
        Ok(listed.split('\0').find_map(|entry| {
            let (meta, listed) = entry.split_once('\t')?;
            let mut fields = meta.split(' ');
            let (_, kind, object) = (fields.next()?, fields.next()?, fields.next()?);
            (listed == path && kind == "tree").then(|| object.to_owned())
        }))
    }

    /// How many files and folders `tree` holds, read from tree objects alone, so no blob is
    /// fetched to count them.
    pub fn tree_entries(&self, cache: &Path, tree: &str) -> AppResult<usize> {
        let listed = self.output(cache, &["ls-tree", "-r", "-t", "-z", "--name-only", tree])?;
        Ok(listed.split_terminator('\0').count())
    }

    /// Writes the files of `tree` into the empty directory `into`, fetching missing blobs.
    pub fn checkout_tree(&self, cache: &Path, tree: &str, into: &Path) -> AppResult<()> {
        // The cache's index only ever describes the last checkout; start from an empty one.
        match fs::remove_file(cache.join("index")) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                return Err(AppError::storage());
            }
            _ => {}
        }
        let work_tree = into.to_str().ok_or_else(AppError::storage)?;
        self.network(
            Some(cache),
            &[
                "--work-tree",
                work_tree,
                "-c",
                "core.symlinks=true",
                "read-tree",
                "--reset",
                "-u",
                tree,
            ],
        )
        .map(drop)
    }

    fn network(&self, repo: Option<&Path>, args: &[&str]) -> AppResult<String> {
        let output = process::run(&self.executable, repo, args, NETWORK_TIMEOUT)?;
        if !output.status.success() {
            return Err(failure(
                "Git could not read the source. Check access and the repository.",
            ));
        }
        String::from_utf8(output.stdout).map_err(|_| failure("Git output is unreadable."))
    }
}

/// Refuses a source Git could misread as an option, or whose folder leaves its repository.
pub(crate) fn validate(source: &SkillSource) -> AppResult<()> {
    let invalid = || AppError::validation("The skill's source is not a usable Git repository.");
    validation::url(&source.url).map_err(|_| invalid())?;
    if source.reference.as_deref().is_some_and(|reference| {
        reference.is_empty()
            || reference.starts_with('-')
            || reference.contains(':')
            || reference
                .chars()
                .any(|c| c.is_control() || c.is_whitespace())
    }) {
        return Err(invalid());
    }
    let path = &source.path;
    if !path.is_empty()
        && (path.starts_with('/')
            || path.contains('\\')
            || path.chars().any(char::is_control)
            || path.split('/').any(|part| {
                part.is_empty() || part == "." || part == ".." || part.starts_with('-')
            }))
    {
        return Err(invalid());
    }
    Ok(())
}
