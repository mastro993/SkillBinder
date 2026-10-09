//! Bounded read-only payload inspection and verified link-free materialization.
mod frontmatter;
mod manifest;
mod traversal;
pub use manifest::{Manifest, ManifestEntry, ManifestKind};

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
    let staged = tempfile::Builder::new()
        .prefix(".payload-")
        .tempdir_in(parent)
        .map_err(|_| AppError::storage())?;
    for entry in &snapshot.inspection.manifest.entries {
        let target = staged.path().join(&entry.path);
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
    #[cfg(unix)]
    {
        for entry in snapshot
            .inspection
            .manifest
            .entries
            .iter()
            .rev()
            .filter(|entry| entry.kind == ManifestKind::Directory)
        {
            fs::File::open(staged.path().join(&entry.path))
                .and_then(|file| file.sync_all())
                .map_err(|_| AppError::storage())?;
        }
        fs::File::open(staged.path())
            .and_then(|file| file.sync_all())
            .map_err(|_| AppError::storage())?;
    }
    let staged_manifest = inspect(staged.path())?.manifest;
    if staged_manifest != expected.manifest || inspect(&expected.canonical_source)? != *expected {
        return Err(AppError::stale("SourceChanged"));
    }
    fs::rename(staged.path(), destination).map_err(|_| AppError::storage())?;
    #[cfg(unix)]
    fs::File::open(destination.parent().ok_or_else(AppError::storage)?)
        .and_then(|file| file.sync_all())
        .map_err(|_| AppError::storage())?;
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
