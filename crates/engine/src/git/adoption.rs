use super::{AppError, AppResult, Git, REMOTE_REF, failure, validation};
use serde::{Deserialize, Serialize};
use skillbinder_proto::Library;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const JOURNAL: &str = "skillbinder-initial-adoption.json";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Adoption {
    metadata: String,
    target: String,
}

impl Git {
    /// Recovers interrupted initial adoption before the engine reads library metadata.
    /// Unknown or externally modified states are left untouched for manual recovery.
    pub fn recover(&self, repo: &Path) -> AppResult<()> {
        let path = journal_path(repo)?;
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(_) => return Err(AppError::storage()),
        };
        if !fs::symlink_metadata(&path)
            .map_err(|_| AppError::storage())?
            .file_type()
            .is_file()
        {
            return Err(failure("Initial adoption journal is not a regular file."));
        }
        let journal: Adoption = serde_json::from_slice(&bytes)
            .map_err(|_| failure("Initial adoption journal is invalid."))?;
        if !empty_metadata(journal.metadata.as_bytes())?
            || !matches!(journal.target.len(), 40 | 64)
            || !journal.target.bytes().all(|c| c.is_ascii_hexdigit())
        {
            return Err(failure("Initial adoption journal is invalid."));
        }
        match self.revision(repo, "HEAD")? {
            None => {
                if !self.output(repo, &["ls-files", "-z"])?.is_empty() || !empty_payload(repo)? {
                    return Err(failure(
                        "Interrupted adoption has unexpected local content.",
                    ));
                }
                let metadata = repo.join(".skillbinder.json");
                match fs::read(&metadata) {
                    Ok(current)
                        if current == journal.metadata.as_bytes()
                            && fs::symlink_metadata(&metadata)
                                .map_err(|_| AppError::storage())?
                                .file_type()
                                .is_file() => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        write_new(&metadata, journal.metadata.as_bytes())?;
                        sync_dir(repo)?;
                    }
                    _ => return Err(failure("Interrupted adoption metadata was modified.")),
                }
            }
            Some(head) if head == journal.target => {
                self.validate_tree(repo, &head)?;
                if !self
                    .output(
                        repo,
                        &[
                            "status",
                            "--porcelain=v1",
                            "--untracked-files=all",
                            "--",
                            ".skillbinder.json",
                            "skills",
                        ],
                    )?
                    .is_empty()
                {
                    return Err(failure("Adopted library has unexpected local changes."));
                }
            }
            Some(_) => {
                return Err(failure(
                    "Interrupted adoption has an unexpected Git revision.",
                ));
            }
        }
        fs::remove_file(path).map_err(|_| AppError::storage())?;
        sync_dir(&repo.join(".git"))
    }

    pub(super) fn can_adopt(&self, repo: &Path) -> AppResult<bool> {
        if self.revision(repo, "HEAD")?.is_some()
            || !self.output(repo, &["ls-files", "-z"])?.is_empty()
            || !empty_payload(repo)?
        {
            return Ok(false);
        }
        let metadata = repo.join(".skillbinder.json");
        if !fs::symlink_metadata(&metadata)
            .map_err(|_| AppError::storage())?
            .file_type()
            .is_file()
        {
            return Ok(false);
        }
        empty_metadata(&fs::read(metadata).map_err(|_| AppError::storage())?)
    }

    pub(super) fn adopt(&self, repo: &Path) -> AppResult<()> {
        if !self.can_adopt(repo)? {
            return Err(failure(
                "Only a newly initialized empty library can adopt a remote.",
            ));
        }
        let journal = Adoption {
            metadata: fs::read_to_string(repo.join(".skillbinder.json"))
                .map_err(|_| AppError::storage())?,
            target: self
                .revision(repo, REMOTE_REF)?
                .ok_or_else(|| failure("Remote revision is unavailable."))?,
        };
        let bytes = serde_json::to_vec(&journal).map_err(|_| AppError::storage())?;
        write_new(&journal_path(repo)?, &bytes)?;
        sync_dir(&repo.join(".git"))?;
        fs::remove_file(repo.join(".skillbinder.json")).map_err(|_| AppError::storage())?;
        sync_dir(repo)?;
        let merged = self.output(repo, &["merge", "--ff-only", "--no-edit", &journal.target]);
        self.recover(repo)?;
        merged.map(|_| ())
    }
}

fn empty_metadata(bytes: &[u8]) -> AppResult<bool> {
    validation::metadata(bytes)?;
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| AppError::storage())?;
    let Some(object) = value.as_object() else {
        return Ok(false);
    };
    if object.keys().any(|key| {
        !matches!(
            key.as_str(),
            "schemaVersion"
                | "libraryId"
                | "createdAt"
                | "contentPolicyVersion"
                | "skills"
                | "folders"
                | "tags"
        )
    }) {
        return Ok(false);
    }
    let library: Library = serde_json::from_slice(bytes).map_err(|_| AppError::storage())?;
    Ok(library.skills.is_empty() && library.folders.is_empty() && library.tags.is_empty())
}

fn empty_payload(repo: &Path) -> AppResult<bool> {
    let path = repo.join("skills");
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Ok(metadata) if metadata.file_type().is_dir() => Ok(fs::read_dir(path)
            .map_err(|_| AppError::storage())?
            .next()
            .is_none()),
        Ok(_) => Ok(false),
        Err(_) => Err(AppError::storage()),
    }
}

fn journal_path(repo: &Path) -> AppResult<PathBuf> {
    if !fs::symlink_metadata(repo.join(".git"))
        .map_err(|_| AppError::storage())?
        .file_type()
        .is_dir()
    {
        return Err(failure(
            "Initial adoption requires a dedicated library repository.",
        ));
    }
    Ok(repo.join(".git").join(JOURNAL))
}

fn write_new(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| AppError::storage())?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| AppError::storage())
}

fn sync_dir(path: &Path) -> AppResult<()> {
    #[cfg(unix)]
    fs::File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|_| AppError::storage())?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}
