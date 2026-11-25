//! Library skills' sources: learned from lock files at import, kept in each skill's record,
//! checked for updates, and applied.

mod apply;
mod checks;
pub(crate) mod lock;

pub(crate) use checks::{Updates, check};

use crate::logging::{Area, Level};
use crate::{git::Git, payload, payload::Inspection, worker::State};
use sha2::{Digest, Sha256};
use skillbinder_proto::{
    AppError, AppResult, Drift, SkillSource, ValidationStatus, normalized_url,
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

impl State {
    /// Gives each library skill without a source the one a `skills` lock file records for a found
    /// copy with exactly its recorded content, as importing that copy would have, so skills
    /// imported before sources were kept, or that arrived by Pull, can update. A learned source
    /// is never dropped when the copy goes away. Only the database learns it here: the record
    /// file follows at the next Push or Sync, so learning never dirties the library before a
    /// Pull.
    pub(crate) fn learn_sources(&self) -> AppResult<()> {
        let copies = self.scans.library_copies()?;
        let library = self.read_library()?;
        // Each lock file is read once per learning pass.
        let mut locks: BTreeMap<PathBuf, Option<lock::Lock>> = BTreeMap::new();
        for skill in library.skills.iter().filter(|skill| skill.source.is_none()) {
            let Some(source) = copies
                .iter()
                .filter(|(id, copy)| {
                    id == &skill.id
                        && copy.inspection.status != ValidationStatus::Blocked
                        && copy.inspection.tree == skill.recorded
                })
                .find_map(|(_, copy)| {
                    copy.locks.iter().find_map(|path| {
                        locks
                            .entry(path.clone())
                            .or_insert_with(|| lock::read(path))
                            .as_ref()?
                            .source(&copy.inspection.slug)
                    })
                })
            else {
                continue;
            };
            let slug = skill.id.as_str();
            crate::library_store::set_source(&mut self.database.borrow_mut(), slug, &source)?;
            if skill.drift == Some(Drift::Lost) {
                // The source may still publish the recorded content.
                crate::library_store::set_drift(
                    &mut self.database.borrow_mut(),
                    slug,
                    Some(Drift::Restorable),
                )?;
            }
            self.log
                .event(Level::Info, Area::Library, "source learned")
                .field("slug", slug);
        }
        Ok(())
    }
}

/// Who uses a source cache. Checks and updates keep separate caches, so a slow check never holds
/// up an update on the worker; each has one user at a time. Previews share one cache per
/// repository, taking its lock in turn.
#[derive(Clone, Copy)]
pub(crate) enum CacheUser {
    Check,
    Update,
    Catalog,
}

impl State {
    /// Removes the source caches no library skill's source uses any more, such as those of
    /// skills a Pull removed, the install caches no running install uses, and the catalog
    /// caches no saved repository and no open or running preview or listing uses. Run before
    /// any check or update starts, and once a removed repository's Undo is no longer offered.
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
            .chain(
                self.installs
                    .running()?
                    .into_iter()
                    .map(|id| install_cache(&self.config.data_dir, id)),
            )
            .chain(
                self.previews
                    .open_source()
                    .map(|source| cache_dir(&self.config.data_dir, CacheUser::Catalog, &source)),
            )
            .chain(self.repositories.current().0.iter().map(|repository| {
                let source = SkillSource {
                    url: repository.url.clone(),
                    reference: None,
                    path: String::new(),
                };
                cache_dir(&self.config.data_dir, CacheUser::Catalog, &source)
            }))
            .collect();
        for user in ["check", "update", "install", "catalog"] {
            let Ok(entries) = fs::read_dir(self.config.data_dir.join("source-cache").join(user))
            else {
                continue;
            };
            for path in entries.flatten().map(|entry| entry.path()) {
                if used.contains(&path) {
                    continue;
                }
                // A preview or listing reading a catalog cache holds its lock; holding it here
                // keeps the next one from starting until the cache is gone.
                let lock = (user == "catalog").then(|| self.previews.cache_lock(&path));
                let reading = lock.as_ref().map(|lock| lock.try_lock());
                if matches!(reading, Some(Err(_))) {
                    continue;
                }
                self.log
                    .event(Level::Debug, Area::Update, "source cache removed")
                    .path(&path);
                fs::remove_dir_all(&path).map_err(|_| AppError::storage())?;
            }
        }
        Ok(())
    }
}

