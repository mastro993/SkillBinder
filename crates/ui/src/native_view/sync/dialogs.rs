//! The ⋯ menu's dialogs: Change branch… (Y11), which connects the same remote on another branch
//! and checks it, and Disconnect…, which asks first (Y7).

use super::super::{Effect, NativeView};
use gpui_kit::component::dialog::Dialog;
use gpui_kit::component::input::Input;
use gpui_kit::component::v_flex;
use gpui_kit::{Context, ParentElement, Styled, px};
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{Button, DialogFooter, DialogHeader, DialogSize, Text};
use skillbinder_proto::{RemoteConfig, SyncAction};

impl NativeView {
    pub(in crate::native_view) fn change_branch_dialog(
        &self,
        dialog: Dialog,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let working = self.working();
        let change = Button::new("sync-branch-confirm")
            .label("Change")
            .primary()
            .loading(working)
            .inactive(working)
            .on_click(cx.listener(|view, _, _, cx| {
                let Some(url) = view.snapshot.sync.remote.as_ref().map(|r| r.url.clone()) else {
                    return;
                };
                let branch = Self::input_text(&view.remote_branch, cx);
                let client = view.client.clone();
                view.run(
                    async move {
                        client
                            .connect_remote(RemoteConfig { url, branch })
                            .await
                            .map(|_| Effect::CloseDialog)
                    },
                    cx,
                );
            }));
        let footer = DialogFooter::new()
            .secondary(self.dialog_cancel("sync-branch-cancel", cx))
            .primary(change);
        self.dialog_frame(DialogSize::Compact, DialogHeader::new("Change branch"), cx)
            .footer(footer)
            .apply(dialog, cx)
            .child(
                v_flex()
                    .gap(px(space::MD))
                    .child(
                        v_flex()
                            .gap(px(space::XS))
                            .child(Text::new("Branch").level(text::LABEL_SM))
                            .child(
                                Input::new(&self.remote_branch)
                                    .aria_label("Branch")
                                    .disabled(working),
                            ),
                    )
                    .child(
                        Text::labeled(
                            "sync-branch-note",
                            "SkillBinder checks the new branch right away. Your library does \
                             not change until you Sync.",
                        )
                        .level(text::CAPTION)
                        .muted(),
                    ),
            )
    }

    pub(in crate::native_view) fn disconnect_dialog(
        &self,
        dialog: Dialog,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let remote = self
            .snapshot
            .sync
            .remote
            .as_ref()
            .map_or_else(|| "the remote".to_owned(), RemoteConfig::address);
        let disconnect = Button::new("sync-disconnect-confirm")
            .label("Disconnect")
            .danger()
            .loading(self.working())
            .inactive(self.working())
            .on_click(cx.listener(|view, _, _, cx| view.synchronize(SyncAction::Disconnect, cx)));
        let footer = DialogFooter::new()
            .secondary(self.dialog_cancel("sync-disconnect-cancel", cx))
            .primary(disconnect);
        self.dialog_frame(
            DialogSize::Standard,
            DialogHeader::new(format!("Disconnect from {remote}?")),
            cx,
        )
        .footer(footer)
        .apply(dialog, cx)
        .child(
            "This device stops syncing. Your library and the remote stay as they are, and you \
             can connect again at any time.",
        )
    }
}
