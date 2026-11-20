//! Your repositories (design T6): Git repositories the user added to Explore, kept on this device
//! only. Each keeps its last listing, so one that is offline, unreachable, or restored by Undo
//! keeps its skills without the network. Listing runs on the Git pool; only saving its result
//! runs on the worker.

mod listing;
mod store;

pub(crate) use listing::{Listed, Listing};

use crate::lifecycle::RepositoryUrls;
use skillbinder_proto::{InstallAllProgress, Repository, RepositoryId};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, MutexGuard},
};

#[derive(Default)]
struct Shared {
    saved: Arc<Vec<Repository>>,
    install_all: BTreeMap<RepositoryId, InstallAllProgress>,
    refreshing: bool,
}

/// Your repositories as saved, and their running Install alls, shared by the worker's
/// snapshots, the client, and the work that lists and installs them.
#[derive(Clone, Default)]
pub(crate) struct Repositories {
    shared: Arc<Mutex<Shared>>,
}

impl Repositories {
    fn lock(&self) -> MutexGuard<'_, Shared> {
        // Nothing panics while holding it, so a poisoned lock still holds consistent data.
        self.shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The saved repositories and each running Install all.
    pub(crate) fn current(
        &self,
    ) -> (
        Arc<Vec<Repository>>,
        BTreeMap<RepositoryId, InstallAllProgress>,
    ) {
        let shared = self.lock();
        (shared.saved.clone(), shared.install_all.clone())
    }

    /// Replaces the saved repositories, as the worker read them after a change.
    fn publish(&self, saved: Vec<Repository>) {
        self.lock().saved = Arc::new(saved);
    }

    /// Starts listing the repositories again; `None` while that runs already.
    pub(crate) fn begin_refresh(&self) -> Option<Refreshing> {
        let mut shared = self.lock();
        (!std::mem::replace(&mut shared.refreshing, true)).then(|| Refreshing(self.clone()))
    }

    /// Starts an Install all of repository `id`'s `total` skills; `None` while one runs there.
    pub(crate) fn begin_install_all(&self, id: &RepositoryId, total: usize) -> Option<InstallAll> {
        let mut shared = self.lock();
        if shared.install_all.contains_key(id) {
            return None;
        }
        shared
            .install_all
            .insert(id.clone(), InstallAllProgress { done: 0, total });
        Some(InstallAll {
            repositories: self.clone(),
            id: id.clone(),
        })
    }
}

/// A running listing of the repositories, which ends when dropped.
pub(crate) struct Refreshing(Repositories);

impl Drop for Refreshing {
    fn drop(&mut self) {
        self.0.lock().refreshing = false;
    }
}

/// A running Install all, counting its ended installs; dropping it ends it.
pub(crate) struct InstallAll {
    repositories: Repositories,
    id: RepositoryId,
}

impl InstallAll {
    /// Counts one more install ended.
    pub(crate) fn advance(&self) {
        if let Some(progress) = self.repositories.lock().install_all.get_mut(&self.id) {
            progress.done += 1;
        }
    }
}

impl Drop for InstallAll {
    fn drop(&mut self) {
        self.repositories.lock().install_all.remove(&self.id);
    }
}

/// Whether `url` can be added as a repository: an HTTPS URL without credentials, SSH, or
/// SCP-style, and `file://` only where `accepted` allows local repositories.
pub(crate) fn accepts(url: &str, accepted: RepositoryUrls) -> bool {
    let local = url::Url::parse(url).is_ok_and(|parsed| parsed.scheme() == "file");
    crate::git::validate_url(url).is_ok() && (!local || accepted == RepositoryUrls::Local)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_urls_are_accepted_only_for_local_repositories() {
        for url in [
            "https://github.com/acme/skills",
            "ssh://git@github.com/acme/skills.git",
            "git@github.com:acme/skills.git",
        ] {
            assert!(accepts(url, RepositoryUrls::Remote), "{url}");
        }
        // An absolute path starts at a drive on Windows.
        let local = if cfg!(windows) {
            "file:///C:/skills"
        } else {
            "file:///tmp/skills"
        };
        assert!(!accepts(local, RepositoryUrls::Remote));
        assert!(accepts(local, RepositoryUrls::Local));
        for url in [
            "not a url",
            "https://user:token@github.com/a/b",
            "http://example.com/a",
        ] {
            assert!(!accepts(url, RepositoryUrls::Local), "{url}");
        }
    }
}
