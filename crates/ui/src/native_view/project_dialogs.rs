//! Adding a project through the native directory picker, its scan modal, and the remove
//! confirmation.

use super::controls::{Inactive, button};
use super::{DialogState, Effect, NativeView};
use gpui_kit::component::{button::ButtonVariants, dialog::Dialog, progress::Progress};
use gpui_kit::{Context, ParentElement, PathPromptOptions, Styled, div, prelude::FluentBuilder};
use skillbinder_proto::*;

impl NativeView {
    /// Picks a folder, grants it, and adds it as a project. The scan's progress and outcome
    /// arrive through the snapshot; only refusals before the scan starts are errors here.
    pub(super) fn choose_project_folder(&mut self, cx: &mut Context<Self>) {
        if self.adding_project() {
            return;
        }
        let picker = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Add project".into()),
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
            client.add_project(grant.id).await.map(|_| ())
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

    /// Opens the modal while a project addition is retained, and closes it once dismissed.
    pub(super) fn sync_addition_dialog(&mut self) {
        let addition = self.snapshot.project_addition.as_ref();
        let shown =
            addition.is_some_and(|addition| self.dismissed_addition.as_ref() != Some(&addition.id));
        match (&self.dialog, shown) {
            (DialogState::None, true) => self.dialog = DialogState::AddProject,
            (DialogState::AddProject, false) => self.dialog = DialogState::None,
            _ => {}
        }
        if addition.is_none() {
            self.dismissed_addition = None;
        }
    }

    /// Escape, the backdrop, Cancel, and Done close the modal: a running scan is cancelled,
    /// a finished one is dismissed.
    pub(super) fn close_addition(&mut self, cx: &mut Context<Self>) {
        let Some(addition) = &self.snapshot.project_addition else {
            return;
        };
        self.dismissed_addition = Some(addition.id.clone());
        self.dialog = DialogState::None;
        let running = addition.status == ScanStatus::Running;
        let client = self.client.clone();
        // Not `run`: a cancel must never be dropped because another operation is pending.
        let handle = client.runtime().spawn(async move {
            if running {
                client.cancel_project_addition().await
            } else {
                client.dismiss_project_addition().await.map(|()| true)
            }
        });
        cx.spawn(async move |this, cx| {
            let result = handle.await;
            let _ = this.update(cx, |view, cx| {
                match result {
                    Ok(Ok(true)) => {}
                    // The scan was saved before Cancel arrived: show its summary again.
                    Ok(Ok(false)) => {
                        view.dismissed_addition = None;
                        view.sync_addition_dialog();
                    }
                    Ok(Err(error)) => view.error = Some(error),
                    Err(_) => view.error = Some(AppError::storage()),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn add_project_dialog(&self, dialog: Dialog, cx: &mut Context<Self>) -> Dialog {
        let Some(addition) = &self.snapshot.project_addition else {
            return dialog;
        };
        let (title, action) = match addition.status {
            ScanStatus::Running => ("Adding project", "Cancel"),
            ScanStatus::Finished => ("Project added", "Done"),
            ScanStatus::Failed | ScanStatus::Cancelled => ("Could not add project", "Close"),
        };
        let mut body = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(format!("{} · {}", addition.name, addition.path));
        body = match addition.status {
            ScanStatus::Running => body
                .child(
                    Progress::new("project-scan-progress")
                        .loading(true)
                        .accessibility_label("Scanning project"),
                )
                .child(format!("{} folders checked", addition.folders_checked)),
            ScanStatus::Finished => body
                .child(format!(
                    "{} in library, {} importable",
                    addition.in_library,
                    addition.importable.len()
                ))
                .children(
                    addition
                        .importable
                        .iter()
                        .map(|slug| div().child(format!("{slug} · importable"))),
                ),
            ScanStatus::Failed | ScanStatus::Cancelled => body.child(
                addition
                    .error
                    .clone()
                    .unwrap_or_else(|| "The project could not be scanned.".into()),
            ),
        };
        let done = addition.status != ScanStatus::Running;
        dialog.title(title).child(body).footer(
            div().flex().gap_2().child(
                button("project-addition-close")
                    .label(action)
                    .when(done, |button| button.primary())
                    .inactive(self.busy)
                    .on_click(cx.listener(|view, _, _, cx| view.close_addition(cx))),
            ),
        )
    }

    pub(super) fn remove_project_dialog(
        &self,
        dialog: Dialog,
        id: &ProjectId,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let name = self
            .snapshot
            .projects
            .iter()
            .find(|project| &project.id == id)
            .map_or_else(|| "this project".to_owned(), |project| project.name.clone());
        let deployments = self.deployment_count(&DeploymentTarget::Project(id.clone()));
        let id = id.clone();
        dialog
            .title(self.dialog_title("Remove project", cx))
            .child(div().flex().flex_col().gap_2().child(format!("Remove {name}?")).child(
                format!(
                    "Removes the project and its {deployments} deployments from SkillBinder. Files in the folder are not touched."
                ),
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
                            .label("Remove project")
                            .inactive(self.busy)
                            .on_click(cx.listener(move |view, _, _, cx| {
                                let client = view.client.clone();
                                let id = id.clone();
                                view.run(
                                    async move {
                                        client.remove_project(id).await.map(|_| Effect::CloseDialog)
                                    },
                                    cx,
                                );
                            })),
                    ),
            )
    }
}
