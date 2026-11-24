//! The Git install flow's actions and checks: Install Git starts the system's installer through
//! the engine, Download Git opens Git's download page, and Copy, whose command box puts the
//! package manager's command on the clipboard, only starts the wait. Then the app checks for Git every 3 seconds, and on focus, until it is found.

use super::super::{NativeView, Route, onboarding};
use super::{Action, CHECK_EVERY, DOWNLOAD_URL, READY_FOR, Status};
use gpui_kit::Context;
use skillbinder_proto::AppSnapshot;

impl NativeView {
    /// Takes `action`, then waits on Git.
    pub(in crate::native_view) fn git_action(&mut self, action: Action, cx: &mut Context<Self>) {
        match action {
            Action::Install => self.install_git(cx),
            Action::Download => cx.open_url(DOWNLOAD_URL),
            Action::Copy => {}
        }
        self.git_setup.act(action);
        self.watch_git(cx);
        cx.notify();
    }

    /// Starts the installer; one that cannot start ends the wait and says why.
    fn install_git(&self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let started = self
            .client
            .runtime()
            .spawn(async move { client.install_git().await });
        cx.spawn(async move |this, cx| {
            let Ok(Err(error)) = started.await else {
                return;
            };
            let _ = this.update(cx, |view, cx| {
                view.git_setup.waiting = None;
                view.error = Some(error);
                cx.notify();
            });
        })
        .detach();
    }

    /// The app checks for Git while none is found and the Git step shows, or Settings waits on
    /// Git.
    pub(in crate::native_view) fn checks_git(&self) -> bool {
        let onboarding = &self.snapshot.bootstrap.onboarding;
        self.git_missing()
            && (onboarding::git_step_shown(onboarding, self.onboarding_back_to).is_some()
                || (onboarding.finished
                    && self.route == Route::Settings
                    && self.git_setup.waiting.is_some()))
    }

    /// Checks for Git every 3 seconds while [`Self::checks_git`], unless that check runs already.
    pub(in crate::native_view) fn watch_git(&mut self, cx: &mut Context<Self>) {
        if self.git_checking || !self.checks_git() {
            return;
        }
        self.git_checking = true;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(CHECK_EVERY).await;
                let checking = this.update(cx, |view, _| {
                    view.git_checking = view.checks_git();
                    if view.git_checking {
                        view.client.check_git();
                    }
                    view.git_checking
                });
                if !matches!(checking, Ok(true)) {
                    break;
                }
            }
        })
        .detach();
    }

    /// Follows Git being found in `latest`, before it replaces the shown snapshot: the Git step
    /// stays up reading "Git <version> is ready" for a moment, then moves on by itself (T6);
    /// Settings, waiting on it, says so in a toast.
    pub(in crate::native_view) fn follow_git(
        &mut self,
        latest: &AppSnapshot,
        cx: &mut Context<Self>,
    ) {
        let Some(version) = &latest.bootstrap.git_version else {
            return;
        };
        if !self.git_missing() {
            return;
        }
        let shown = onboarding::git_step_shown(
            &self.snapshot.bootstrap.onboarding,
            self.onboarding_back_to,
        );
        if let Some(place) = shown {
            self.held_step = Some(place);
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(READY_FOR).await;
                let _ = this.update(cx, |view, cx| {
                    view.held_step = None;
                    cx.notify();
                });
            })
            .detach();
        } else if self.git_setup.waiting.is_some() && self.route == Route::Settings {
            self.show_toast(Status::Ready(version.clone()).text(), cx);
        }
        self.git_setup.waiting = None;
    }
}
