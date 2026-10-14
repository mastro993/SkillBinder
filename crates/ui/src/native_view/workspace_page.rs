//! A workspace's page: its location, read-only. The skills a workspace holds are listed in
//! Discovery.

use super::NativeView;
use gpui_kit::{AnyElement, IntoElement, ParentElement, Styled, div};
use skillbinder_proto::WorkspaceId;

impl NativeView {
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
}
