//! Fetching a skill's source into a bare, shallow, blobless cache repository, and checking out
//! one of its folders. The cache is application data, never the library.

use super::{Git, failure, process, validation};
use crate::storage;
use skillbinder_proto::{AppError, AppResult, ErrorCategory, SkillSource};
use std::{fs, path::Path, time::Duration};

/// Where the fetched source commit is kept in a cache.
const SOURCE_REF: &str = "refs/skillbinder/source";
/// Fetches and lazy blob reads cross the network.
const NETWORK_TIMEOUT: Duration = Duration::from_secs(120);

/// One file or folder of a fetched tree, as `git ls-tree` lists it.
#[derive(Clone, Debug)]
pub struct TreeEntry {
    /// Such as `100644` for a file, `040000` for a folder, `160000` for a submodule.
    pub mode: String,
    /// `blob`, `tree`, or `commit`.
    pub kind: String,
    /// The id of its blob, tree, or commit.
    pub object: String,
    /// The path under the listed tree, `/`-separated.
    pub path: String,
}

impl Git {
    /// Fetches the commit `source` follows into the cache `cache`, creating the cache first.
    /// Blobs are fetched later, only for the folders that are checked out.
    pub fn fetch_source(&self, cache: &Path, source: &SkillSource) -> AppResult<()> {
        validate(source)?;
        if !cache.join("HEAD").is_file() {
            let _ = fs::remove_dir_all(cache);
            let parent = cache.parent().ok_or_else(AppError::storage)?;
            fs::create_dir_all(parent).map_err(storage::error_at("creating", parent))?;
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

    /// Each file and folder `tree` holds, read from tree objects alone, so no blob is fetched to
    /// list them. Sizes are never asked for, since reading one fetches its blob.
    pub fn tree_entries(&self, cache: &Path, tree: &str) -> AppResult<Vec<TreeEntry>> {
        let listed = self.output(cache, &["ls-tree", "-r", "-t", "-z", tree])?;
        // `<mode> <type> <object>\t<path>`
        Ok(listed
            .split_terminator('\0')
            .filter_map(|entry| {
                let (meta, path) = entry.split_once('\t')?;
                let mut fields = meta.split(' ');
                Some(TreeEntry {
                    mode: fields.next()?.to_owned(),
                    kind: fields.next()?.to_owned(),
                    object: fields.next()?.to_owned(),
                    path: path.to_owned(),
                })
            })
            .collect())
    }

    /// The text of the blob `object`, fetched from the source when the cache lacks it.
    pub fn read_blob(&self, cache: &Path, object: &str) -> AppResult<String> {
        String::from_utf8(self.read_blob_bytes(cache, object)?).map_err(|error| {
            failure("Git output is unreadable.")
                .because(format_args!("reading blob {object} as text: {error}"))
        })
    }

    /// The bytes of the blob `object`, fetched from the source when the cache lacks it; an error
    /// is always one of reading, never of the bytes themselves.
    pub fn read_blob_bytes(&self, cache: &Path, object: &str) -> AppResult<Vec<u8>> {
        self.network_bytes(Some(cache), &["cat-file", "blob", object])
    }

    /// Fetches the blobs `objects` into the cache in one request; reading each one missing
    /// would fetch it on its own.
    pub fn fetch_blobs(&self, cache: &Path, objects: &[&str]) -> AppResult<()> {
        if objects.is_empty() {
            return Ok(());
        }
        let mut args = vec![
            "-c",
            "fetch.negotiationAlgorithm=noop",
            "fetch",
            "--no-tags",
            "--no-write-fetch-head",
            "--recurse-submodules=no",
            "--filter=blob:none",
            "origin",
        ];
        args.extend(objects);
        self.network(Some(cache), &args).map(drop)
    }

    /// Writes the files of `tree` into the empty directory `into`, fetching missing blobs.
    pub fn checkout_tree(&self, cache: &Path, tree: &str, into: &Path) -> AppResult<()> {
        // The cache's index only ever describes the last checkout; start from an empty one.
        let index = cache.join("index");
        match fs::remove_file(&index) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                return Err(storage::error_at("removing", &index)(error));
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

    /// Runs a command that reads the source over the network, as [`Self::network_bytes`] does,
    /// and reads its output as text.
    fn network(&self, repo: Option<&Path>, args: &[&str]) -> AppResult<String> {
        String::from_utf8(self.network_bytes(repo, args)?).map_err(|error| {
            failure("Git output is unreadable.").because(process::unreadable(args, error))
        })
    }

    /// Runs a command that reads the source over the network. A host Git could not reach
    /// fails as [`ErrorCategory::Offline`]; any other failure is one of access, as [`failure`].
    fn network_bytes(&self, repo: Option<&Path>, args: &[&str]) -> AppResult<Vec<u8>> {
        let output = process::run(
            &self.program,
            repo,
            args,
            NETWORK_TIMEOUT,
            Some(&self.shutdown),
        )
        .map_err(|error| {
            if error.message == process::TIMED_OUT {
                AppError {
                    cause: error.cause,
                    ..offline(error.message)
                }
            } else {
                error
            }
        })?;
        if !output.status.success() {
            let message = "Git could not read the source. Check access and the repository.";
            let error = if unreachable(&output.stderr) {
                offline(message)
            } else {
                failure(message)
            };
            return Err(error.because(process::exited(args, &output)));
        }
        Ok(output.stdout)
    }
}

/// Whether Git's error output says the host could not be reached, rather than refusing access.
fn unreachable(stderr: &[u8]) -> bool {
    let stderr = String::from_utf8_lossy(stderr).to_ascii_lowercase();
    [
        "could not resolve host",
        "could not resolve hostname",
        "failed to connect",
        "couldn't connect to server",
        "connection timed out",
        "operation timed out",
        "network is unreachable",
        "no route to host",
        "connection refused",
    ]
    .iter()
    .any(|cause| stderr.contains(cause))
}

/// A source read that failed because the host could not be reached.
fn offline(message: impl Into<String>) -> AppError {
    AppError::new(
        ErrorCategory::Offline,
        message,
        "Check your connection and try again.",
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unreachable_host_is_told_apart_from_refused_access() {
        for stderr in [
            "fatal: unable to access 'https://x.invalid/': Could not resolve host: x.invalid",
            "ssh: Could not resolve hostname example.com: nodename nor servname provided",
            "fatal: unable to access 'https://github.com/': Failed to connect to github.com",
        ] {
            assert!(unreachable(stderr.as_bytes()), "{stderr}");
        }
        for stderr in [
            "fatal: could not read Username for 'https://github.com': terminal prompts disabled",
            "remote: Repository not found.",
            "git@github.com: Permission denied (publickey).",
        ] {
            assert!(!unreachable(stderr.as_bytes()), "{stderr}");
        }
    }
}
