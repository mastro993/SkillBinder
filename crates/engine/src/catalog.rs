//! Explore's catalog: fetched from SkillBinder's CDN on a thread of its own, never the worker,
//! translated from the v1 format, and cached under `<data>/catalog/` (ADR 0014). Every failure
//! keeps the cache and maps to one [`CatalogState`]. Its search also asks skills.sh, and a
//! skills.sh result's folder is resolved before it is previewed or installed.

mod cache;
mod fetch;
mod folders;
mod preview;
mod repositories;
mod resolve;
mod search;
mod snapshot;

pub use fetch::CatalogSource;
pub(crate) use fetch::{Answer, Fetcher};
pub(crate) use preview::{Previews, Read, Request};
pub(crate) use repositories::{Listed, Listing, Repositories};
pub(crate) use resolve::{Begin, Resolutions, Resolve, Resolved, not_found};
pub(crate) use search::Searches;

use crate::{persistence, shutdown::Shutdown};
use cache::{Cache, Meta};
use skillbinder_proto::{AppError, AppResult, Catalog, CatalogState, ExploreSnapshot};
use snapshot::{Refusal, Translated, translate};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};
use tokio::sync::oneshot;

/// The CDN's base URL, compiled in (ADR 0014).
pub(crate) const BASE_URL: &str = "https://mastro993.github.io/SkillBinder/";
const CATALOG: &str = "v1/catalog.json";
/// The largest catalog file read; about 0.5 MB is expected for 2,000 skills.
const CATALOG_LIMIT: u64 = 8 * 1024 * 1024;
const AVATAR_LIMIT: u64 = 256 * 1024;

#[derive(Default)]
struct Shared {
    state: CatalogState,
    catalog: Option<Arc<Catalog>>,
    /// Unix seconds of the last answer that returned or confirmed the catalog.
    fetched_at: u64,
    running: bool,
}

/// The catalog shared by the worker's snapshots, the client, and the refresh thread.
#[derive(Clone)]
pub(crate) struct Catalogs {
    shared: Arc<Mutex<Shared>>,
    source: CatalogSource,
    data_dir: PathBuf,
    shutdown: Shutdown,
}

impl Catalogs {
    /// Starts from the cached catalog, if any, in the Loading state until a refresh ends.
    pub(crate) fn open(source: CatalogSource, data_dir: &Path, shutdown: Shutdown) -> Self {
        let cache = Cache::new(data_dir);
        let shared = match cache.load() {
            Some((translated, meta)) => Shared {
                catalog: Some(Arc::new(with_avatars(translated, &cache))),
                fetched_at: meta.fetched_at,
                ..Shared::default()
            },
            None => Shared::default(),
        };
        Self {
            shared: Arc::new(Mutex::new(shared)),
            source,
            data_dir: data_dir.to_path_buf(),
            shutdown,
        }
    }

