//! The filesystem port a scan and a payload inspection read through.

use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
    Device,
    Hardlink,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryMetadata {
    pub kind: EntryKind,
    pub executable: bool,
    pub size: u64,
}

pub trait PayloadSource: Send + Sync {
    fn list_entries(&self, path: &Path) -> Result<Vec<PathBuf>, SourceError>;
    fn list_entries_limited(&self, path: &Path, cap: usize) -> Result<Vec<PathBuf>, SourceError> {
        let mut entries = self.list_entries(path)?;
        if entries.len() > cap {
            entries.truncate(cap);
        }
        Ok(entries)
    }
    fn physical_identity(&self, path: &Path) -> Result<String, SourceError> {
        self.canonicalize_root(path)
            .map(|value| value.display().to_string())
    }
    fn entry_metadata(&self, path: &Path) -> Result<EntryMetadata, SourceError>;
    fn read_file(&self, path: &Path, cap: usize) -> Result<Vec<u8>, SourceError>;
    fn resolve_symlink(&self, path: &Path, max_hops: u8) -> Result<PathBuf, SourceError>;
    fn canonicalize_root(&self, path: &Path) -> Result<PathBuf, SourceError> {
        Ok(path.to_path_buf())
    }
    fn volume_id(&self, _path: &Path) -> Option<u64> {
        None
    }
    fn exists(&self, path: &Path) -> bool {
        self.entry_metadata(path).is_ok()
    }
}
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SourceError {
    #[error("source is missing")]
    Missing,
    #[error("source is unreadable: {0}")]
    Unreadable(String),
    #[error("source unavailable: {0}")]
    Unavailable(String),
    #[error("source limit exceeded")]
    Limit,
}
