use super::{AppError, AppResult, Git, REMOTE_REF, failure};
use std::{fs, path::Path};

impl Git {
    /// Whether the library is a newly initialized repository without commits or skills, which
    /// may adopt a populated remote. Presets and folders are machine-local and never block it.
    pub(super) fn can_adopt(&self, repo: &Path) -> AppResult<bool> {
        Ok(self.revision(repo, "HEAD")?.is_none()
            && self.output(repo, &["ls-files", "-z"])?.is_empty()
            && empty_payload(&repo.join("skills"))?
            && empty_payload(&repo.join("sources"))?)
    }

    pub(super) fn adopt(&self, repo: &Path) -> AppResult<()> {
        if !self.can_adopt(repo)? {
            return Err(failure(
                "Only a newly initialized empty library can adopt a remote.",
            ));
        }
        self.output(repo, &["merge", "--ff-only", "--no-edit", REMOTE_REF])
            .map(drop)
    }
}

/// Whether the managed directory `path` is absent or empty.
fn empty_payload(path: &Path) -> AppResult<bool> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Ok(metadata) if metadata.file_type().is_dir() => Ok(fs::read_dir(path)
            .map_err(|_| AppError::storage())?
            .next()
            .is_none()),
        Ok(_) => Ok(false),
        Err(_) => Err(AppError::storage()),
    }
}
