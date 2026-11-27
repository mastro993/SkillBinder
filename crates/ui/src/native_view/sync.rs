//! The Sync page (Y1): while a remote is set, the status with one next action and the remote's
//! facts, with this device's name, under it; otherwise the setup column, or what to do without Git (Y10).

mod copy;
mod dialogs;
mod setup;
mod status;

use super::{Effect, NativeView};
use gpui_kit::component::v_flex;
use gpui_kit::{AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Styled, px};
use skillbinder_design::sync::COLUMN;
use skillbinder_design::tokens::space;
use skillbinder_proto::*;

impl NativeView {
    /// Runs one Git operation, which the client shows as syncing until it returns. Push, Pull,
    /// and Sync end with a toast naming the remote (Y7), and Disconnect closes its dialog. A
    /// check that fails shows on the page as Unreachable, not as an error.
    pub(super) fn synchronize(&mut self, action: SyncAction, cx: &mut Context<Self>) {
        if self.blocked() {
            return;
        }
        let client = self.client.clone();
        let address = self
            .snapshot
            .sync
            .remote
            .as_ref()
            .map(RemoteConfig::address);
        self.run(
            async move {
                match client.synchronize(action).await {
                    Ok(_) => Ok(match address.and_then(|at| copy::done(action, &at)) {
                        Some(done) => Effect::Toast(done),
                        None if action == SyncAction::Disconnect => Effect::CloseDialog,
                        None => Effect::None,
                    }),
                    Err(_)
                        if action == SyncAction::Refresh
                            && client.snapshot().sync.last_error.is_some() =>
                    {
                        Ok(Effect::None)
                    }
                    Err(error) => Err(error),
                }
            },
            cx,
        );
    }

    pub(super) fn render_sync(&mut self, cx: &mut Context<Self>) -> AnyElement {
        if self.git_missing() {
            return self.sync_needs_git(cx);
        }
        let Some(remote) = self.snapshot.sync.remote.clone() else {
            return self.sync_setup(cx);
        };
        v_flex()
            .id("sync-page")
            .w_full()
            .max_w(px(COLUMN))
            .gap(px(space::MD))
            .children(self.sync_status(cx))
            .child(status::facts(&remote, &self.snapshot.sync.device, cx))
            .into_any_element()
    }
}