/// The cache repository of a source's repository under the application data folder. Previews
/// key it by the normalized URL, so two spellings of one repository share it.
pub(crate) fn cache_dir(data_dir: &Path, user: CacheUser, source: &SkillSource) -> PathBuf {
    let (user, url) = match user {
        CacheUser::Check => ("check", source.url.clone()),
        CacheUser::Update => ("update", source.url.clone()),
        CacheUser::Catalog => ("catalog", normalized_url(&source.url)),
    };
    let key = format!("{:x}", Sha256::digest(url.as_bytes()));
    data_dir.join("source-cache").join(user).join(&key[..32])
}

/// The cache repository of the install `id`. Each install has its own, since installs from one
/// repository run at once and a cache has one user at a time.
pub(crate) fn install_cache(data_dir: &Path, id: uuid::Uuid) -> PathBuf {
    data_dir
        .join("source-cache")
        .join("install")
        .join(id.simple().to_string())
}

/// Why fetched source content cannot be put in the library. Update and install each say it in
/// their own words.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Refusal {
    Blocked,
    /// Importing it would need an acknowledgement, which neither update nor install asks for.
    Invalid,
    /// Git tracks files, not directories, so a skill without files would never sync.
    Empty,
}

impl Refusal {
    /// Why an update cannot replace a library skill with the fetched content.
    fn update_text(self) -> &'static str {
        match self {
            Self::Blocked => "The source's current version is blocked and cannot be imported.",
            Self::Invalid => "The source's current version has invalid skill metadata.",
            Self::Empty => "The source's current version has no files.",
        }
    }
}

/// Why fetched source content cannot be put in the library, if it cannot.
pub(crate) fn refusal(fetched: &Inspection) -> Option<Refusal> {
    if fetched.status == ValidationStatus::Blocked {
        Some(Refusal::Blocked)
    } else if fetched.status == ValidationStatus::Invalid {
        Some(Refusal::Invalid)
    } else if fetched.manifest.file_count() == 0 {
        Some(Refusal::Empty)
    } else {
        None
    }
}

/// Checks out the fetched source's folder as `<into>/<slug>` and inspects it; `None` when the
/// fetched commit no longer has the folder.
pub(crate) fn checkout(
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
    let entries = git.tree_entries(cache, &tree)?;
    if entries.len() > payload::MAX_ENTRIES {
        return Err(AppError::validation(
            "The source's folder has too many files to check.",
        ));
    }
    // A submodule would check out as an empty folder, so its content could never match.
    if entries.iter().any(|entry| entry.mode == "160000") {
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
        let state = State::isolated_test_state(dunce::canonicalize(directory.path()).unwrap());
        let unused = state.config.data_dir.join("source-cache/check/unused");
        fs::create_dir_all(unused.join("objects")).unwrap();
        state.prune_source_caches().unwrap();
        assert!(!unused.exists());
    }

    #[test]
    fn a_catalog_cache_is_kept_while_its_preview_is_open_or_reading() {
        let directory = tempfile::tempdir().unwrap();
        let state = State::isolated_test_state(dunce::canonicalize(directory.path()).unwrap());
        let skill = skillbinder_proto::CatalogSkill {
            slug: "pdf".into(),
            key: "pdf".into(),
            name: "pdf".into(),
            description: String::new(),
            origin: skillbinder_proto::CatalogOrigin::Curated {
                creator: "acme".into(),
            },
            source: SkillSource {
                url: "https://github.com/Acme/Skills.git".into(),
                reference: None,
                path: "skills/pdf".into(),
            },
            installs_week: 0,
        };
        // Two spellings of one repository share its cache.
        let spelled = SkillSource {
            url: "https://github.com/acme/skills".into(),
            ..skill.source.clone()
        };
        let data = &state.config.data_dir;
        let open = cache_dir(data, CacheUser::Catalog, &spelled);
        let reading = data.join("source-cache/catalog/reading");
        for cache in [&open, &reading] {
            fs::create_dir_all(cache.join("objects")).unwrap();
        }
        state.previews.open(&skill);
        let lock = state.previews.cache_lock(&reading);
        let held = lock.lock().unwrap();
        state.prune_source_caches().unwrap();
        assert!(open.exists() && reading.exists());

        drop(held);
        state.previews.close();
        state.prune_source_caches().unwrap();
        assert!(!open.exists() && !reading.exists());
    }
}
