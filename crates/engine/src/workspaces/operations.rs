//! Public engine operations for added workspaces.

use crate::worker::Engine;
use skillbinder_proto::*;

impl Engine {
    /// Saves a granted folder's projects as workspaces: the folder itself and each direct child
    /// that has an agent skill directory, or the folder alone when none has one. Agent folders,
    /// such as the home folder, are skipped. An expired grant, or a folder that adds nothing, is
    /// an error.
    pub async fn add_workspaces(&self, grant: GrantId) -> AppResult<Vec<Workspace>> {
        self.call(move |state| state.add_workspaces(&grant)).await
    }

    /// Renames a workspace; names need not be unique.
    pub async fn rename_workspace(&self, id: WorkspaceId, name: String) -> AppResult<()> {
        self.call(move |state| state.rename_workspace(&id, &name))
            .await
    }

    /// Sets or clears a workspace glyph's color.
    pub async fn set_workspace_color(
        &self,
        id: WorkspaceId,
        color: Option<PresetColor>,
    ) -> AppResult<()> {
        self.call(move |state| state.set_workspace_color(&id, color))
            .await
    }

    /// Sets a workspace glyph, or restores the default folder glyph, and removes its image icon.
    pub async fn set_workspace_icon(
        &self,
        id: WorkspaceId,
        icon: Option<PresetIcon>,
    ) -> AppResult<()> {
        self.call(move |state| state.set_workspace_icon(&id, icon))
            .await
    }

    /// Copies a picked image into application storage and makes it the workspace icon.
    pub async fn set_workspace_image(
        &self,
        id: WorkspaceId,
        image: std::path::PathBuf,
    ) -> AppResult<()> {
        self.call(move |state| state.set_workspace_image(&id, &image))
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
