//! Native-test controls: the smoke steps, and the holds a parity capture uses to wait for a strip
//! or toast state that real timing passes too fast to catch.

use super::{DialogState, NativeView, Route};
use gpui_kit::component::{WindowExt, notification::Notification};
use gpui_kit::{Context, Focusable, SharedString, Window};
use skillbinder_proto::WorkspaceId;
use std::time::Instant;

/// What a native capture holds.
#[derive(Clone, Debug, Default)]
pub(super) struct Holds {
    strip: Option<StripHold>,
    toast: bool,
    /// The message of the toast the toast hold keeps up, until it closes.
    held_toast: Option<SharedString>,
}

/// How a held progress strip shows. A held strip never ends.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StripHold {
    /// Only the transparent layer that blocks input: the wash and the pill do not show.
    Hidden,
    /// The pill shows this percent once revealed, whatever the change has done.
    Percent(u8),
}

impl NativeView {
    /// Holds the progress strip as `strip` says, or releases it with `Some(None)`, and holds the
    /// end toast while `toast` is `Some(true)`; `None` leaves that hold as it is. Releasing shows
    /// or ends a strip whose time is up at once, and pushes a held toast again, so it closes by
    /// itself after Kit's time. Returns the holds now in place.
    pub fn hold_activation(
        &mut self,
        strip: Option<Option<StripHold>>,
        toast: Option<bool>,
        cx: &mut Context<Self>,
    ) -> (Option<StripHold>, bool) {
        if let Some(strip) = strip {
            self.holds.strip = strip;
        }
        if let Some(toast) = toast {
            self.holds.toast = toast;
        }
        let (strip, toast) = (self.holds.strip, self.holds.toast);
        if let Some(current) = self.strip.as_ref() {
            let (due, ends) = (current.due(Instant::now()), current.ends());
            if due {
                self.reveal_strip(cx);
            }
            if let (None, Some(ends)) = (strip, ends) {
                self.end_strip_at(ends, cx);
            }
        }
        if !toast && let Some(message) = self.holds.held_toast.take() {
            self.show_toast(message, cx);
        }
        cx.notify();
        (strip, toast)
    }

    /// Whether a hold keeps the strip from ending.
    pub(super) fn strip_held(&self) -> bool {
        self.holds.strip.is_some()
    }

    /// Whether a hold keeps the strip from showing.
    pub(super) fn reveal_held(&self) -> bool {
        self.holds.strip == Some(StripHold::Hidden)
    }

    /// The percent a hold shows instead of the change's.
    pub(super) fn held_percent(&self) -> Option<u8> {
        match self.holds.strip {
            Some(StripHold::Percent(percent)) => Some(percent.min(100)),
            _ => None,
        }
    }

    /// `toast`, showing `message`, as the toast hold keeps it: while the hold is on, it does not
    /// close by itself, and releasing the hold pushes it again unless it was closed meanwhile.
    pub(super) fn hold_toast(
        &mut self,
        toast: Notification,
        message: SharedString,
        cx: &mut Context<Self>,
    ) -> Notification {
        if !self.holds.toast {
            return toast;
        }
        self.holds.held_toast = Some(message);
        let view = cx.entity().downgrade();
        toast.autohide(false).on_close(move |_, cx| {
            let _ = view.update(cx, |view, _| view.holds.held_toast = None);
        })
    }

    /// Exercises native rendering and public GPUI Kit editing and modal APIs.
    pub fn smoke_step(
        &mut self,
        step: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<bool, String> {
        match step {
            0 => {
                if Self::input_text(&self.remote_branch, cx) != "main" {
                    return Err("New remote branch must default to main".into());
                }
                self.navigate(Route::Sync, window, cx);
                self.remote_url.update(cx, |input, cx| {
                    input.set_value("Fixture café 日本語", window, cx)
                });
                window.focus(&self.remote_url.read(cx).focus_handle(cx), cx);
            }
            1 => {
                if Self::input_text(&self.remote_url, cx) != "Fixture café 日本語"
                    || !self.remote_url.read(cx).focus_handle(cx).is_focused(window)
                {
                    return Err("Native text or focus was lost".into());
                }
                self.navigate(Route::Settings, window, cx);
            }
            2 => self.navigate(Route::Library, window, cx),
            3 => self.navigate(Route::Discovery, window, cx),
            4 => {
                window.focus(&self.focus, cx);
                self.dialog = DialogState::RemoveWorkspace(WorkspaceId::new());
            }
            5 => {
                if !window.has_active_dialog(cx) || self.focus.is_focused(window) {
                    return Err("Native dialog did not take focus".into());
                }
                window.dispatch_action(
                    Box::new(gpui_kit::base::actions::Confirm { secondary: false }),
                    cx,
                );
            }
            6 => {
                if !window.has_active_dialog(cx) || matches!(self.dialog, DialogState::None) {
                    return Err("Dialog confirmation bypassed the explicit action".into());
                }
                self.dialog = DialogState::None;
                self.sync_dialog(window, cx);
            }
            7 => {
                if window.has_active_dialog(cx) || !self.focus.is_focused(window) {
                    return Err("Native dialog did not restore focus".into());
                }
            }
            _ => return Ok(true),
        }
        cx.notify();
        Ok(false)
    }
}
