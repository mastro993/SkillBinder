//! The Sync page (Y1): while a remote is set, the status with one next action, the remote's
//! facts, with this device's name, under it, the skills not synced yet, and History; otherwise
//! the setup column, or what to do without Git (Y10). A Sync while skills changed on both sides
//! keeps the side chosen for each (Y9).

mod copy;
mod dialogs;
mod history;
mod pending;
mod resolve;
mod restore;
mod setup;
mod status;

pub(super) use history::HistoryPages;
pub(super) use resolve::Choices;
pub(super) use restore::Restoring;

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
        if action == SyncAction::Sync && self.snapshot.sync.state == SyncState::NeedsResolve {
            return self.resolve_sync(cx);
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
        self.follow_history(cx);
        let snapshot = self.snapshot.clone();
        let discards = (snapshot.sync.local.iter())
            .map(|skill| self.discard_button(skill, cx))
            .collect();
        v_flex()
            .id("sync-page")
            .w_full()
            .max_w(px(COLUMN))
            .gap(px(space::MD))
            .children(self.sync_status(cx))
            .child(status::facts(&remote, &self.snapshot.sync.device, cx))
            .children(pending::pending(
                &self.snapshot.sync,
                &self.sync_choices,
                chrono::Utc::now(),
                discards,
                cx,
            ))
            .children(self.sync_history(cx))
            .into_any_element()
    }
}
