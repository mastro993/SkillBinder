//! The full projection the worker reads: the library, settings, and Git status, with the
//! records of the threads that run beside it.

use super::{State, catalog};
use skillbinder_proto::*;
use std::collections::BTreeMap;

impl State {
    pub(super) fn snapshot(&mut self) -> AppResult<AppSnapshot> {
        let bootstrap = self.bootstrap()?;
        let library = if bootstrap.onboarding.finished {
            Some(self.library_snapshot()?)
        } else {
            None
        };
        let mut sync: SyncSnapshot = self.setting("sync")?.unwrap_or_default();
        if library.is_some()
            && let Some(git) = &self.git
            && self.library_dir().join(".git").exists()
        {
            let remote: Option<RemoteConfig> = self.setting("remote")?.flatten();
            let mut current = git.status(&self.library_dir(), remote.as_ref(), false)?;
            current.refreshed_at = sync.refreshed_at;
            current.last_error = sync.last_error;
            if current.remote.is_some() && current.last_error.is_some() {
                current.state = SyncState::Unreachable;
            }
            sync = current;
        }
        let (scan, scanning) = self.scans.current()?;
        let updates = match &library {
            Some(library) => self.updates.current(&library.library.skills)?,
            None => BTreeMap::new(),
        };
        Ok(AppSnapshot {
            checking: self.updates.running()?,
            // Update all and Sync run in the client, which keeps their state across reads.
            updating: None,
            update_all_outcome: None,
            syncing: None,
            generation: 0,
            background_error: None,
            bootstrap,
            library,
            home: self.config.home.display().to_string(),
            agents: self.agents()?,
            workspaces: self.workspaces()?,
            scan,
            scanning,
            ignored: self.ignored()?,
            updates,
            explore: catalog::explore(
                &self.catalog,
                &self.installs,
                &self.previews,
                &self.repositories,
                &self.searches,
                &self.resolutions,
            ),
            sync,
            // A snapshot is a worker job, so no tracked change is in flight while it runs.
            activation: None,
            preferences: self.setting("preferences")?.unwrap_or_default(),
            app_update: self.app_updates.current()?,
        })
    }
}
