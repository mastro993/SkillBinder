//! Explore's operations: refreshing the catalog and installing from it. Both run on the engine's
//! own threads, so the explore projection publishes without waiting on the worker.

use crate::{Client, Shared};
use skillbinder_proto::*;
use std::{sync::Arc, time::Duration};

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
