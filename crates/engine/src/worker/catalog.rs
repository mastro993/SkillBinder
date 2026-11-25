//! The engine's side of Explore's projection: refreshing the catalog on its own thread, and
//! reading the catalog, your repositories, installs, preview, and skills.sh search without the
//! worker.

use super::{Engine, action::Action};
use crate::catalog::{Catalogs, Previews, Repositories, Resolutions, Searches};
use crate::install::Installs;
use crate::logging::{Area, Level};
use crate::storage;
use skillbinder_proto::*;

impl Engine {
    /// Refreshes Explore's catalog on its own thread and waits for it to end, unless a refresh is
    /// running or the catalog is current and was fetched within `max_age`.
    pub async fn refresh_catalog(&self, max_age: std::time::Duration) -> AppResult<()> {
        self.action(Action::RefreshCatalog, Vec::new(), |engine| async move {
            let started = std::time::Instant::now();
            if let Some(finished) = engine.owner.catalog.refresh(max_age.as_secs())? {
                finished
                    .await
                    .map_err(storage::error("waiting for the catalog refresh"))?;
                let current = engine.owner.catalog.current();
                engine
                    .log
                    .event(Level::Info, Area::Explore, "catalog refresh finished")
                    .field("state", format!("{:?}", current.state).to_lowercase())
                    .field(
                        "skills",
                        current.catalog.map_or(0, |catalog| catalog.skills.len()),
                    )
                    .took(started);
            }
            Ok(())
        })
        .await
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
