use super::{Effect, NativeView};
use gpui_kit::component::input::Input;
use gpui_kit::{Context, IntoElement, ParentElement, Styled, div, px};
use skillbinder_design::tokens::space;
use skillbinder_design::{Button, KeyValueList};
use skillbinder_proto::*;

impl NativeView {
    /// Runs one Git operation, marking the view as syncing until it settles.
    pub(super) fn synchronize(&mut self, action: SyncAction, cx: &mut Context<Self>) {
        if self.blocked() {
            return;
        }
        self.syncing = Some(action);
        let client = self.client.clone();
        self.run(
            async move { client.synchronize(action).await.map(|_| Effect::None) },
            cx,
        );
    }

    pub(super) fn render_sync(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let page = div().flex().flex_col().gap(px(space::XL));
        if self.git_missing() {
            return page.child("Sync needs Git.");
        }
        let sync = &self.snapshot.sync;
        let status = format!(
            "{:?} · {} ahead · {} behind{}",
            sync.state,
            sync.ahead,
            sync.behind,
            if sync.uncommitted {
                " · local changes"
            } else {
                ""
            }
        );
        let mut facts = KeyValueList::new("sync-facts").row("Status", status);
        if let Some(remote) = &sync.remote {
            facts = facts
                .mono("Remote", remote.url.clone())
                .row("Branch", remote.branch.clone());
            if let Some(refreshed) = sync.refreshed_at {
                facts = facts.row("Last checked", refreshed.to_string());
            }
        }
        let mut page = page.child(facts);
        if sync.remote.is_some() {
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
                    Button::new(id)
                        .label(label)
                        .inactive(self.busy || !enabled)
                        .on_click(cx.listener(move |view, _, _, cx| view.synchronize(action, cx))),
                );
            }
        } else {
            page = page
                .child("Connect a Git remote to synchronize the managed library.")
                .child(Input::new(&self.remote_url))
                .child(Input::new(&self.remote_branch))
                .child(
                    Button::new("sync-connect")
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
