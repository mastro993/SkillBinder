//! Two devices whose libraries sync through one local bare remote: "Studio Mac" published
//! `alpha` and `beta`, and "Laptop" adopted them, so each can change the library and Sync.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#![allow(dead_code, reason = "Not every test crate uses every helper.")]

use crate::device::{Device, git};
use crate::support::{Fixture, write_skill};
use skillbinder_engine::Engine;
use skillbinder_proto::{SyncAction, SyncChange, SyncSnapshot};
use std::{fs, path::PathBuf};

/// One device of a [`Pair`].
pub struct Side {
    pub fixture: Fixture,
    pub engine: Engine,
}

impl Side {
    pub fn library(&self) -> PathBuf {
        self.fixture.data.join("library")
    }

    /// Changes `slug`'s skill outside SkillBinder to read `body`.
    pub fn edit(&self, slug: &str, body: &str) {
        write_skill(&self.library().join("skills"), slug, body);
    }

    /// The body `slug`'s skill reads, if the library holds it.
    pub fn body(&self, slug: &str) -> Option<String> {
        let path = self.library().join("skills").join(slug).join("SKILL.md");
        let text = fs::read_to_string(path).ok()?;
        Some(text.lines().last()?.to_owned())
    }

    pub async fn sync(&self) -> SyncSnapshot {
        self.engine.snapshot().await.unwrap().sync
    }

    pub async fn refresh(&self) -> SyncSnapshot {
        self.engine.synchronize(SyncAction::Refresh).await.unwrap()
    }

    pub async fn history(&self) -> Vec<SyncChange> {
        self.engine.sync_history(None, 50).await.unwrap().changes
    }

    /// The library's commit HEAD names.
    pub fn head(&self) -> String {
        git(&self.library(), &["rev-parse", "HEAD"])
            .trim()
            .to_owned()
    }
}

/// Two devices that share a remote, each with the same library.
pub struct Pair {
    pub studio: Side,
    pub laptop: Side,
    /// The remote's bare repository.
    pub remote: PathBuf,
}

impl Pair {
    pub async fn new() -> Self {
        let Device {
            fixture,
            engine,
            remote,
        } = Device::published(&["alpha", "beta"]).await;
        engine.rename_device("Studio Mac".into()).await.unwrap();
        let laptop = Fixture::new();
        let other = laptop.ready().await;
        other.rename_device("Laptop".into()).await.unwrap();
        other.connect_remote(remote.clone()).await.unwrap();
        // An empty library adopts the remote.
        other.synchronize(SyncAction::Sync).await.unwrap();
        let bare = fixture.workspace.with_file_name("remote.git");
        Self {
            studio: Side { fixture, engine },
            laptop: Side {
                fixture: laptop,
                engine: other,
            },
            remote: bare,
        }
    }

    /// The commit the remote's branch names.
    pub fn published(&self) -> String {
        git(&self.remote, &["rev-parse", "main"]).trim().to_owned()
    }
}
