//! Native-test controls: the smoke steps, and the holds a parity capture uses to wait for a strip
//! or toast state that real timing passes too fast to catch.

use super::{DialogState, NativeView, Route};
use gpui_kit::component::WindowExt;
use gpui_kit::{Context, Focusable, Window};
use skillbinder_proto::WorkspaceId;
use std::time::Instant;

/// What a native capture holds.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Holds {
    strip: Option<StripHold>,
    toast: bool,
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
    /// or ends a strip whose time is up at once, and gives a released toast 1.5 s more. Returns
    /// the holds now in place.
    pub fn hold_activation(
        &mut self,
        strip: Option<Option<StripHold>>,
        toast: Option<bool>,
        cx: &mut Context<Self>,
    ) -> (Option<StripHold>, bool) {
        let toast_released = self.holds.toast && toast == Some(false);
        let holds = Holds {
            strip: strip.unwrap_or(self.holds.strip),
            toast: toast.unwrap_or(self.holds.toast),
        };
        self.holds = holds;
        if let Some(current) = self.strip.as_ref() {
            let (due, ends) = (current.due(Instant::now()), current.ends());
            if due {
                self.reveal_strip(cx);
            }
            if let (None, Some(ends)) = (holds.strip, ends) {
                self.end_strip_at(ends, cx);
            }
        }
        if toast_released {
            self.hover_toast(false, cx);
        }
        cx.notify();
        (holds.strip, holds.toast)
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

    /// Whether a hold keeps the toast up.
    pub(super) fn toast_held(&self) -> bool {
        self.holds.toast
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
