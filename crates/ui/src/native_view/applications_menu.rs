//! The applications menu (spec rule 23): launched on Linux as an AppImage, the app asks once,
//! before anything else, onboarding included, whether to add itself to the applications menu.
//! Add writes the menu entry and closes; a failed Add says so in the dialog, which stays open.
//! Not now, or closing the dialog, declines. Either answer is remembered.

use super::{Effect, NativeView};
use gpui_kit::component::dialog::Dialog;
use gpui_kit::{Context, ParentElement, prelude::FluentBuilder};
use skillbinder_design::tokens::text;
use skillbinder_design::{
    Button, DialogFooter, DialogHeader, DialogSize, Notice, NoticeSize, Status, Text,
};

const TITLE: &str = "Add SkillBinder to your applications menu?";
const BODY: &str = "Open SkillBinder from the menu like your other apps.";
const FAILED: &str = "Couldn't add SkillBinder to the applications menu.";

impl NativeView {
    /// The dialog, with the line saying the last Add failed, if it did.
    pub(super) fn applications_menu_dialog(
        &self,
        dialog: Dialog,
        failed: bool,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let not_now = Button::new("applications-menu-not-now")
            .label("Not now")
            .inactive(self.working())
            .on_click(cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx)));
        let add = Button::new("applications-menu-add")
            .label("Add")
            .primary()
            .inactive(self.working())
            .on_click(cx.listener(|view, _, _, cx| view.answer_applications_menu(true, cx)));
        let footer = DialogFooter::new().secondary(not_now).primary(add);
        self.dialog_frame(DialogSize::Compact, DialogHeader::new(TITLE), cx)
            .footer(footer)
            .apply(dialog, cx)
            .child(Text::new(BODY).level(text::BODY_SM))
            .when(failed, |dialog| {
                dialog.child(
                    Notice::new("applications-menu-failed", FAILED)
                        .status(Status::Danger)
                        .size(NoticeSize::Md),
                )
            })
    }

    /// Answers the offer: `add` writes the menu entry and closes the dialog, or says it failed;
    /// declining only remembers the answer, as closing the dialog declines.
    pub(super) fn answer_applications_menu(&mut self, add: bool, cx: &mut Context<Self>) {
        let client = self.client.clone();
        self.run(
            async move {
                let answered = client.answer_applications_menu(add).await;
                Ok(match (add, answered) {
                    (true, Ok(())) => Effect::CloseDialog,
                    (true, Err(_)) => Effect::ApplicationsMenuFailed,
                    (false, answered) => answered.map(|()| Effect::None)?,
                })
            },
            cx,
        );
    }
}
