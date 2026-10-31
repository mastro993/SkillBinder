//! Activation operations: they change your agents' or a workspace's skill directories, so each
//! scans again once it lands.

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
    /// Holds a library skill in a workspace for `targets`, or deactivates it there with `None`,
    /// then scans again so the workspace's found skills show its copies.
    pub async fn set_workspace_skill(
        &self,
        workspace: WorkspaceId,
        skill: SkillId,
        targets: Option<WorkspaceTargets>,
    ) -> AppResult<()> {
        let set = self
            .shared
            .engine
            .set_workspace_skill(workspace, skill, targets)
            .await;
        self.refresh_preserving_outcome(set).await?;
        self.spawn_scan();
        Ok(())
    }
    /// Tries again every directory where a held skill is not applied, then scans again.
    pub async fn retry_copies(&self) -> AppResult<()> {
        let retried = self.shared.engine.retry_copies().await;
        self.refresh_preserving_outcome(retried).await?;
        self.spawn_scan();
        Ok(())
    }
}
