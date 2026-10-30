//! Activation operations: they change your agents' skill directories, so each scans again once
//! it lands.

use crate::Client;
use skillbinder_proto::*;

impl Client {
    /// Replaces or keeps a conflicting, kept, or edited copy, then scans again.
    pub async fn resolve_copy(&self, copy: &ManagedCopy, choice: CopyChoice) -> AppResult<()> {
        let resolved = self
            .shared
            .engine
            .resolve_copy(copy.directory.clone(), copy.skill.clone(), choice)
            .await;
        self.refresh_preserving_outcome(resolved).await?;
        self.spawn_scan();
        Ok(())
    }
    /// Tries again every directory where the active preset is not applied, then scans again.
    pub async fn retry_copies(&self) -> AppResult<()> {
        let retried = self.shared.engine.retry_copies().await;
        self.refresh_preserving_outcome(retried).await?;
        self.spawn_scan();
        Ok(())
    }
}