    fn lock(&self) -> AppResult<MutexGuard<'_, Shared>> {
        self.shared.lock().map_err(|_| AppError::storage())
    }

    /// The catalog state and the cached catalog, with no installs or preview.
    pub(crate) fn current(&self) -> ExploreSnapshot {
        self.lock()
            .map(|shared| ExploreSnapshot {
                state: shared.state,
                catalog: shared.catalog.clone(),
                ..ExploreSnapshot::default()
            })
            .unwrap_or_default()
    }

    /// Refreshes the catalog on its own thread unless a refresh is running or the catalog is
    /// Ready and was fetched within `max_age` seconds. The receiver answers once the refresh
    /// ends; `None` when none started.
    pub(crate) fn refresh(&self, max_age: u64) -> AppResult<Option<oneshot::Receiver<()>>> {
        {
            let mut shared = self.lock()?;
            let fresh = shared.state == CatalogState::Ready
                && persistence::now().saturating_sub(shared.fetched_at) < max_age;
            if shared.running || fresh {
                return Ok(None);
            }
            shared.running = true;
            shared.state = CatalogState::Loading;
        }
        let running = Running(Some(self.clone()));
        let (done, finished) = oneshot::channel();
        let catalogs = self.clone();
        std::thread::Builder::new()
            .name("skillbinder-catalog".into())
            .spawn(move || {
                running.finish(catalogs.fetch());
                let _ = done.send(());
            })
            .map_err(|_| AppError::storage())?;
        Ok(Some(finished))
    }

    /// Fetches the catalog, and on a changed one the avatars, updating the cache.
    fn fetch(&self) -> Outcome {
        let cache = Cache::new(&self.data_dir);
        let fetcher = Fetcher::new(&self.source);
        let cached = cache.load();
        let etag = cached.as_ref().and_then(|(_, meta)| meta.etag.clone());
        let state = match fetcher.get(CATALOG, etag.as_deref(), CATALOG_LIMIT) {
            Answer::Body { bytes, etag } => match translate(&bytes) {
                Ok(translated) => {
                    let mut meta = Meta {
                        etag,
                        fetched_at: persistence::now(),
                        avatars: cached.map(|(_, meta)| meta.avatars).unwrap_or_default(),
                    };
                    // The cache is a convenience: the catalog shows even when it can't be saved.
                    // Unsaved, it keeps no ETag, so the next refresh never confirms the old file.
                    if cache.save_catalog(&bytes, &meta).is_err() {
                        meta.etag = None;
                    }
                    self.fetch_avatars(&fetcher, &cache, &translated, &mut meta, false);
                    return Outcome::fetched(translated, &cache, meta.fetched_at);
                }
                Err(Refusal::Outdated) => CatalogState::Outdated,
                Err(Refusal::Unreadable) => CatalogState::Unavailable,
            },
            Answer::NotModified => match cached {
                Some((translated, mut meta)) => {
                    meta.fetched_at = persistence::now();
                    // Avatars a failed fetch left out are fetched again.
                    self.fetch_avatars(&fetcher, &cache, &translated, &mut meta, true);
                    return Outcome::fetched(translated, &cache, meta.fetched_at);
                }
                None => CatalogState::Unavailable,
            },
            Answer::NotFound => CatalogState::Outdated,
            Answer::Offline => CatalogState::Offline,
            Answer::Failed => CatalogState::Unavailable,
        };
        // Every failure keeps the catalog already shown.
        Outcome {
            state,
            catalog: None,
            fetched_at: None,
        }
    }

    /// Fetches each creator's avatar with its own ETag, or with `missing` only those not cached,
    /// then removes the avatars of creators the catalog no longer lists, and saves `meta`.
    fn fetch_avatars(
        &self,
        fetcher: &Fetcher,
        cache: &Cache,
        translated: &Translated,
        meta: &mut Meta,
        missing: bool,
    ) {
        for (creator, path) in &translated.avatars {
            if self.shutdown.cancelled() {
                break;
            }
            let cached = cache.avatar(creator).is_file();
            if missing && cached {
                continue;
            }
            let etag = meta.avatars.get(creator).filter(|_| cached);
            match fetcher.get(
                &format!("v1/{path}"),
                etag.map(String::as_str),
                AVATAR_LIMIT,
            ) {
                Answer::Body { bytes, etag } => {
                    if cache.save_avatar(creator, &bytes).is_ok() {
                        match etag {
                            Some(etag) => meta.avatars.insert(creator.clone(), etag),
                            None => meta.avatars.remove(creator),
                        };
                    }
                }
                Answer::NotModified | Answer::NotFound | Answer::Offline | Answer::Failed => {}
            }
        }
        let listed = |creator: &str| translated.avatars.iter().any(|(id, _)| id == creator);
        cache.retain_avatars(listed);
        meta.avatars.retain(|creator, _| listed(creator));
        let _ = cache.save_meta(meta);
    }
}

/// What a refresh ended with: its state, and the catalog to show when the source answered with
/// one.
struct Outcome {
    state: CatalogState,
    catalog: Option<Catalog>,
    /// When the source returned or confirmed the catalog; `None` when it did not.
    fetched_at: Option<u64>,
}

impl Outcome {
    fn fetched(translated: Translated, cache: &Cache, fetched_at: u64) -> Self {
        Self {
            state: CatalogState::Ready,
            catalog: Some(with_avatars(translated, cache)),
            fetched_at: Some(fetched_at),
        }
    }
}

/// A running refresh. Finishing it publishes its outcome; dropping it unfinished, as a panicking
/// thread does, ends it as Unavailable so a later refresh can start.
struct Running(Option<Catalogs>);

impl Running {
    fn finish(mut self, outcome: Outcome) {
        if let Some(catalogs) = self.0.take() {
            catalogs.end(outcome);
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        if let Some(catalogs) = self.0.take() {
            catalogs.end(Outcome {
                state: CatalogState::Unavailable,
                catalog: None,
                fetched_at: None,
            });
        }
    }
}

impl Catalogs {
    fn end(&self, outcome: Outcome) {
        let Ok(mut shared) = self.shared.lock() else {
            return;
        };
        shared.running = false;
        shared.state = outcome.state;
        if let Some(catalog) = outcome.catalog {
            shared.catalog = Some(Arc::new(catalog));
        }
        if let Some(fetched_at) = outcome.fetched_at {
            shared.fetched_at = fetched_at;
        }
    }
}

/// The catalog with each creator's cached avatar, where there is one.
fn with_avatars(translated: Translated, cache: &Cache) -> Catalog {
    let mut catalog = translated.catalog;
    for creator in &mut catalog.creators {
        let path = cache.avatar(&creator.id);
        creator.avatar = path.is_file().then(|| path.display().to_string());
    }
    catalog
}
