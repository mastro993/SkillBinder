//! Bounded read-only payload inspection and verified link-free materialization.
mod frontmatter;
mod manifest;
mod traversal;
mod tree;
pub use manifest::{Manifest, ManifestEntry, ManifestKind};
pub(crate) use traversal::{GIT_EXCLUDED, MAX_ENTRIES, portable};

use serde::{Deserialize, Serialize};
use skillbinder_proto::{AppError, AppResult, ValidationMessage, ValidationStatus};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Identity guards replacement of a source directory between prepare and apply.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceIdentity {
    /// Canonical source directory.
    pub canonical_path: PathBuf,
    #[cfg(unix)]
    /// Unix filesystem device identifier.
    pub device: u64,
    #[cfg(unix)]
    /// Unix directory inode.
    pub inode: u64,
    #[cfg(not(unix))]
    /// Creation timestamp fallback where inode identity is unavailable.
    pub created_nanos: Option<u128>,
}
impl SourceIdentity {
    fn read(canonical_path: &Path) -> AppResult<Self> {
        let metadata = fs::metadata(canonical_path).map_err(|_| AppError::storage())?;
        if !metadata.is_dir() {
            return Err(AppError::validation("A skill source must be a directory."));
        }
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        Ok(Self {
            canonical_path: canonical_path.to_owned(),
            #[cfg(unix)]
            device: metadata.dev(),
            #[cfg(unix)]
            inode: metadata.ino(),
            #[cfg(not(unix))]
            created_nanos: metadata
                .created()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos()),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
/// Immutable source snapshot for prepare and apply validation.
pub struct Inspection {
    /// Source directory after resolving root links.
    pub canonical_source: PathBuf,
    /// Physical source identity at inspection.
    pub identity: SourceIdentity,
    /// Source directory basename.
    pub slug: String,
    /// Valid string description extracted from frontmatter.
    pub description: String,
    /// Strongest validation finding.
    pub status: ValidationStatus,
    /// Ordered validation explanations.
    pub messages: Vec<ValidationMessage>,
    /// Materialized content fingerprint and entries.
    pub manifest: Manifest,
    /// The Git tree id of the materialized content. Plans saved before it existed read an empty
    /// one, which no inspection matches, so applying them reports the source as changed.
    #[serde(default)]
    pub tree: String,
    /// Unix seconds of the newest file's modification, if any was read.
    #[serde(default)]
    pub modified: Option<u64>,
}

/// Inspects a source without writing it. Unsafe payloads return a blocked report;
/// inaccessible source roots return an error. Run outside the UI thread.
pub fn inspect(path: &Path) -> AppResult<Inspection> {
    Ok(traversal::snapshot(path)?.inspection)
}

/// Creates an absent destination from bounded in-memory bytes. Both source identity
/// and manifest must still match the inspection; all links become ordinary entries.
/// Failed staging leaves no destination. The caller owns the destination parent.
pub fn materialize(expected: &Inspection, destination: &Path) -> AppResult<()> {
    let snapshot = verified(expected, destination)?;
    let parent = destination.parent().ok_or_else(AppError::storage)?;
    let staged = tempfile::Builder::new()
        .prefix(".payload-")
        .tempdir_in(parent)
        .map_err(|_| AppError::storage())?;
    write(&snapshot, staged.path())?;
    confirm(expected, staged.path())?;
    fs::rename(staged.path(), destination).map_err(|_| AppError::storage())?;
    sync(parent)
}

/// Re-reads the source and checks that `destination` is absent and outside it.
fn verified(expected: &Inspection, destination: &Path) -> AppResult<traversal::Snapshot> {
    if expected.status == ValidationStatus::Blocked {
        return Err(AppError::validation("Blocked payloads cannot be imported."));
    }
    let snapshot = traversal::snapshot(&expected.canonical_source)?;
    if snapshot.inspection != *expected {
        return Err(AppError::stale("SourceChanged"));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| AppError::validation("A staging parent is required."))?;
    let parent = fs::canonicalize(parent).map_err(|_| AppError::storage())?;
    if parent.starts_with(&expected.canonical_source) || fs::symlink_metadata(destination).is_ok() {
        return Err(AppError::validation(
            "The staging destination must be absent and outside the source.",
        ));
    }
    Ok(snapshot)
}

/// Writes every manifest entry under `root` and syncs it.
fn write(snapshot: &traversal::Snapshot, root: &Path) -> AppResult<()> {
    for entry in &snapshot.inspection.manifest.entries {
        let target = root.join(&entry.path);
        match entry.kind {
            ManifestKind::Directory => fs::create_dir(&target).map_err(|_| AppError::storage())?,
            ManifestKind::File => {
                let bytes = snapshot
                    .files
                    .get(&entry.path)
                    .ok_or_else(AppError::storage)?;
                use std::io::Write;
                let mut file = fs::File::create(&target).map_err(|_| AppError::storage())?;
                file.write_all(bytes)
                    .and_then(|()| file.sync_all())
                    .map_err(|_| AppError::storage())?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(
                        &target,
                        fs::Permissions::from_mode(if entry.executable { 0o755 } else { 0o644 }),
                    )
                    .map_err(|_| AppError::storage())?;
                }
            }
        }
    }
    for entry in snapshot
        .inspection
        .manifest
        .entries
        .iter()
        .rev()
        .filter(|entry| entry.kind == ManifestKind::Directory)
    {
        sync(&root.join(&entry.path))?;
    }
    sync(root)
}

/// Checks that the copy at `copy` matches the inspection and the source still does.
fn confirm(expected: &Inspection, copy: &Path) -> AppResult<()> {
    if inspect(copy)?.manifest != expected.manifest
        || inspect(&expected.canonical_source)? != *expected
    {
        return Err(AppError::stale("SourceChanged"));
    }
    Ok(())
}

fn sync(directory: &Path) -> AppResult<()> {
    #[cfg(unix)]
    fs::File::open(directory)
        .and_then(|file| file.sync_all())
        .map_err(|_| AppError::storage())?;
    #[cfg(not(unix))]
    let _ = directory;
    Ok(())
}

fn finding(messages: &mut Vec<ValidationMessage>, status: ValidationStatus, message: &str) {
    messages.push(ValidationMessage {
        status,
        message: message.to_owned(),
    });
}

#[cfg(test)]
mod tests;
