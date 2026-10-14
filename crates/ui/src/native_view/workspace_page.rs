//! The page of System or an added workspace: its location, read-only. The skills a workspace
//! holds are listed in Discovery.

use std::path::{Path, PathBuf};

use super::NativeView;
use super::agent_skills::installed_agents;
use gpui_kit::{AnyElement, IntoElement, ParentElement, Styled, div};
use skillbinder_proto::WorkspaceId;

impl NativeView {
    pub(super) fn render_system(&self) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child("Agent skill locations under your home")
            .child(self.snapshot.home.clone())
            .into_any_element()
    }

    pub(super) fn render_workspace(&self, id: &WorkspaceId) -> AnyElement {
        let Some(workspace) = self.workspace(id) else {
            return div().into_any_element();
        };
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(workspace.path.clone())
            .children(
                workspace
                    .missing
                    .then_some("The workspace folder is missing."),
            )
            .into_any_element()
    }

    pub(super) fn workspace(&self, id: &WorkspaceId) -> Option<&skillbinder_proto::Workspace> {
        self.snapshot
            .workspaces
            .iter()
            .find(|workspace| &workspace.id == id)
    }

    /// The installed agents' global skill directories, which System's scan reads.
    pub(super) fn system_dirs(&self) -> Vec<PathBuf> {
        installed_agents(&self.snapshot.agents)
            .into_iter()
            .filter_map(|agent| agent.global_skills_dir.as_deref().map(PathBuf::from))
            .collect()
    }

    /// The agent skill directories at the top of a workspace folder, which its scan reads.
    pub(super) fn workspace_dirs(&self, path: &str) -> Vec<PathBuf> {
        let mut dirs: Vec<_> = self
            .snapshot
            .agents
            .iter()
            .map(|agent| Path::new(path).join(&agent.workspace_skills_dir))
            .collect();
        dirs.sort();
        dirs.dedup();
        dirs
    }
}
