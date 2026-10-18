//! Library skills' sources: learned from lock files at import, kept in each skill's record,
//! checked for updates, and applied.

mod apply;
mod checks;
pub(crate) mod lock;

pub(crate) use checks::{Updates, check};

use crate::{git::Git, payload, payload::Inspection, records::Record, worker::State};
use sha2::{Digest, Sha256};
use skillbinder_proto::{AppError, AppResult, Drift, SkillSource, ValidationStatus};
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
            // ponytail: reads the locks once per skill; learning happens once per skill.
            let Some(source) = copies
                .iter()
                .filter(|(id, copy)| {
                    id == &skill.id
                        && copy.inspection.status != ValidationStatus::Blocked
                        && copy.inspection.tree == skill.recorded
                })
                .find_map(|(_, copy)| lock::source(&copy.locks, &copy.inspection.slug))
            else {
                continue;
            };
            let slug = skill.id.as_str();
            let previous = self.read_record(slug);
            let record = Record {
                tree: skill.recorded.clone(),
                source: Some(source.clone()),
            };
            self.write_record(slug, &record)?;
            let cached =
                crate::library_store::set_source(&mut self.database.borrow_mut(), slug, &source);
            if let Err(error) = cached {
                // The cache would never learn the file's source again, so the file goes back.
                self.restore_record(slug, previous.as_ref());
                return Err(error);
            }
            if skill.drift == Some(Drift::Lost) {
                // The source may still publish the recorded content.
                crate::library_store::set_drift(
                    &mut self.database.borrow_mut(),
                    slug,
                    Some(Drift::Restorable),
                )?;
            }
            self.log.record(format!("Learned the source of {slug}"));
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

impl State {
    /// Removes the source caches no library skill's source uses any more, such as those of
    /// skills a Pull removed. Run before any check or update starts.
    pub(crate) fn prune_source_caches(&self) -> AppResult<()> {
        let used: std::collections::BTreeSet<_> = self
            .read_library()?
            .skills
            .iter()
            .filter_map(|skill| skill.source.as_ref())
            .flat_map(|source| {
                [CacheUser::Check, CacheUser::Update]
                    .map(|user| cache_dir(&self.config.data_dir, user, source))
            })
            .collect();
        for user in ["check", "update"] {
            let Ok(entries) = fs::read_dir(self.config.data_dir.join("source-cache").join(user))
            else {
                continue;
            };
            for path in entries.flatten().map(|entry| entry.path()) {
                if !used.contains(&path) {
                    self.log.record(format!(
                        "Removing an unused source cache {}",
                        path.display()
                    ));
                    fs::remove_dir_all(&path).map_err(|_| AppError::storage())?;
                }
            }
        }
        Ok(())
    }
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
    // Bounded before any file is fetched or written; inspection bounds the bytes afterwards.
    let modes = git.tree_modes(cache, &tree)?;
    if modes.len() > payload::MAX_ENTRIES {
        return Err(AppError::validation(
            "The source's folder has too many files to check.",
        ));
    }
    // A submodule would check out as an empty folder, so its content could never match.
    if modes.iter().any(|mode| mode == "160000") {
        return Err(AppError::validation(
            "The source's folder holds a Git submodule, which SkillBinder cannot check out.",
        ));
    }
    let directory = into.join(slug);
    fs::create_dir_all(&directory).map_err(|_| AppError::storage())?;
    git.checkout_tree(cache, &tree, &directory)?;
    payload::inspect(&directory).map(Some)
}

/// Why an update cannot happen: the source's folder is gone.
const GONE: &str = "Its source no longer has this skill.";

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn caches_no_skill_uses_are_pruned() {
        let directory = tempfile::tempdir().unwrap();
        let state = State::isolated_test_state(fs::canonicalize(directory.path()).unwrap());
        let unused = state.config.data_dir.join("source-cache/check/unused");
        fs::create_dir_all(unused.join("objects")).unwrap();
        state.prune_source_caches().unwrap();
        assert!(!unused.exists());
    }
}
