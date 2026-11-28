//! Explore's operations: refreshing the catalog, searching skills.sh, resolving a skills.sh
//! result's folder, previewing a skill, installing it, and your repositories. They run on the
//! engine's own threads, so the explore projection publishes without waiting on the worker.

use crate::{Client, Shared};
use skillbinder_proto::*;
use std::{future::Future, sync::Arc, time::Duration};

/// How old the catalog, or a repository's listing, may be before opening Explore fetches it
/// again.
const MAX_AGE: Duration = Duration::from_secs(60 * 60);

impl Client {
    /// Fetches the catalog again when Explore opens, if it is older than an hour or the last
    /// refresh failed, and lists again each repository listed over an hour ago.
    pub fn explore_opened(&self) {
        self.spawn_catalog(MAX_AGE);
        let client = self.clone();
        self.shared.runtime.spawn(async move {
            let listed = client.shared.engine.refresh_repositories(MAX_AGE).await;
            client.shared.publish_explore();
            if let Err(error) = listed {
                client.background_error(error);
            }
        });
    }
    /// Fetches the catalog again if it is older than an hour or the last refresh failed, as
    /// opening Explore does, and waits for it; a failed refresh keeps the cached catalog.
    pub async fn fresh_catalog(&self) -> AppResult<()> {
        let refreshed = self.shared.engine.refresh_catalog(MAX_AGE).await;
        self.shared.publish_explore();
        refreshed
    }
    /// Asks skills.sh for `query` without changing the search Explore shows.
    pub async fn ask_skills_sh(&self, query: &str) -> AppResult<SkillsShState> {
        self.shared.engine.ask_skills_sh(query).await
    }
    /// Fetches the catalog now, as Retry does.
    pub fn refresh_catalog(&self) {
        self.spawn_catalog(Duration::ZERO);
    }
    /// Refreshes the catalog in the background unless it was fetched within `max_age`. The
    /// progress poll shows it loading; its result is published as soon as it ends.
    pub(crate) fn spawn_catalog(&self, max_age: Duration) {
        let client = self.clone();
        self.shared.runtime.spawn(async move {
            let refreshed = client.shared.engine.refresh_catalog(max_age).await;
            client.shared.publish_explore();
            if let Err(error) = refreshed {
                client.background_error(error);
            }
        });
    }
    /// Installs `skill` into the library, inactive. The projection shows it installing at once
    /// and, once it ends, holds the skill or the install's failure. The install runs on the
    /// engine's runtime, so it keeps running whatever happens to the caller's page.
    pub async fn install_skill(&self, skill: CatalogSkill) -> AppResult<()> {
        let client = self.clone();
        self.shared
            .runtime
            .spawn(async move {
                let install = client.shared.engine.install_skill(skill);
                client.shared.publish_explore();
                let installed = install.await;
                // The full read holds the library and the install's end together.
                client.refresh_preserving_outcome(installed).await
            })
            .await
            .map_err(|_| AppError::storage())?
    }
    /// Installs the skills.sh result `result`: resolves its folder, then installs the skill found,
    /// as [`Self::install_skill`] does.
    pub async fn install_skills_sh(&self, result: SkillsShResult) -> AppResult<()> {
        let client = self.clone();
        self.shared
            .runtime
            .spawn(async move {
                let install = client.shared.engine.install_skills_sh(result);
                client.shared.publish_explore();
                let installed = install.await;
                client.refresh_preserving_outcome(installed).await
            })
            .await
            .map_err(|_| AppError::storage())?
    }
    /// Asks skills.sh for `query` in place of any earlier query: the projection shows it
    /// searching at once, then its answer; an answer to an older query never shows.
    pub fn search_skills_sh(&self, query: &str) {
        let search = self.shared.engine.search_skills_sh(query);
        self.shared.publish_explore();
        let client = self.clone();
        self.shared.runtime.spawn(async move {
            let searched = search.await;
            client.shared.publish_explore();
            if let Err(error) = searched {
                client.background_error(error);
            }
        });
    }
    /// Finds the folder of the skills.sh result `result`, as opening its detail dialog does: the
    /// projection shows it resolving at once, then what was found.
    pub fn resolve(&self, result: SkillsShResult) {
        let resolve = self.shared.engine.resolve(result);
        self.spawn_published(async move {
            // The resolution's state shows what went wrong.
            let _ = resolve.await;
        });
    }
    /// Installs, one after another, each skill of repository `id` the library lacks. The
    /// projection shows its progress, and the library each skill as it lands.
    pub async fn install_all(&self, id: RepositoryId) -> AppResult<InstallAllOutcome> {
        let client = self.clone();
        self.shared
            .runtime
            .spawn(async move {
                let landed = {
                    let client = client.clone();
                    move || {
                        let client = client.clone();
                        client.shared.runtime.clone().spawn(async move {
                            if let Err(error) = client.refresh().await {
                                client.background_error(error);
                            }
                        });
                    }
                };
                let outcome = client.shared.engine.install_all(id, landed).await;
                client.refresh_preserving_outcome(outcome).await
            })
            .await
            .map_err(|_| AppError::storage())?
    }
    /// Lists repository `url`'s skills without adding it.
    pub async fn list_repository(
        &self,
        url: String,
    ) -> Result<RepositoryListing, RepositoryProblem> {
        self.shared.engine.list_repository(url).await
    }
    /// Lists repository `url` and adds it with its skills.
    pub async fn add_repository(&self, url: String) -> Result<Repository, RepositoryProblem> {
        let added = self.shared.engine.add_repository(url).await;
        self.shared.publish_explore();
        added
    }
    /// Forgets repository `id`; the skills installed from it stay in the library.
    pub async fn remove_repository(&self, id: RepositoryId) -> AppResult<()> {
        let removed = self.shared.engine.remove_repository(id).await;
        self.shared.publish_explore();
        removed
    }
    /// Adds a removed repository back with its last listing, as Undo does.
    pub async fn restore_repository(&self, repository: Repository) -> AppResult<()> {
        let restored = self.shared.engine.restore_repository(repository).await;
        self.shared.publish_explore();
        restored
    }
    /// Removes the Git caches nothing uses, such as a removed repository's once its Undo is no
    /// longer offered.
    pub fn prune_source_caches(&self) {
        let client = self.clone();
        self.shared.runtime.spawn(async move {
            if let Err(error) = client.shared.engine.prune_source_caches().await {
                client.background_error(error);
            }
        });
    }
    /// Opens `skill`'s preview for its detail dialog: the projection shows it loading at once,
    /// then its files and SKILL.md, or why it has none.
    pub fn preview(&self, skill: &CatalogSkill) {
        self.spawn_published(self.shared.engine.open_preview(skill));
    }
    /// Loads the open preview again, as the dialog's Retry does.
    pub fn retry_preview(&self) {
        self.spawn_published(self.shared.engine.retry_preview());
    }
    /// Drops the preview, as closing the dialog does.
    pub fn close_preview(&self) {
        self.shared.engine.close_preview();
        self.shared.publish_explore();
    }
    /// Publishes the projection at once, showing the work started, then again once it ends.
    fn spawn_published(&self, load: impl Future<Output = ()> + Send + 'static) {
        self.shared.publish_explore();
        let client = self.clone();
        self.shared.runtime.spawn(async move {
            load.await;
            client.shared.publish_explore();
        });
    }
}

impl Shared {
    /// Publishes the catalog, its state, your repositories, and the installs without waiting on
    /// the worker. It reads the engine under the snapshot's lock, so a publisher that read before
    /// a later change can't overwrite what that change published.
    pub(crate) fn publish_explore(&self) {
        self.snapshots.send_if_modified(|current| {
            let explore = self.engine.current_explore();
            if current.explore == explore {
                return false;
            }
            let mut next = (**current).clone();
            next.explore = explore;
            next.generation = current.generation + 1;
            *current = Arc::new(next);
            true
        });
    }
}
