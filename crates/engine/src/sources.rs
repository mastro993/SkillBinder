//! Library skills' sources: learned from lock files at import, kept in each skill's record,
//! checked for updates, and applied.

mod apply;
mod checks;
pub(crate) mod lock;

pub(crate) use checks::{Updates, check};

use crate::{git::Git, payload, payload::Inspection, records::Record, worker::State};
use sha2::{Digest, Sha256};
use skillbinder_proto::{AppError, AppResult, SkillSource};
use std::{
    fs,
    path::{Path, PathBuf},
};

impl State {
    /// Gives each library skill without a source the one a `skills` lock file records for a found
    /// copy with exactly its recorded content, as importing that copy would have, so skills
    /// imported before sources were kept, or that arrived by Pull, can update. A learned source
    /// is never dropped when the copy goes away.
    pub(crate) fn learn_sources(&self) -> AppResult<()> {
        let copies = self.scans.library_copies()?;
        let library = self.read_library()?;
        for skill in library.skills.iter().filter(|skill| skill.source.is_none()) {
            // ponytail: reads the lock once per skill; learning happens once per skill.
            let Some(source) = copies
                .iter()
                .filter(|(id, copy)| id == &skill.id && copy.inspection.tree == skill.recorded)
                .find_map(|(_, copy)| lock::source(&copy.lock, &copy.inspection.slug))
            else {
                continue;
            };
            let record = Record {
                tree: skill.recorded.clone(),
                source: Some(source.clone()),
            };
            self.write_record(skill.id.as_str(), &record)?;
            crate::library_store::set_source(
                &mut self.database.borrow_mut(),
                skill.id.as_str(),
                &source,
            )?;
            self.log
                .record(format!("Learned the source of {}", skill.id.as_str()));
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
        Some("The source's current version is blocked and cannot be imported.")
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
