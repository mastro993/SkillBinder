//! Public engine operations for added workspaces.

use crate::worker::{Action, Engine};
use skillbinder_proto::*;

impl Engine {
    /// Saves a granted folder's projects as workspaces: the folder itself and each direct child
    /// that has an agent skill directory, or the folder alone when none has one. Agent folders,
    /// such as the home folder, are skipped. An expired grant, or a folder that adds nothing, is
    /// an error. Each library skill a new workspace holds becomes active there, without writing.
    pub async fn add_workspaces(&self, grant: GrantId) -> AppResult<Vec<Workspace>> {
        self.action(Action::AddWorkspaces, Vec::new(), |engine| async move {
            engine.call(move |state| state.add_workspaces(&grant)).await
        })
        .await
    }

    /// Renames a workspace; names need not be unique.
    pub async fn rename_workspace(&self, id: WorkspaceId, name: String) -> AppResult<()> {
        self.action(Action::RenameWorkspace, subject(&id), |engine| async move {
            engine
                .call(move |state| state.rename_workspace(&id, &name))
                .await
        })
        .await
    }

    /// Sets or clears a workspace glyph's color.
    pub async fn set_workspace_color(
        &self,
        id: WorkspaceId,
        color: Option<PresetColor>,
    ) -> AppResult<()> {
        self.action(
            Action::SetWorkspaceColor,
            subject(&id),
            |engine| async move {
                engine
                    .call(move |state| state.set_workspace_color(&id, color))
                    .await
            },
        )
        .await
    }

    /// Sets a workspace glyph, or restores the default folder glyph, and removes its image icon.
    pub async fn set_workspace_icon(
        &self,
        id: WorkspaceId,
        icon: Option<PresetIcon>,
    ) -> AppResult<()> {
        self.action(
            Action::SetWorkspaceIcon,
            subject(&id),
            |engine| async move {
                engine
                    .call(move |state| state.set_workspace_icon(&id, icon))
                    .await
            },
        )
        .await
    }

    /// Copies a picked image into application storage and makes it the workspace icon.
    pub async fn set_workspace_image(
        &self,
        id: WorkspaceId,
        image: std::path::PathBuf,
    ) -> AppResult<()> {
        self.action(
            Action::SetWorkspaceImage,
            subject(&id),
            |engine| async move {
                engine
                    .call(move |state| state.set_workspace_image(&id, &image))
                    .await
            },
        )
        .await
    }

    /// Replaces the workspace order with a permutation of every workspace.
    pub async fn reorder_workspaces(&self, order: Vec<WorkspaceId>) -> AppResult<()> {
        self.action(Action::ReorderWorkspaces, Vec::new(), |engine| async move {
            engine
                .call(move |state| state.reorder_workspaces(&order))
                .await
        })
        .await
    }

    /// Holds a library skill in a workspace for all its agents or for specific workspace skill
    /// directories, or, with `None`, deactivates it there, and writes or removes its managed copies
    /// at once. A directory the workspace does not use yet must be a detected agent's, and joins
    /// its agents; a workspace without agents takes the chosen ones.
    pub async fn set_workspace_skill(
        &self,
        id: WorkspaceId,
        skill: SkillId,
        targets: Option<WorkspaceTargets>,
    ) -> AppResult<()> {
        let mut acted = subject(&id);
        acted.push(("slug", skill.to_string()));
        self.action(Action::SetWorkspaceSkill, acted, |engine| async move {
            engine
                .call(move |state| state.set_workspace_skill(&id, &skill, targets))
                .await
        })
        .await
    }

    /// Forgets a workspace without touching its files; its managed copies stay, unmanaged.
    pub async fn remove_workspace(&self, id: WorkspaceId) -> AppResult<()> {
        self.action(Action::RemoveWorkspace, subject(&id), |engine| async move {
            engine.call(move |state| state.remove_workspace(&id)).await
        })
        .await
    }
}

/// The workspace an action acts on.
fn subject(id: &WorkspaceId) -> crate::worker::Subject {
    vec![("workspace", id.to_string())]
}
