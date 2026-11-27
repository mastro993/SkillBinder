//! Sync's requests and changes: History, connecting a remote, renaming this device, each Git
//! operation, and Restore and Discard.

use crate::Client;
use skillbinder_proto::*;

impl Client {
    /// Reads a page of History: up to `limit` changes after the commit `before`, or the newest
    /// ones. A request, not part of the snapshot: read it again when the sync snapshot's `head`
    /// moves.
    pub async fn sync_history(
        &self,
        before: Option<String>,
        limit: usize,
    ) -> AppResult<SyncHistory> {
        self.shared.engine.sync_history(before, limit).await
    }
    /// Connects and fetches a remote without implicit commits.
    pub async fn connect_remote(&self, remote: RemoteConfig) -> AppResult<SyncSnapshot> {
        self.refresh_preserving_outcome(self.shared.engine.connect_remote(remote).await)
            .await
    }
    /// Puts a skill back to its version in a History change, as a change on this device, then
    /// scans again, since its copies were rewritten.
    pub async fn restore_skill(&self, id: SkillId, commit: String) -> AppResult<()> {
        self.refresh_preserving_outcome(self.shared.engine.restore_skill(id, commit).await)
            .await?;
        self.spawn_scan();
        Ok(())
    }
    /// Undoes this device's change to a skill not pushed yet, then scans again: its copies were
    /// rewritten, or, for a skill added here, kept as found copies.
    pub async fn discard_change(&self, id: SkillId) -> AppResult<()> {
        self.refresh_preserving_outcome(self.shared.engine.discard_change(id).await)
            .await?;
        self.spawn_scan();
        Ok(())
    }
    /// Renames this device; commits Push or Sync makes from now on carry the new name.
    pub async fn rename_device(&self, name: String) -> AppResult<()> {
        self.refresh_preserving_outcome(self.shared.engine.rename_device(name).await)
            .await
    }
    /// Performs the synchronization or remote refresh the user asked for, publishing it as
    /// `syncing` while it runs.
    pub async fn synchronize(&self, action: SyncAction) -> AppResult<SyncSnapshot> {
        self.publish_syncing(Some(action));
        let synchronized = self.synchronized(action).await;
        self.publish_syncing(None);
        synchronized
    }
    /// Synchronizes or refreshes the remote. A Pull or Sync may rewrite your agents' copies, so
    /// it scans again.
    pub(crate) async fn synchronized(&self, action: SyncAction) -> AppResult<SyncSnapshot> {
        let synchronized = self.shared.engine.synchronize(action).await;
        if matches!(action, SyncAction::Pull | SyncAction::Sync) {
            self.spawn_scan();
        }
        self.refresh_preserving_outcome(synchronized).await
    }
}
