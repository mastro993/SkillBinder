//! A workspace's page: its location, then Library scoped to the workspace, whose rows activate
//! skills in the workspace instead of the active preset.

use super::{NativeView, Route};
use gpui_kit::component::{ActiveTheme, v_flex};
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Styled, Window, div};
use skillbinder_proto::{Workspace, WorkspaceId};

impl NativeView {
    pub(super) fn render_workspace(
        &self,
        id: &WorkspaceId,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(workspace) = self.workspace(id) else {
            return div().into_any_element();
        };
        v_flex()
            .gap_4()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(workspace.path.clone()),
            )
            .children(
                workspace
                    .missing
                    .then_some("The workspace folder is missing."),
            )
            .child(self.render_library(window, cx))
            .into_any_element()
    }

    pub(super) fn workspace(&self, id: &WorkspaceId) -> Option<&Workspace> {
        self.snapshot
            .workspaces
            .iter()
            .find(|workspace| &workspace.id == id)
    }

    /// The workspace whose skills Library rows edit: the one whose page is open.
    pub(super) fn scoped_workspace(&self) -> Option<&Workspace> {
        match &self.route {
            Route::Workspace(id) => self.workspace(id),
            _ => None,
        }
    }
}
