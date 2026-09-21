use skillbinder_core::source::{EntryKind, EntryMetadata, PayloadSource, SourceError};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Default)]
pub struct FilesystemPayloadSource;
impl PayloadSource for FilesystemPayloadSource {
    fn list_entries(&self, path: &Path) -> Result<Vec<PathBuf>, SourceError> {
        fs::read_dir(path)
            .map_err(map_error)?
            .map(|entry| entry.map(|e| e.path()).map_err(map_error))
            .collect()
    }
    fn list_entries_limited(&self, path: &Path, cap: usize) -> Result<Vec<PathBuf>, SourceError> {
        let mut entries = Vec::new();
        for entry in fs::read_dir(path).map_err(map_error)? {
            if entries.len() >= cap {
                break;
            }
            entries.push(entry.map_err(map_error)?.path());
        }
        Ok(entries)
    }
    fn entry_metadata(&self, path: &Path) -> Result<EntryMetadata, SourceError> {
        let metadata = fs::symlink_metadata(path).map_err(map_error)?;
        let kind = if metadata.file_type().is_symlink() {
            EntryKind::Symlink
        } else if metadata.is_file() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if metadata.nlink() > 1 {
                    EntryKind::Hardlink
                } else {
                    EntryKind::File
                }
            }
            #[cfg(not(unix))]
            {
                EntryKind::File
            }
        } else if metadata.is_dir() {
            EntryKind::Directory
        } else {
            EntryKind::Device
        };
        #[cfg(unix)]
        let executable = {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode() & 0o111 != 0
        };
        #[cfg(not(unix))]
        let executable = false;
        Ok(EntryMetadata {
            kind,
            executable,
            size: metadata.len(),
        })
    }
    fn read_file(&self, path: &Path, cap: usize) -> Result<Vec<u8>, SourceError> {
        let mut file = fs::File::open(path).map_err(|e| SourceError::Unavailable(e.to_string()))?;
        let mut bytes = Vec::new();
        file.by_ref()
            .take((cap as u64) + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| SourceError::Unavailable(e.to_string()))?;
        if bytes.len() > cap {
            return Err(SourceError::Limit);
        }
        Ok(bytes)
    }
    fn resolve_symlink(&self, path: &Path, max_hops: u8) -> Result<PathBuf, SourceError> {
        let mut current = path.to_path_buf();
        for _ in 0..=max_hops {
            let metadata = fs::symlink_metadata(&current)
                .map_err(|e| SourceError::Unavailable(e.to_string()))?;
            if !metadata.file_type().is_symlink() {
                return current
                    .canonicalize()
                    .map_err(|e| SourceError::Unavailable(e.to_string()));
            }
            let target =
                fs::read_link(&current).map_err(|e| SourceError::Unavailable(e.to_string()))?;
            current = if target.is_absolute() {
                target
            } else {
                current.parent().unwrap_or(Path::new("/")).join(target)
            };
        }
        Err(SourceError::Unavailable(
            "symlink hop limit or cycle".into(),
        ))
    }
    fn physical_identity(&self, path: &Path) -> Result<String, SourceError> {
        let metadata = fs::symlink_metadata(path).map_err(map_error)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Ok(format!("{}:{}", metadata.dev(), metadata.ino()))
        }
        #[cfg(not(unix))]
        {
            let modified = metadata
                .modified()
                .map_err(|error| SourceError::Unavailable(error.to_string()))?;
            Ok(format!(
                "{}:{}:{:?}",
                self.canonicalize_root(path)?.display(),
                metadata.len(),
                modified
            ))
        }
    }
    fn canonicalize_root(&self, path: &Path) -> Result<PathBuf, SourceError> {
        path.canonicalize()
            .map_err(|error| SourceError::Unavailable(error.to_string()))
    }
    fn volume_id(&self, path: &Path) -> Option<u64> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            fs::symlink_metadata(path)
                .ok()
                .map(|metadata| metadata.dev())
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            None
        }
    }
}
fn map_error(error: std::io::Error) -> SourceError {
    if error.kind() == std::io::ErrorKind::NotFound {
        SourceError::Missing
    } else if error.kind() == std::io::ErrorKind::PermissionDenied {
        SourceError::Unreadable(error.to_string())
    } else {
        SourceError::Unavailable(error.to_string())
    }
}
