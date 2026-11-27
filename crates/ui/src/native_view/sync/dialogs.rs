//! The ⋯ menu's dialogs: Change branch… (Y11), which connects the same remote on another branch
//! and checks it; Rename this device… (Y11), which names the device later commits carry; and
//! Disconnect…, which asks first (Y7).

use super::super::{Effect, NativeView};
use super::setup::field;
use gpui_kit::component::dialog::Dialog;
use gpui_kit::component::v_flex;
use gpui_kit::{Context, IntoElement, ParentElement, Styled, px};
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
            .child(field_with_note(
                field("Branch", &self.remote_branch, working),
                "sync-branch-note",
                "SkillBinder checks the new branch right away. Your library does not change \
                 until you Sync.",
            ))
    }

    pub(in crate::native_view) fn rename_device_dialog(
        &self,
        dialog: Dialog,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let working = self.working();
        let rename = Button::new("sync-rename-confirm")
            .label("Rename")
            .primary()
            .loading(working)
            .inactive(working)
            .on_click(cx.listener(|view, _, _, cx| {
                let name = Self::input_text(&view.device_name, cx);
                let client = view.client.clone();
                view.run(
                    async move {
                        client
                            .rename_device(name)
                            .await
                            .map(|()| Effect::CloseDialog)
                    },
                    cx,
                );
            }));
        let footer = DialogFooter::new()
            .secondary(self.dialog_cancel("sync-rename-cancel", cx))
            .primary(rename);
        self.dialog_frame(
            DialogSize::Compact,
            DialogHeader::new("Rename this device"),
            cx,
        )
        .footer(footer)
        .apply(dialog, cx)
        .child(field_with_note(
            field("Name", &self.device_name, working),
            "sync-rename-note",
            "Changes you push from now on carry this name. Earlier ones keep theirs. It is \
             visible to anyone who can read the remote.",
        ))
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

/// A dialog's one labeled field, with a muted note under it.
fn field_with_note(
    field: impl IntoElement,
    id: &'static str,
    note: &'static str,
) -> impl IntoElement {
    v_flex()
        .gap(px(space::MD))
        .child(field)
        .child(Text::labeled(id, note).level(text::CAPTION).muted())
}
