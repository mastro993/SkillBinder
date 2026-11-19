//! Finding a skills.sh result's folder (design T7): its repository fetched blobless into the
//! catalog cache on the Git pool, never the worker, its skill folders found by T1's rules from
//! Git's trees alone, and the folder named `skillId` taken, else the one whose SKILL.md name is
//! `skillId`, reading only the SKILL.md blobs it needs. A result resolves once per session.

use super::{Previews, folders::skill_folders};
use crate::{git::Git, library::normalized, logging::Logger, payload};
use skillbinder_proto::{
    AppError, AppResult, CatalogOrigin, CatalogSkill, Resolution, SkillSource, SkillsShResult,
    owner_repo, result_key,
};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};
use tokio::sync::oneshot;

/// The longest description kept, in characters, as the catalog keeps it.
const DESCRIPTION: usize = 1024;

/// What resolving a result found: the skill, `None` when no folder matches, or why the
/// repository could not be read.
pub(crate) type Resolved = AppResult<Option<CatalogSkill>>;

/// How a caller takes part in a result's resolution.
pub(crate) enum Begin {
    /// Nothing resolves it: the caller does, and finishes it.
    Start,
    /// Another caller resolves it; the receiver answers with what it found.
    Wait(oneshot::Receiver<Resolved>),
    /// It is resolved, or known to have no folder.
    Known(Resolved),
}

#[derive(Default)]
struct Shared {
    /// Each result's resolution, by [`result_key`].
    states: BTreeMap<String, Resolution>,
    /// Callers waiting for a running resolution, by [`result_key`].
    waiting: BTreeMap<String, Vec<oneshot::Sender<Resolved>>>,
}

/// The resolutions shared by the worker's snapshots, the client, and the pool.
#[derive(Clone, Default)]
pub(crate) struct Resolutions {
    shared: Arc<Mutex<Shared>>,
}

impl Resolutions {
    fn lock(&self) -> MutexGuard<'_, Shared> {
        // Nothing panics while holding it, so a poisoned lock still holds consistent data.
        self.shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Each result's resolution.
    pub(crate) fn current(&self) -> BTreeMap<String, Resolution> {
        self.lock().states.clone()
    }

    /// Joins `result`'s resolution, marking it resolving when the caller is to start it: when it
    /// was never resolved or the last attempt failed.
    pub(crate) fn begin(&self, result: &SkillsShResult) -> Begin {
        let key = result_key(result);
        let mut shared = self.lock();
        match shared.states.get(&key) {
            Some(Resolution::Resolved(skill)) => Begin::Known(Ok(Some(skill.clone()))),
            Some(Resolution::NotFound) => Begin::Known(Ok(None)),
            Some(Resolution::Resolving) => {
                let (done, found) = oneshot::channel();
                shared.waiting.entry(key).or_default().push(done);
                Begin::Wait(found)
            }
            Some(Resolution::Failed) | None => {
                shared.states.insert(key, Resolution::Resolving);
                Begin::Start
            }
        }
    }

    /// Ends `result`'s resolution with `resolved`, answering every caller waiting for it.
    pub(crate) fn finish(&self, result: &SkillsShResult, resolved: &Resolved) {
        let key = result_key(result);
        let state = match resolved {
            Ok(Some(skill)) => Resolution::Resolved(skill.clone()),
            Ok(None) => Resolution::NotFound,
            Err(_) => Resolution::Failed,
        };
        let mut shared = self.lock();
        shared.states.insert(key.clone(), state);
        for waiting in shared.waiting.remove(&key).unwrap_or_default() {
            let _ = waiting.send(resolved.clone());
        }
    }
}

/// A resolution's work on the Git pool.
pub(crate) struct Resolve {
    pub git: Git,
    /// The repository's catalog cache, which its preview shares.
    pub cache: PathBuf,
    pub previews: Previews,
    /// The registry's workspace skill directories, which T1's rules search.
    pub agent_dirs: Vec<String>,
    pub log: Logger,
}

