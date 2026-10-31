use super::controls::{Inactive, button};
use super::{DialogState, Effect, NativeView};
use gpui_kit::component::{
    IconName, Sizable, WindowExt, button::ButtonVariants, dialog::Dialog, h_flex,
};
use gpui_kit::{Context, IntoElement, ParentElement, Styled, Window, div, prelude::FluentBuilder};
use skillbinder_proto::*;

impl NativeView {
    pub(super) fn sync_dialog(&self, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(self.dialog, DialogState::None) {
            if window.has_active_dialog(cx) {
                window.close_dialog(cx);
            }
        } else if !window.has_active_dialog(cx) {
            let weak = cx.entity().downgrade();
            window.open_dialog(cx, move |dialog, _, cx| {
                let Some(view) = weak.upgrade() else {
                    return dialog;
                };
                view.update(cx, |view, cx| {
                    let close_view = cx.entity().downgrade();
                    view.build_dialog(dialog, cx)
                        .overlay_closable(!view.busy)
                        .close_button(false)
                        .keyboard(!view.busy)
                        .on_ok(|_, _, _| false)
                        .on_close(move |_, _, cx| {
                            let _ = close_view.update(cx, |view, cx| {
                                view.dialog = DialogState::None;
                                cx.notify();
                            });
                        })
                })
            });
        }
    }

    pub(super) fn dismiss_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.busy {
            self.dialog = DialogState::None;
            window.close_dialog(cx);
            cx.notify();
        }
    }

    /// Title row with an app close button: GPUI Kit's built-in close control keeps the arrow cursor.
    pub(super) fn dialog_title(
        &self,
        title: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        h_flex()
            .w_full()
            .justify_between()
            .gap_2()
            .child(title)
            .when(!self.busy, |row| {
                row.child(
                    button("dialog-close")
                        .ghost()
                        .small()
                        .icon(IconName::Close)
                        .accessibility_label("Close dialog")
                        .on_click(
                            cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx)),
                        ),
                )
            })
    }

    /// Review of an import plan.
    fn import_plan_dialog(
        &self,
        dialog: Dialog,
        plan: &ImportPlan,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let busy = self.busy;
        let plan_id = plan.id.clone();
        let mut body = div().flex().flex_col().gap_2().child(format!(
            "Review {} selected sources before copying them into the managed library.",
            plan.items.len()
        ));
        for item in &plan.items {
            body = body.child(format!(
                "{} — {:?} — {} → {} ({} bytes)",
                item.slug, item.decision, item.source, item.destination, item.total_bytes
            ));
            for message in &item.messages {
                body = body.child(format!("{:?}: {}", message.status, message.message));
            }
        }
        dialog
            .title(self.dialog_title("Review import", cx))
            .child(body)
            .footer(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        button("import-close")
                            .label("Back")
                            .inactive(busy)
                            .on_click(
                                cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx)),
                            ),
                    )
                    .child({
                        let weak = cx.entity().downgrade();
                        button("import-apply")
                            .label("Import")
                            .primary()
                            .inactive(busy)
                            .on_click(move |_, _, cx| {
                                let _ = weak.update(cx, |view, cx| {
                                    let client = view.client.clone();
                                    let plan_id = plan_id.clone();
                                    view.run(
                                        async move {
                                            client
                                                .apply_import(plan_id)
                                                .await
                                                .map(|_| Effect::CloseDialog)
                                        },
                                        cx,
                                    );
                                });
                            })
                    }),
            )
    }

    fn build_dialog(&self, dialog: Dialog, cx: &mut Context<Self>) -> Dialog {
        match &self.dialog {
            DialogState::None => dialog,
            DialogState::ImportPlan(plan) => self.import_plan_dialog(dialog, plan, cx),
            DialogState::DeletePreset(preview) => self.delete_preset_dialog(dialog, preview, cx),
            DialogState::Copies(skill) => self.copies_dialog(dialog, skill.as_ref(), cx),
            DialogState::RemoveWorkspace(id) => self.remove_workspace_dialog(dialog, id, cx),
            DialogState::DeleteFolder { id, revision } => {
                self.delete_folder_dialog(dialog, id, revision, cx)
            }
        }
    }
}
