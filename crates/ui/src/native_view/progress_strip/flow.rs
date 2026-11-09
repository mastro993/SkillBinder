//! How the strip runs in the view: blocking the app for a change, revealing the strip, ending it
//! once the change lands, and the Done pill's Review and Close.

use super::super::copies_review::Review;
use super::super::copy_dialogs::CopiesListing;
use super::super::{DialogState, Effect, NativeView};
use super::{Activity, Finish, Landing, REVEAL_AFTER, Shown, Strip, WASH_FADE};
use gpui_kit::{Context, Window};
use skillbinder_proto::AppResult;
use std::{
    future::Future,
    time::{Duration, Instant},
};

impl NativeView {
    /// Whether the app takes no new change: one is in flight, or the strip is up.
    pub(in crate::native_view) fn blocked(&self) -> bool {
        self.busy || self.strip.is_some()
    }

    /// Runs `operation`, a change that can write many copies, blocking the app until it lands and
    /// the strip ends.
    pub(in crate::native_view) fn block<F>(
        &mut self,
        activity: Activity,
        operation: F,
        cx: &mut Context<Self>,
    ) where
        F: Future<Output = AppResult<Landing>> + Send + 'static,
    {
        if self.blocked() {
            return;
        }
        self.menu = None;
        self.run(async move { operation.await.map(Effect::Activated) }, cx);
        let strip = Strip::new(activity, Instant::now());
        let started = strip.started();
        self.strip = Some(strip);
        self.at_strip_time(REVEAL_AFTER, started, Self::reveal_strip, cx);
    }

    /// Keeps keyboard focus on the overlay while the strip or the review list is up, and gives it
    /// back to the window once they are gone.
    pub(in crate::native_view) fn sync_overlay_focus(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let up = self.strip.is_some()
            || matches!(
                self.dialog,
                DialogState::Copies(CopiesListing::Attention(_))
            );
        if up && !self.overlay_focus.is_focused(window) {
            window.focus(&self.overlay_focus, cx);
        } else if !up && self.overlay_focus.is_focused(window) {
            window.focus(&self.focus, cx);
        }
    }

    /// Runs `step` after `delay`, while the strip that started at `started` is still up.
    fn at_strip_time(
        &self,
        delay: Duration,
        started: Instant,
        step: fn(&mut Self, &mut Context<Self>),
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update(cx, |view, cx| {
                if view.strip.as_ref().map(Strip::started) == Some(started) {
                    step(view, cx);
                }
            });
        })
        .detach();
    }

    pub(in crate::native_view) fn reveal_strip(&mut self, cx: &mut Context<Self>) {
        if self.reveal_held() {
            return;
        }
        if let Some(strip) = self.strip.as_mut() {
            strip.reveal(Instant::now());
            cx.notify();
        }
    }

    /// The blocking change landed: the strip ends once it has shown long enough.
    pub(in crate::native_view) fn land_strip(&mut self, landing: Landing, cx: &mut Context<Self>) {
        if let Some(strip) = self.strip.as_mut() {
            let ends = strip.land(landing, Instant::now());
            self.end_strip_at(ends, cx);
        }
    }

    /// The blocking change was refused: the strip closes without a toast once it has shown long
    /// enough, and the error shows in the error box.
    pub(in crate::native_view) fn fail_strip(&mut self, cx: &mut Context<Self>) {
        if let Some(strip) = self.strip.as_mut() {
            let ends = strip.fail(Instant::now());
            self.end_strip_at(ends, cx);
        }
    }

    pub(in crate::native_view) fn end_strip_at(&mut self, ends: Instant, cx: &mut Context<Self>) {
        let Some(started) = self.strip.as_ref().map(Strip::started) else {
            return;
        };
        match ends.checked_duration_since(Instant::now()) {
            Some(wait) if !wait.is_zero() => self.at_strip_time(wait, started, Self::end_strip, cx),
            _ => self.end_strip(cx),
        }
    }

    fn end_strip(&mut self, cx: &mut Context<Self>) {
        if self.strip_held() {
            return;
        }
        let Some(strip) = self.strip.as_mut() else {
            return;
        };
        match strip.finish(Instant::now()) {
            Finish::Toast(message) => {
                self.take_strip(cx);
                self.show_toast(message, Some("check-circle"), cx);
                self.prompt_copies();
            }
            Finish::Closed => {
                self.take_strip(cx);
                self.prompt_copies();
            }
            Finish::Done => {}
        }
        cx.notify();
    }

    /// Takes the strip down at once, so input reaches the app again; a wash that showed fades out
    /// over [`WASH_FADE`] unless motion is reduced.
    fn take_strip(&mut self, cx: &mut Context<Self>) {
        let Some(strip) = self.strip.take() else {
            return;
        };
        if !strip.washed() || cx.reduce_motion() {
            return;
        }
        let fading = Instant::now();
        self.wash_fade = Some(fading);
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(WASH_FADE).await;
            let _ = this.update(cx, |view, cx| {
                if view.wash_fade == Some(fading) {
                    view.wash_fade = None;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Closes the Done pill, opening the review of the copies that need the user when `review` is
    /// true. Either way the copies that ask count as asked, so the Skill copies dialog does not
    /// open by itself right after.
    pub(in crate::native_view) fn close_strip(&mut self, review: bool, cx: &mut Context<Self>) {
        if !matches!(self.strip.as_ref().map(Strip::shown), Some(Shown::Done(_))) {
            return;
        }
        self.take_strip(cx);
        let copies = self
            .snapshot
            .library
            .as_ref()
            .map(|library| library.copies.clone())
            .unwrap_or_default();
        for copy in copies.iter().filter(|copy| copy.asks()) {
            let asked = (
                copy.directory.clone(),
                copy.skill.clone(),
                copy.state.clone(),
            );
            if !self.asked_copies.contains(&asked) {
                self.asked_copies.push(asked);
            }
        }
        let listing = Review::of(&copies);
        // Copies settled meanwhile, such as by a scan's retry, leave nothing to review.
        if review && !listing.is_empty() {
            self.dialog = DialogState::Copies(CopiesListing::Attention(listing));
        }
        cx.notify();
    }
}

#[cfg(not(feature = "native-test"))]
impl NativeView {
    /// Whether a native-test hold keeps the strip from ending.
    fn strip_held(&self) -> bool {
        false
    }

    /// Whether a native-test hold keeps the strip from showing.
    fn reveal_held(&self) -> bool {
        false
    }

    /// The percent a native-test hold shows instead of the change's.
    pub(in crate::native_view) fn held_percent(&self) -> Option<u8> {
        None
    }
}
