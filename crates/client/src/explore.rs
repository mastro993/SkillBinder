//! Explore's operations: refreshing the catalog, searching skills.sh, resolving a skills.sh
//! result's folder, previewing a skill, and installing it. They run on the engine's own threads,
//! so the explore projection publishes without waiting on the worker.

use crate::{Client, Shared};
use skillbinder_proto::*;
use std::{future::Future, sync::Arc, time::Duration};

/// How old the catalog may be before opening Explore fetches it again.
const CATALOG_MAX_AGE: Duration = Duration::from_secs(60 * 60);

impl Client {
    /// Fetches the catalog again when Explore opens, if it is older than an hour or the last
    /// refresh failed.
    pub fn explore_opened(&self) {
        self.spawn_catalog(CATALOG_MAX_AGE);
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
    /// Publishes the catalog, its state, and the installs without waiting on the worker.
    pub(crate) fn publish_explore(&self) {
        let explore = self.engine.current_explore();
        self.snapshots.send_if_modified(|current| {
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
