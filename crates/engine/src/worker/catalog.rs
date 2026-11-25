//! The engine's side of Explore's projection: refreshing the catalog on its own thread, and
//! reading the catalog, your repositories, installs, preview, and skills.sh search without the
//! worker.

use super::Engine;
use crate::catalog::{Catalogs, Previews, Repositories, Resolutions, Searches};
use crate::install::Installs;
use crate::storage;
use skillbinder_proto::*;

impl Engine {
    /// Refreshes Explore's catalog on its own thread and waits for it to end, unless a refresh is
    /// running or the catalog is current and was fetched within `max_age`.
    pub async fn refresh_catalog(&self, max_age: std::time::Duration) -> AppResult<()> {
        if let Some(finished) = self.owner.catalog.refresh(max_age.as_secs())? {
            finished
                .await
                .map_err(storage::error("waiting for the catalog refresh"))?;
        }
        Ok(())
    }
    /// Explore's catalog, its state, your repositories, the installs not idle, the open dialog's
    /// preview, and the skills.sh search with its results' folders, without waiting for a durable
    /// mutation.
    pub fn current_explore(&self) -> ExploreSnapshot {
        let owner = &self.owner;
        explore(
            &owner.catalog,
            &owner.installs,
            &owner.previews,
            &owner.repositories,
            &owner.searches,
            &owner.resolutions,
        )
    }
}

pub(super) fn explore(
    catalog: &Catalogs,
    installs: &Installs,
    previews: &Previews,
    repositories: &Repositories,
    searches: &Searches,
    resolutions: &Resolutions,
) -> ExploreSnapshot {
    let (repositories, install_all) = repositories.current();
    ExploreSnapshot {
        installs: installs.current(),
        preview: previews.current(),
        repositories,
        install_all,
        skills_sh: searches.current(),
        resolutions: resolutions.current(),
        ..catalog.current()
    }
}
