//! A catalog skill's preview for its detail dialog (design T7): its repository fetched blobless
//! into `<data>/source-cache/catalog/<sha>` on the Git pool, never the worker, its folder listed
//! from Git's trees alone, and only its SKILL.md blob read. One dialog is open at a time.

use crate::{
    git::Git,
    logging::{Area, Level, Logger},
    payload,
};
use skillbinder_proto::{
    AppError, AppResult, CatalogPreview, CatalogSkill, Preview, PreviewState, SkillSource,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};

/// The most of a SKILL.md the dialog shows.
const SKILL_MD_LIMIT: usize = 256 * 1024;

#[derive(Default)]
struct Shared {
    /// The open dialog's preview and the request loading it.
    open: Option<(u64, Preview)>,
    requests: u64,
}

/// The open dialog's preview, shared by the worker's snapshots, the client, and the pool.
#[derive(Clone, Default)]
pub(crate) struct Previews {
    shared: Arc<Mutex<Shared>>,
    /// One lock per catalog cache: a preview holds it while it reads the cache, and pruning
    /// while it removes it, so neither runs into the other or into a second preview.
    caches: Arc<Mutex<BTreeMap<PathBuf, Arc<Mutex<()>>>>>,
}

/// A load of the open preview.
#[derive(Clone)]
pub(crate) struct Request {
    id: u64,
    /// The skill's source, its folder without a trailing `/`.
    pub(crate) source: SkillSource,
}

impl Previews {
    fn lock(&self) -> MutexGuard<'_, Shared> {
        // Nothing panics while holding it, so a poisoned lock still holds consistent data.
        self.shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The open dialog's preview, if one is open.
    pub(crate) fn current(&self) -> Option<Preview> {
        self.lock()
            .open
            .as_ref()
            .map(|(_, preview)| preview.clone())
    }

    /// Opens `skill`'s preview, loading, in place of any other.
    pub(crate) fn open(&self, skill: &CatalogSkill) -> Request {
        let mut shared = self.lock();
        shared.requests += 1;
        let request = Request {
            id: shared.requests,
            source: SkillSource {
                url: skill.source.url.clone(),
                reference: None,
                path: skill.source.path.trim_matches('/').to_owned(),
            },
        };
        let preview = Preview {
            source: skill.source.clone(),
            state: PreviewState::Loading,
        };
        shared.open = Some((request.id, preview));
        request
    }

    /// Loads the open preview again, as Retry does; `None` when no dialog is open.
    pub(crate) fn reopen(&self) -> Option<Request> {
        let mut shared = self.lock();
        shared.requests += 1;
        let id = shared.requests;
        let (request, preview) = shared.open.as_mut()?;
        *request = id;
        preview.state = PreviewState::Loading;
        Some(Request {
            id,
            source: SkillSource {
                path: preview.source.path.trim_matches('/').to_owned(),
                reference: None,
                ..preview.source.clone()
            },
        })
    }

    /// Closes the dialog; a load still running ends unseen.
    pub(crate) fn close(&self) {
        self.lock().open = None;
    }

    /// Whether `request` still loads the open preview.
    fn wanted(&self, request: u64) -> bool {
        self.lock()
            .open
            .as_ref()
            .is_some_and(|(id, _)| *id == request)
    }

    /// Ends `request` with `state`, unless the dialog closed or loads something else since.
    pub(crate) fn finish(&self, request: &Request, state: PreviewState) {
        if let Some((id, preview)) = &mut self.lock().open
            && *id == request.id
        {
            preview.state = state;
        }
    }

    /// The source of the open dialog's skill, whose cache pruning keeps.
    pub(crate) fn open_source(&self) -> Option<SkillSource> {
        self.lock()
            .open
            .as_ref()
            .map(|(_, preview)| preview.source.clone())
    }

    /// The lock of the catalog cache `cache`.
    pub(crate) fn cache_lock(&self, cache: &Path) -> Arc<Mutex<()>> {
        let mut caches = self
            .caches
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        caches.entry(cache.to_path_buf()).or_default().clone()
    }
}

/// A preview's work on the Git pool.
pub(crate) struct Read {
    pub git: Git,
    pub cache: PathBuf,
    pub previews: Previews,
    pub log: Logger,
}

impl Read {
    /// Reads `request`'s preview, unless the dialog moved on while it waited for a thread or
    /// for the cache.
    pub(crate) fn run(self, request: &Request) -> PreviewState {
        if !self.previews.wanted(request.id) {
            return PreviewState::Failed;
        }
        let lock = self.previews.cache_lock(&self.cache);
        let _reading = lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Another preview of this repository may have held the lock through a long fetch.
        if !self.previews.wanted(request.id) {
            return PreviewState::Failed;
        }
        match self.read(&request.source) {
            Ok(Some(preview)) => PreviewState::Loaded(Arc::new(preview)),
            Ok(None) => PreviewState::NotFound,
            Err(error) => {
                self.log
                    .failure(
                        Level::Warn,
                        Area::Explore,
                        ("preview failed", "preview stopped at quit"),
                        &error,
                    )
                    .field("url", &request.source.url)
                    .field("path", &request.source.path);
                PreviewState::Failed
            }
        }
    }

    /// The folder's files and SKILL.md; `None` when the fetched commit has no such folder or
    /// it has no SKILL.md.
    fn read(&self, source: &SkillSource) -> AppResult<Option<CatalogPreview>> {
        self.git.fetch_source(&self.cache, source)?;
        let Some(tree) = self.git.source_tree(&self.cache, &source.path)? else {
            return Ok(None);
        };
        let entries = self.git.tree_entries(&self.cache, &tree)?;
        if entries.len() > payload::MAX_ENTRIES {
            return Err(AppError::validation(
                "The source's folder has too many files to list.",
            ));
        }
        // A symbolic link's blob holds its target, not the file.
        let Some(skill_md) = entries
            .iter()
            .find(|entry| entry.path == "SKILL.md" && entry.mode.starts_with("1006"))
        else {
            return Ok(None);
        };
        let mut text = self.git.read_blob(&self.cache, &skill_md.object)?;
        if text.len() > SKILL_MD_LIMIT {
            text.truncate(text.floor_char_boundary(SKILL_MD_LIMIT));
        }
        let files = entries
            .into_iter()
            .filter(|entry| entry.kind != "tree")
            .map(|entry| entry.path)
            .collect();
        Ok(Some(CatalogPreview {
            files,
            front_matter: payload::front_matter_fields(text.as_bytes()),
            skill_md: text,
        }))
    }
}
