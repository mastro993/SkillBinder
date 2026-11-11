//! Activation operations: they change your agents' or a workspace's skill directories, so each
//! scans again once it lands. Those that can write many copies publish how far they have got.

use crate::Client;
use skillbinder_proto::*;

impl Client {
    /// Applies preset or membership edits, publishing their progress while they run. One that
    /// wrote or removed copies scans again, so the agents' found skills show them.
    pub async fn change_organization(
        &self,
        change: OrganizationChange,
    ) -> AppResult<LibrarySnapshot> {
        let before = self
            .snapshot()
            .library
            .as_ref()
            .map(|old| old.copies.clone());
        let changed = self
            .tracked(self.shared.engine.change_organization(change))
            .await;
        let library = self.refresh_preserving_outcome(changed).await?;
        if before.as_ref() != Some(&library.copies) {
            self.spawn_scan();
        }
        Ok(library)
    }
    /// Applies a plan with durable replay and outcome retention, publishing its progress while it
    /// runs, then scans again so the imported skills are found in the library; that scan's
    /// completion checks their sources.
    pub async fn apply_import(&self, plan: PlanId) -> AppResult<ImportOutcome> {
        let applied = self.tracked(self.shared.engine.apply_import(plan)).await;
        let outcome = self.refresh_preserving_outcome(applied).await?;
        self.spawn_scan();
        Ok(outcome)
    }
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
