use super::copy_dialogs::CopiesListing;
use super::{DialogState, NativeView};
use gpui_kit::component::{WindowExt, dialog::Dialog};
use gpui_kit::{Context, Pixels, Size, Window};
use skillbinder_design::{Button, DialogFrame, DialogHeader, DialogSize};

impl DialogState {
    /// Whether this dialog stays up while an operation runs, `busy`: every dialog but the review
    /// list, which closes while one of its answers is being carried out, and a skill's detail,
    /// which changes nothing.
    fn locked(&self, busy: bool) -> bool {
        busy && !matches!(
            self,
            Self::Copies(CopiesListing::Attention(_)) | Self::SkillDetail(_)
        )
    }
}

impl NativeView {
    pub(super) fn sync_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.drop_lost_picker();
        if matches!(self.dialog, DialogState::None) {
            if window.has_active_dialog(cx) {
                window.close_dialog(cx);
            }
        } else if !window.has_active_dialog(cx) {
            let weak = cx.entity().downgrade();
            self.dialogs_opened += 1;
            let opened = self.dialogs_opened;
            window.open_dialog(cx, move |dialog, window, cx| {
                let Some(view) = weak.upgrade() else {
                    return dialog;
                };
                view.update(cx, |view, cx| {
                    // Kit builds a closing dialog's layer through its exit; built from the view
                    // once a newer dialog opened, it would draw that one's focused field twice.
                    if view.dialogs_opened != opened {
                        return dialog;
                    }
                    let close_view = cx.entity().downgrade();
                    let locked = view.dialog_locked();
                    // A click outside the picker or the Import skills dialog keeps it, so a long
                    // selection is not lost.
                    let picking = matches!(
                        view.dialog,
                        DialogState::AddSkill(_)
                            | DialogState::ImportSkills(_)
                            | DialogState::FirstImport
                    );
                    view.build_dialog(dialog, window.viewport_size(), cx)
                        .overlay_closable(!locked && !picking)
                        .keyboard(!locked)
                        .on_ok(|_, _, _| false)
                        .on_close(move |_, _, cx| {
                            let _ = close_view.update(cx, |view, cx| {
                                view.clear_dialog(cx);
                                cx.notify();
                            });
                        })
                })
            });
            // The dialog takes focus as it opens; the picker hands it to its search, Add repository
            // to its URL, Change branch to its branch, and Rename this device to its name.
            let field = match &self.dialog {
                DialogState::AddSkill(picker) => Some(&picker.search),
                DialogState::AddRepository(add) => Some(&add.input),
                DialogState::ChangeBranch => Some(&self.remote_branch),
                DialogState::RenameDevice => Some(&self.device_name),
                _ => None,
            };
            if let Some(field) = field {
                field.update(cx, |input, cx| input.focus(window, cx));
            }
        }
    }

    /// Whether the open dialog stays up while an operation runs, as [`DialogState::locked`] says.
    fn dialog_locked(&self) -> bool {
        self.dialog.locked(self.working())
    }

    pub(super) fn dismiss_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.dialog_locked() {
            self.clear_dialog(cx);
            window.close_dialog(cx);
            cx.notify();
        }
    }

    /// Forgets the closed dialog. Closing the applications-menu offer, by any route, answers it
    /// as Not now.
    fn clear_dialog(&mut self, cx: &mut Context<Self>) {
        let closed = std::mem::replace(&mut self.dialog, DialogState::None);
        if matches!(closed, DialogState::ApplicationsMenu { .. }) {
            self.answer_applications_menu(false, cx);
        }
    }

    /// The frame of a dialog of `size` under `header`, which ends with the close button while
    /// the dialog can close.
    pub(super) fn dialog_frame(
        &self,
        size: DialogSize,
        header: DialogHeader,
        cx: &mut Context<Self>,
    ) -> DialogFrame {
        let header = if self.dialog_locked() {
            header
        } else {
            header.close(
                "dialog-close",
                "Close dialog",
                cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx)),
            )
        };
        DialogFrame::new(size).header(header)
    }

    /// The secondary Cancel button `id`, which closes the dialog.
    pub(super) fn dialog_cancel(&self, id: &'static str, cx: &mut Context<Self>) -> Button {
        Button::new(id)
            .label("Cancel")
            .inactive(self.working())
            .on_click(cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx)))
    }

    fn build_dialog(
        &self,
        dialog: Dialog,
        viewport: Size<Pixels>,
        cx: &mut Context<Self>,
    ) -> Dialog {
        match &self.dialog {
            DialogState::None => dialog,
            DialogState::AddSkill(picker) => self.skill_picker_dialog(dialog, picker, viewport, cx),
            DialogState::ImportSkills(state) => {
                self.import_skills_dialog(dialog, state, viewport, cx)
            }
            DialogState::FirstImport => match &self.first_import {
                Some(list) => self.import_skills_dialog(dialog, list, viewport, cx),
                None => dialog,
            },
            DialogState::DeletePreset(preview) => self.delete_preset_dialog(dialog, preview, cx),
            DialogState::Copies(CopiesListing::Attention(review)) => {
                self.review_dialog(dialog, review, cx)
            }
            DialogState::Copies(CopiesListing::Changed(changed)) => {
                self.changed_dialog(dialog, changed, cx)
            }
            DialogState::Copies(listing) => self.copies_dialog(dialog, listing, cx),
            DialogState::RemoveWorkspace(id) => self.remove_workspace_dialog(dialog, id, cx),
            DialogState::SkillDetail(detail) => {
                self.skill_detail_dialog(dialog, detail, viewport, cx)
            }
            DialogState::RemoveFromWorkspace { workspace, skill } => {
                self.remove_from_workspace_dialog(dialog, workspace, skill, cx)
            }
            DialogState::DeleteFolder { id, revision } => {
                self.delete_folder_dialog(dialog, id, revision, cx)
            }
            DialogState::AddRepository(add) => self.add_repository_dialog(dialog, add, cx),
            DialogState::RemoveRepository(repository) => {
                self.remove_repository_dialog(dialog, repository, cx)
            }
            DialogState::MoveToApplications { failed } => {
                self.move_to_applications_dialog(dialog, *failed, cx)
            }
            DialogState::ApplicationsMenu { failed } => {
                self.applications_menu_dialog(dialog, *failed, cx)
            }
            DialogState::ChangeBranch => self.change_branch_dialog(dialog, cx),
            DialogState::RenameDevice => self.rename_device_dialog(dialog, cx),
            DialogState::Disconnect => self.disconnect_dialog(dialog, cx),
            DialogState::DiscardChange(skill) => self.discard_dialog(dialog, skill, cx),
            DialogState::RestoreVersion(restoring) => self.restore_dialog(dialog, restoring, cx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::DialogState;
    use super::super::copies_review::Review;
    use super::CopiesListing;
    use skillbinder_proto::WorkspaceId;

    #[test]
    fn only_the_review_list_closes_while_an_operation_runs() {
        let review = DialogState::Copies(CopiesListing::Attention(Review::default()));
        assert!(!review.locked(true));
        assert!(!review.locked(false));
        for other in [
            DialogState::Copies(CopiesListing::Asking),
            DialogState::RemoveWorkspace(WorkspaceId::new()),
        ] {
            assert!(other.locked(true));
            assert!(!other.locked(false), "nothing runs, so it closes");
        }
    }
}
