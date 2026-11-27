//! Checks of the remote: at launch and on focus with the rest of the app's refresh, and every 15
//! minutes while a remote is set and nothing blocking runs (spec Design 8, A1). A check that
//! fails shows on the Sync page as Unreachable, never as a toast.

use crate::Client;
use skillbinder_proto::*;
use std::{sync::Arc, time::Duration};
use tokio::time::{Instant, MissedTickBehavior};

/// How often the remote is checked between launch, focus, and Check now.
pub(crate) const SYNC_CHECK_INTERVAL: Duration = Duration::from_secs(15 * 60);

impl Client {
    /// Checks the remote every `period` from now on, skipping a check while [`due`] says no.
    pub(crate) fn start_sync_checks(&self, period: Duration) {
        let weak = Arc::downgrade(&self.shared);
        self.shared.runtime.spawn(async move {
            let mut interval = tokio::time::interval_at(Instant::now() + period, period);
            interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
            loop {
                interval.tick().await;
                let Some(shared) = weak.upgrade() else {
                    break;
                };
                let client = Self { shared };
                if due(&client.snapshot()) {
                    client.check_remote().await;
                }
            }
        });
    }

    /// Fetches the remote's status in the background. A failure the engine recorded shows as
    /// Unreachable; any other is the app's background error.
    pub(crate) async fn check_remote(&self) {
        if let Err(error) = self.synchronized(SyncAction::Refresh).await
            && self.snapshot().sync.last_error.is_none()
        {
            self.background_error(error);
        }
    }
}

/// Whether a periodic check runs now: a remote is set, and no blocking change, Update all,
/// Sync, or install runs.
fn due(snapshot: &AppSnapshot) -> bool {
    let explore = &snapshot.explore;
    let installing = !explore.install_all.is_empty()
        || explore
            .installs
            .values()
            .any(|install| *install == InstallState::Installing);
    snapshot.library.is_some()
        && snapshot.sync.remote.is_some()
        && snapshot.activation.is_none()
        && snapshot.updating.is_none()
        && snapshot.syncing.is_none()
        && !installing
}

#[cfg(test)]
mod tests;
