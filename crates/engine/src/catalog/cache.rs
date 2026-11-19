//! The catalog's cache under `<data>/catalog/`: the last v1 file with its ETag and fetch time,
//! and each creator's avatar with its own ETag. Every file is replaced atomically.

use super::snapshot::{Translated, translate};
use serde::{Deserialize, Serialize};
use skillbinder_proto::{AppError, AppResult};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const CATALOG: &str = "v1-catalog.json";
const META: &str = "v1-catalog.meta.json";
const AVATARS: &str = "avatars";

/// What the cache knows about its files.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct Meta {
    /// The cached catalog's ETag.
    pub etag: Option<String>,
    /// Unix seconds of the last answer that returned or confirmed the cached catalog.
    pub fetched_at: u64,
    /// Each cached avatar's ETag, by creator id.
    #[serde(default)]
    pub avatars: BTreeMap<String, String>,
}

pub(super) struct Cache {
    dir: PathBuf,
    staging: PathBuf,
}

impl Cache {
    pub(super) fn new(data_dir: &Path) -> Self {
        Self {
            dir: data_dir.join("catalog"),
            staging: data_dir.join("staging"),
        }
    }

    /// The cached catalog and what is known about it, or `None` when there is none or it no
    /// longer reads as a v1 catalog.
    pub(super) fn load(&self) -> Option<(Translated, Meta)> {
        let translated = translate(&fs::read(self.dir.join(CATALOG)).ok()?).ok()?;
        let meta = fs::read(self.dir.join(META))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Some((translated, meta))
    }

    /// Replaces the cached catalog and its meta.
    pub(super) fn save_catalog(&self, bytes: &[u8], meta: &Meta) -> AppResult<()> {
        self.write(&self.dir.join(CATALOG), bytes)?;
        self.save_meta(meta)
    }

    pub(super) fn save_meta(&self, meta: &Meta) -> AppResult<()> {
        let bytes = serde_json::to_vec(meta).map_err(|_| AppError::storage())?;
        self.write(&self.dir.join(META), &bytes)
    }

    /// Where `creator`'s avatar is cached.
    pub(super) fn avatar(&self, creator: &str) -> PathBuf {
        self.dir.join(AVATARS).join(format!("{creator}.png"))
    }

    pub(super) fn save_avatar(&self, creator: &str, bytes: &[u8]) -> AppResult<()> {
        self.write(&self.avatar(creator), bytes)
    }

    /// Removes the avatars of every creator but `keep`.
    pub(super) fn retain_avatars(&self, keep: impl Fn(&str) -> bool) {
        let Ok(entries) = fs::read_dir(self.dir.join(AVATARS)) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let creator = path.file_stem().and_then(|stem| stem.to_str());
            if !creator.is_some_and(&keep) {
                let _ = fs::remove_file(path);
            }
        }
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> AppResult<()> {
        let directory = path.parent().ok_or_else(AppError::storage)?;
        fs::create_dir_all(directory).map_err(|_| AppError::storage())?;
        fs::create_dir_all(&self.staging).map_err(|_| AppError::storage())?;
        let mut file =
            tempfile::NamedTempFile::new_in(&self.staging).map_err(|_| AppError::storage())?;
        file.write_all(bytes)
            .and_then(|()| file.as_file().sync_all())
            .map_err(|_| AppError::storage())?;
        file.persist(path).map_err(|_| AppError::storage())?;
        Ok(())
    }
}
