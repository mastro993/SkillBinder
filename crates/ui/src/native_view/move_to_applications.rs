//! Move to Applications (spec rule 21): launched on macOS from outside `/Applications` and
//! `~/Applications`, the app asks before anything else, onboarding included. Move copies it there
//! and quits, and the moved copy starts; a failed move says so in the dialog, which Not now
//! closes so the app goes on where it is. Not now always goes on, and the dialog asks again at
//! the next launch.

use super::{DialogState, Effect, NativeView};
use gpui_kit::component::dialog::Dialog;
use gpui_kit::{Context, ParentElement, prelude::FluentBuilder};
use skillbinder_design::tokens::text;
use skillbinder_design::{
    Button, DialogFooter, DialogHeader, DialogSize, Notice, NoticeSize, Status, Text,
};

const TITLE: &str = "Move SkillBinder to Applications?";
const BODY: &str = "SkillBinder updates itself only from your Applications folder.";
const FAILED: &str = "Couldn't move SkillBinder. Drag it to Applications yourself.";

impl NativeView {
    /// The dialog the app opens with: asking to move outside Applications, or none.
    pub(super) fn first_dialog(outside_applications: bool) -> DialogState {
        if outside_applications {
            DialogState::MoveToApplications { failed: false }
        } else {
            DialogState::None
        }
    }

    /// The dialog, with the line saying the last Move failed, if it did.
    pub(super) fn move_to_applications_dialog(
        &self,
        dialog: Dialog,
        failed: bool,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let not_now = Button::new("move-to-applications-not-now")
            .label("Not now")
            .inactive(self.busy)
            .on_click(cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx)));
        let move_app = Button::new("move-to-applications-move")
            .label("Move")
            .primary()
            .inactive(self.busy)
            .on_click(cx.listener(|view, _, _, cx| {
                let client = view.client.clone();
                view.run(
                    async move {
                        Ok(match client.move_to_applications().await {
                            Ok(()) => Effect::MovedToApplications,
                            Err(_) => Effect::MoveFailed,
                        })
                    },
                    cx,
                );
            }));
        let footer = DialogFooter::new().secondary(not_now).primary(move_app);
        self.dialog_frame(DialogSize::Compact, DialogHeader::new(TITLE), cx)
            .footer(footer)
            .apply(dialog, cx)
            .child(Text::new(BODY).level(text::BODY_SM))
            .when(failed, |dialog| {
                dialog.child(
                    Notice::new("move-to-applications-failed", FAILED)
                        .status(Status::Danger)
                        .size(NoticeSize::Md),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::super::{DialogState, NativeView};

    #[test]
    fn only_an_app_outside_applications_opens_asking_to_move() {
        assert!(matches!(
            NativeView::first_dialog(true),
            DialogState::MoveToApplications { failed: false }
        ));
        assert!(matches!(NativeView::first_dialog(false), DialogState::None));
    }
}
