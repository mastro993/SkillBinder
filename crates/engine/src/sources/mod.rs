//! Library skills' sources: learned from lock files at install, stored as `sources/<slug>.toml`
//! beside the skill directories so they sync with them, checked for updates, and applied.

mod apply;
mod checks;
pub(crate) mod lock;

pub(crate) use checks::{Updates, check};

use crate::{git::Git, payload, payload::Inspection, persistence, worker::State};
use sha2::{Digest, Sha256};
use skillbinder_proto::{AppError, AppResult, SkillSource};
use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

impl State {
    fn sources_dir(&self) -> PathBuf {
        self.library_dir().join("sources")
    }

    fn source_file(&self, slug: &str) -> AppResult<PathBuf> {
        if !payload::portable(slug) {
            return Err(AppError::storage());
        }
        Ok(self.sources_dir().join(format!("{slug}.toml")))
    }

    /// The source stored for `slug`, if any. An unreadable or invalid file reads as no source,
    /// and the reason is logged.
    pub(crate) fn read_source(&self, slug: &str) -> Option<SkillSource> {
        let text = match fs::read_to_string(self.source_file(slug).ok()?) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
            Err(error) => {
                self.log
                    .record(format!("Source of {slug} is unreadable: {error}"));
                return None;
            }
        };
        match toml::from_str::<SkillSource>(&text)
            .map_err(|error| error.to_string())
            .and_then(|source| {
                crate::git::validate_source(&source)
                    .map(|()| source)
                    .map_err(|error| error.message)
            }) {
            Ok(source) => Some(source),
            Err(error) => {
                self.log
                    .record(format!("Source of {slug} is invalid: {error}"));
                None
            }
        }
    }

    /// Durably writes `slug`'s source file, replacing any previous one.
    pub(crate) fn write_source(&self, slug: &str, source: &SkillSource) -> AppResult<()> {
        let path = self.source_file(slug)?;
        let directory = self.sources_dir();
        fs::create_dir_all(&directory).map_err(|_| AppError::storage())?;
        let text = toml::to_string(source).map_err(|_| AppError::storage())?;
        let mut file =
            tempfile::NamedTempFile::new_in(&directory).map_err(|_| AppError::storage())?;
        file.write_all(text.as_bytes())
            .and_then(|()| file.as_file().sync_all())
            .map_err(|_| AppError::storage())?;
        file.persist(&path).map_err(|_| AppError::storage())?;
        persistence::sync_directory(&directory)
    }

    /// Removes `slug`'s source file, if any.
    pub(crate) fn remove_source(&self, slug: &str) -> AppResult<()> {
        match fs::remove_file(self.source_file(slug)?) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(AppError::storage()),
            _ => Ok(()),
        }
    }

    /// Removes the source files whose skill directory is not among `listed`, such as one an
    /// install interrupted before its skill arrived left behind.
    pub(crate) fn remove_orphan_sources(&self, listed: &BTreeSet<String>) -> AppResult<()> {
        let entries = match fs::read_dir(self.sources_dir()) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(_) => return Err(AppError::storage()),
        };
        for entry in entries {
            let name = entry.map_err(|_| AppError::storage())?.file_name();
            if let Some(slug) = name.to_str().and_then(|name| name.strip_suffix(".toml"))
                && !listed.contains(slug)
            {
                self.log
                    .record(format!("Removing the orphan source of {slug}"));
                self.remove_source(slug)?;
            }
        }
        Ok(())
    }
}

/// Who uses a source cache. Checks and updates keep separate caches, so a slow check never holds
/// up an update on the worker; each has one user at a time.
#[derive(Clone, Copy)]
enum CacheUser {
    Check,
    Update,
}

/// The cache repository of a source's repository under the application data folder.
fn cache_dir(data_dir: &Path, user: CacheUser, source: &SkillSource) -> PathBuf {
    let key = format!("{:x}", Sha256::digest(source.url.as_bytes()));
    let user = match user {
        CacheUser::Check => "check",
        CacheUser::Update => "update",
    };
    data_dir.join("source-cache").join(user).join(&key[..32])
}

/// Why fetched source content cannot replace a library skill, if it cannot.
fn refusal(fetched: &Inspection) -> Option<&'static str> {
    if fetched.status == skillbinder_proto::ValidationStatus::Blocked {
        Some("The source's current version is blocked and cannot be installed.")
    } else if fetched.manifest.file_count() == 0 {
        // Git tracks files, not directories, so a skill without files would never sync.
        Some("The source's current version has no files.")
    } else {
        None
    }
}

/// Checks out the fetched source's folder as `<into>/<slug>` and inspects it; `None` when the
/// fetched commit no longer has the folder.
fn checkout(
    git: &Git,
    cache: &Path,
    source: &SkillSource,
    into: &Path,
    slug: &str,
) -> AppResult<Option<Inspection>> {
    let Some(tree) = git.source_tree(cache, &source.path)? else {
        return Ok(None);
    };
    let directory = into.join(slug);
    fs::create_dir_all(&directory).map_err(|_| AppError::storage())?;
    git.checkout_tree(cache, &tree, &directory)?;
    payload::inspect(&directory).map(Some)
}

/// Why an update cannot happen: the source's folder is gone.
const GONE: &str = "Its source no longer has this skill.";