impl Resolve {
    pub(crate) fn run(self, result: &SkillsShResult) -> Resolved {
        // Pruning and previews of this repository hold the cache's lock while they use it.
        let lock = self.previews.cache_lock(&self.cache);
        let _reading = lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let found = self.find(result);
        if let Err(error) = &found {
            self.log.record(format!(
                "Resolving {} in {} failed: {}",
                result.skill_id,
                owner_repo(&result.url),
                error.message
            ));
        }
        found
    }

    fn find(&self, result: &SkillsShResult) -> Resolved {
        let root = SkillSource {
            url: result.url.clone(),
            reference: None,
            path: String::new(),
        };
        self.git.fetch_source(&self.cache, &root)?;
        let Some(tree) = self.git.source_tree(&self.cache, "")? else {
            return Ok(None);
        };
        let entries = self.git.tree_entries(&self.cache, &tree)?;
        let files: Vec<&str> = entries
            .iter()
            .filter(|entry| entry.kind == "blob")
            .map(|entry| entry.path.as_str())
            .collect();
        let folders = skill_folders(&files, &self.agent_dirs);
        let repository = owner_repo(&result.url);
        // A root skill is named after its repository.
        let leaf = |folder: &str| {
            folder
                .rsplit('/')
                .next()
                .filter(|leaf| !leaf.is_empty())
                .unwrap_or_else(|| repository.rsplit('/').next().unwrap_or_default())
                .to_owned()
        };
        // A folder named `skillId` needs no blob; the others are read only when none is.
        let named = folders
            .iter()
            .copied()
            .find(|folder| leaf(folder) == result.skill_id);
        let candidates = named.into_iter().chain(
            folders
                .iter()
                .copied()
                .filter(|folder| Some(*folder) != named),
        );
        for folder in candidates {
            let path = if folder.is_empty() {
                "SKILL.md".to_owned()
            } else {
                format!("{folder}/SKILL.md")
            };
            // A symbolic link's blob holds its target, not the file.
            let Some(skill_md) = entries
                .iter()
                .find(|entry| entry.path == path && entry.mode.starts_with("1006"))
            else {
                continue;
            };
            let text = self.git.read_blob(&self.cache, &skill_md.object)?;
            let (name, description) =
                payload::name_and_description(text.as_bytes()).unwrap_or_default();
            if Some(folder) == named || name.as_deref() == Some(result.skill_id.as_str()) {
                return Ok(Some(skill(
                    result,
                    folder,
                    &leaf(folder),
                    name,
                    description,
                )));
            }
        }
        Ok(None)
    }
}

/// The skill `result` names, found in `folder`: its slug is its SKILL.md name, else its folder's
/// name, else its `skillId`, whichever is first a portable directory name.
fn skill(
    result: &SkillsShResult,
    folder: &str,
    leaf: &str,
    name: Option<String>,
    description: Option<String>,
) -> CatalogSkill {
    let slug = [name.as_deref(), Some(leaf)]
        .into_iter()
        .flatten()
        .find(|slug| payload::portable(slug))
        .unwrap_or(&result.skill_id)
        .to_owned();
    CatalogSkill {
        key: normalized(&slug),
        slug,
        name: result.name.clone(),
        description: description
            .unwrap_or_default()
            .chars()
            .take(DESCRIPTION)
            .collect(),
        origin: CatalogOrigin::SkillsSh {
            installs: result.installs,
        },
        source: SkillSource {
            url: result.url.clone(),
            reference: None,
            path: folder.to_owned(),
        },
        installs_week: 0,
    }
}

/// The error a resolution that found no folder gives an install of the result.
pub(crate) fn not_found(result: &SkillsShResult) -> AppError {
    AppError::validation(format!(
        "Couldn't find this skill in {}",
        owner_repo(&result.url)
    ))
}
