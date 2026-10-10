use super::controls::{Inactive, button};
use super::{Effect, NativeView};
use gpui_kit::component::{button::ButtonVariants, input::Input};
use gpui_kit::{Context, IntoElement, ParentElement, Styled, div};
use skillbinder_proto::*;

impl NativeView {
    pub(super) fn render_sync(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let sync = &self.snapshot.sync;
        let mut page = div().flex().flex_col().gap_4().child("Sync").child(format!(
            "Status: {:?} · {} ahead · {} behind{}",
            sync.state,
            sync.ahead,
            sync.behind,
            if sync.uncommitted {
                " · local changes"
            } else {
                ""
            }
        ));
        if let Some(remote) = &sync.remote {
            page = page.child(format!("{} · {}", remote.url, remote.branch));
            if let Some(refreshed) = sync.refreshed_at {
                page = page.child(format!("Last checked: {refreshed}"));
            }
            for (action, label, id, enabled) in [
                (SyncAction::Refresh, "Refresh", "sync-refresh", true),
                (
                    SyncAction::Pull,
                    "Pull",
                    "sync-pull",
                    sync.state == SyncState::NeedsPull && !sync.uncommitted,
                ),
                (
                    SyncAction::Push,
                    "Push",
                    "sync-push",
                    sync.state == SyncState::NeedsPush,
                ),
                (
                    SyncAction::Sync,
                    "Sync",
                    "sync-now",
                    sync.state != SyncState::Synced,
                ),
                (
                    SyncAction::Disconnect,
                    "Disconnect",
                    "sync-disconnect",
                    true,
                ),
            ] {
                page = page.child(
                    button(id)
                        .label(label)
                        .inactive(self.busy || !enabled)
                        .on_click(cx.listener(move |view, _, _, cx| {
                            let client = view.client.clone();
                            view.run(
                            async move { client.synchronize(action).await.map(|_| Effect::None) },
                            cx,
                        );
                        })),
                );
            }
        } else {
            page = page
                .child("Connect a Git remote to synchronize the managed library.")
                .child(Input::new(&self.remote_url))
                .child(Input::new(&self.remote_branch))
                .child(
                    button("sync-connect")
                        .label("Connect remote")
                        .primary()
                        .inactive(self.busy)
                        .on_click(cx.listener(|view, _, _, cx| {
                            let url = Self::input_text(&view.remote_url, cx);
                            let branch = Self::input_text(&view.remote_branch, cx);
                            let client = view.client.clone();
                            view.run(
                                async move {
                                    client
                                        .connect_remote(RemoteConfig { url, branch })
                                        .await
                                        .map(|_| Effect::None)
                                },
                                cx,
                            );
                        })),
                );
        }
        page
    }
}
