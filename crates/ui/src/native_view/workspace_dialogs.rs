//! Adding a workspace through the native directory picker, and the remove confirmation.

use super::controls::{Inactive, button};
use super::{Effect, NativeView};
use gpui_kit::component::dialog::Dialog;
use gpui_kit::{Context, ParentElement, PathPromptOptions, Styled, div};
use skillbinder_proto::*;

impl NativeView {
    /// Picks a folder, grants it, and adds it as a workspace; a scan then covers it.
    pub(super) fn choose_workspace_folder(&mut self, cx: &mut Context<Self>) {
        let picker = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Add workspace".into()),
        });
        let client = self.client.clone();
        let handle = client.runtime().spawn(async move {
            let path = match picker.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                Ok(Ok(None)) => None,
                _ => return Err(AppError::storage()),
            };
            let Some(path) = path else {
                return Ok(());
            };
            let grant = client.grant_directory(path).await?;
            client.add_workspace(grant.id).await.map(|_| ())
        });
        cx.spawn(async move |this, cx| {
            let result = handle.await;
            let _ = this.update(cx, |view, cx| {
                match result {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => view.error = Some(error),
                    Err(_) => view.error = Some(AppError::storage()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn remove_workspace_dialog(
        &self,
        dialog: Dialog,
        id: &WorkspaceId,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let name = self
            .snapshot
            .workspaces
            .iter()
            .find(|workspace| &workspace.id == id)
            .map_or_else(
                || "this workspace".to_owned(),
                |workspace| workspace.name.clone(),
            );
        let id = id.clone();
        dialog
            .title(self.dialog_title("Remove workspace", cx))
            .child(div().flex().flex_col().gap_2().child(format!("Remove {name}?")).child(
                "Removes the workspace from SkillBinder. Files in the folder are not touched.",
            ))
            .footer(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        button("remove-cancel")
                            .label("Cancel")
                            .inactive(self.busy)
                            .on_click(cx.listener(|view, _, window, cx| {
                                view.dismiss_dialog(window, cx)
                            })),
                    )
                    .child(
                        button("remove-confirm")
                            .label("Remove workspace")
                            .inactive(self.busy)
                            .on_click(cx.listener(move |view, _, _, cx| {
                                let client = view.client.clone();
                                let id = id.clone();
                                view.run(
                                    async move {
                                        client.remove_workspace(id).await.map(|_| Effect::CloseDialog)
                                    },
                                    cx,
                                );
                            })),
                    ),
            )
    }
}
