//! Public engine operations for added workspaces.

use crate::worker::Engine;
use skillbinder_proto::*;

impl Engine {
    /// Saves a granted folder as a workspace. An expired grant or a duplicate folder is an error.
    pub async fn add_workspace(&self, grant: GrantId) -> AppResult<Workspace> {
        self.call(move |state| state.add_workspace(&grant)).await
    }

    /// Renames a workspace; names need not be unique.
    pub async fn rename_workspace(&self, id: WorkspaceId, name: String) -> AppResult<()> {
        self.call(move |state| state.rename_workspace(&id, &name))
            .await
    }

    /// Replaces the workspace order with a permutation of every workspace.
    pub async fn reorder_workspaces(&self, order: Vec<WorkspaceId>) -> AppResult<()> {
        self.call(move |state| state.reorder_workspaces(&order))
            .await
    }

    /// Forgets a workspace without touching its files.
    pub async fn remove_workspace(&self, id: WorkspaceId) -> AppResult<()> {
        self.call(move |state| state.remove_workspace(&id)).await
    }
}
