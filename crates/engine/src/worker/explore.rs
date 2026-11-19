//! The engine's Explore projection: the catalog, refreshed on its own thread, your
//! repositories, the installs not idle, and the open dialog's preview, all read without waiting
//! for a durable mutation.

use super::Engine;
use crate::{
    catalog::{Catalogs, Previews, Repositories},
    install::Installs,
};
use skillbinder_proto::*;

impl Engine {
    /// Refreshes Explore's catalog on its own thread and waits for it to end, unless a refresh is
    /// running or the catalog is current and was fetched within `max_age`.
    pub async fn refresh_catalog(&self, max_age: std::time::Duration) -> AppResult<()> {
        if let Some(finished) = self.owner.catalog.refresh(max_age.as_secs())? {
            finished.await.map_err(|_| AppError::storage())?;
        }
        Ok(())
    }
    /// Explore's catalog, its state, your repositories, the installs not idle, and the open
    /// dialog's preview, without waiting for a durable mutation.
    pub fn current_explore(&self) -> ExploreSnapshot {
        projection(
            &self.owner.catalog,
            &self.owner.installs,
            &self.owner.previews,
            &self.owner.repositories,
        )
    }
}

/// Explore's projection, from the shared catalog, installs, previews, and repositories.
pub(super) fn projection(
    catalog: &Catalogs,
    installs: &Installs,
    previews: &Previews,
    repositories: &Repositories,
) -> ExploreSnapshot {
    let (repositories, install_all) = repositories.current();
    ExploreSnapshot {
        installs: installs.current(),
        preview: previews.current(),
        repositories,
        install_all,
        ..catalog.current()
    }
}
