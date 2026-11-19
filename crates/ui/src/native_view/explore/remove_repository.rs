//! Removing one of your repositories (R4): a compact confirmation, then the toast "Removed
//! <owner/repo>" with Undo, which restores it with its last listing and no network. Its Git
//! cache is pruned once the toast closes. Skills installed from it stay in the library.

use super::super::toasts::ToastAction;
use super::super::{DialogState, Effect, NativeView};
use super::query::Source;
use gpui_kit::component::dialog::Dialog;
use gpui_kit::component::notification::Notification;
use gpui_kit::{Context, ParentElement};
use skillbinder_design::tokens::text;
use skillbinder_design::{Button, DialogFooter, DialogHeader, DialogSize, Text};
use skillbinder_proto::{Repository, owner_repo};

impl NativeView {
    /// The confirmation that removes `repository`.
    pub(in crate::native_view) fn remove_repository_dialog(
        &self,
        dialog: Dialog,
        repository: &Repository,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let name = owner_repo(&repository.url);
        let removed = repository.clone();
        let remove = Button::new("remove-repository-confirm")
            .label("Remove")
            .danger()
            .inactive(self.busy)
            .on_click(cx.listener(move |view, _, _, cx| {
                let client = view.client.clone();
                let repository = removed.clone();
                view.run(
                    async move {
                        client.remove_repository(repository.id.clone()).await?;
                        Ok(Effect::RepositoryRemoved(Box::new(repository)))
                    },
                    cx,
                );
            }));
        let footer = DialogFooter::new()
            .secondary(self.dialog_cancel("remove-repository-cancel", cx))
            .primary(remove);
        self.dialog_frame(
            DialogSize::Compact,
            DialogHeader::new(format!("Remove {name}?")),
            cx,
        )
        .footer(footer)
        .apply(dialog, cx)
        .child(Text::new("Installed skills stay in your library.").level(text::BODY_SM))
    }

    /// `repository` was removed: the dialog closes, Explore stops narrowing to it or showing it
    /// expanded, and the toast offers Undo.
    pub(in crate::native_view) fn repository_removed(
        &mut self,
        repository: Repository,
        cx: &mut Context<Self>,
    ) {
        self.dialog = DialogState::None;
        self.explore.expanded.remove(&repository.id);
        if self.explore.source == Some(Source::Repository(repository.id.clone())) {
            self.explore.source = None;
        }
        let message = format!("Removed {}", owner_repo(&repository.url));
        self.toast.offer(
            message.into(),
            Some(ToastAction::RestoreRepository(repository)),
            cx,
        );
    }

    /// `toast` with its Undo button, which adds `repository` back and closes the toast. Once the
    /// toast closes, by Undo, its time, or a newer toast, the repository's cache is pruned unless
    /// it is back.
    pub(in crate::native_view) fn with_restore(
        &self,
        toast: Notification,
        repository: Repository,
    ) -> Notification {
        let client = self.client.clone();
        let pruning = self.client.clone();
        toast
            .action(move |_, _, cx| {
                let (client, repository) = (client.clone(), repository.clone());
                Button::new("toast-undo")
                    .label("Undo")
                    .on_click(cx.listener(move |toast, _, window, cx| {
                        let (client, repository) = (client.clone(), repository.clone());
                        client.runtime().spawn(async move {
                            let _ = client.restore_repository(repository).await;
                        });
                        toast.dismiss(window, cx);
                    }))
                    .into_kit()
            })
            .on_close(move |_, _| pruning.prune_source_caches())
            .autohide(true)
    }
}
