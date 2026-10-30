//! A device whose library has pushed its skills to a local bare remote, which a peer clone can
//! change, so tests can Pull what other devices pushed.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#![allow(dead_code, reason = "Not every test crate uses every helper.")]

use crate::support::Fixture;
use skillbinder_engine::Engine;
use skillbinder_proto::{LibrarySnapshot, OrganizationChange, RemoteConfig, SyncAction};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// Runs Git in `dir` as a peer device would.
pub fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=Peer",
            "-c",
            "user.email=peer@example.invalid",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}");
    String::from_utf8(output.stdout).unwrap()
}

/// A device whose library holds `slugs` and has pushed them to a fresh bare remote.
pub struct Device {
    pub fixture: Fixture,
    pub engine: Engine,
    pub remote: RemoteConfig,
}
impl Device {
    pub async fn published(slugs: &[&str]) -> Self {
        let fixture = Fixture::new();
        let engine = fixture.ready().await;
        import(&fixture, &engine, slugs).await;
        let bare = fixture.workspace.with_file_name("remote.git");
        fs::create_dir(&bare).unwrap();
        git(&bare, &["init", "--bare", "--initial-branch=main"]);
        let remote = RemoteConfig {
            url: url::Url::from_file_path(&bare).unwrap().into(),
            branch: "main".into(),
        };
        engine.connect_remote(remote.clone()).await.unwrap();
        engine.synchronize(SyncAction::Push).await.unwrap();
        Self {
            fixture,
            engine,
            remote,
        }
    }

    pub fn library(&self) -> PathBuf {
        self.fixture.data.join("library")
    }

    /// Clones the remote, lets `edit` change the clone, and pushes the result.
    pub fn peer_push(&self, edit: impl FnOnce(&Path)) {
        let peer = self.fixture.workspace.with_file_name("peer");
        if !peer.exists() {
            git(peer.parent().unwrap(), &["clone", &self.remote.url, "peer"]);
        }
        edit(&peer.join("skills"));
        git(&peer, &["add", "-A"]);
        git(&peer, &["commit", "-m", "peer"]);
        git(&peer, &["push", "origin", "main"]);
    }

    pub async fn organize(&self, change: OrganizationChange) -> LibrarySnapshot {
        self.engine.change_organization(change).await.unwrap()
    }
}

/// Imports one workspace skill per slug.
pub async fn import(fixture: &Fixture, engine: &Engine, slugs: &[&str]) {
    for slug in slugs {
        fixture.skill(slug, "First version");
    }
    if slugs.is_empty() {
        return;
    }
    fixture.register(engine).await;
    let ids = fixture
        .scan(engine)
        .await
        .candidates
        .into_iter()
        .map(|candidate| candidate.id)
        .collect();
    let plan = engine.prepare_import(ids, false).await.unwrap();
    engine.apply_import(plan.id).await.unwrap();
}
