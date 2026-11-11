//! The window's one toast: a newer toast replaces the shown one, and each closes by itself after
//! its time, which the pointer pauses while it rests on it. A closing toast leaves over 180 ms,
//! unless motion is reduced.

use super::NativeView;
use super::controls::toast::{LEAVING, SHOWN, Toast, toast_layer};
use gpui_kit::{AnyElement, Context, SharedString, StatefulInteractiveElement};
use std::time::Instant;

impl NativeView {
    /// Shows `message`, after the retained glyph `icon` if any, in place of any shown toast.
    pub(super) fn show_toast(
        &mut self,
        message: impl Into<SharedString>,
        icon: Option<&'static str>,
        cx: &mut Context<Self>,
    ) {
        let now = Instant::now();
        let serial = self
            .toast
            .as_ref()
            .map_or(0, |shown| shown.serial.wrapping_add(1));
        self.toast = Some(Toast::new(message, icon, serial, now));
        self.close_toast_at(now + SHOWN, cx);
        cx.notify();
    }

    /// Closes the shown toast at `deadline`, unless a hover or a newer toast moved it.
    fn close_toast_at(&self, deadline: Instant, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let wait = deadline.saturating_duration_since(Instant::now());
            cx.background_executor().timer(wait).await;
            let _ = this.update(cx, |view, cx| {
                if view.toast_held() {
                    return;
                }
                let Some(toast) = view.toast.as_mut() else {
                    return;
                };
                if !toast.expires_at(deadline) {
                    return;
                }
                if cx.reduce_motion() {
                    view.toast = None;
                } else {
                    let serial = toast.serial;
                    toast.leave();
                    view.remove_toast_after_leaving(serial, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Removes toast `serial` once it has left, unless a newer toast replaced it.
    fn remove_toast_after_leaving(&self, serial: u64, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(LEAVING).await;
            let _ = this.update(cx, |view, cx| {
                if view
                    .toast
                    .as_ref()
                    .is_some_and(|toast| toast.serial == serial && toast.leaving())
                {
                    view.toast = None;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Pauses the toast while the pointer rests on it, and gives it a little more time once the
    /// pointer leaves.
    pub(super) fn hover_toast(&mut self, hovered: bool, cx: &mut Context<Self>) {
        let Some(toast) = self.toast.as_mut() else {
            return;
        };
        if let Some(deadline) = toast.hover(hovered, Instant::now()) {
            self.close_toast_at(deadline, cx);
        }
    }

    /// The toast over the window, while one shows.
    pub(super) fn render_toast(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let toast = self.toast.as_ref()?;
        let hover = cx.listener(|view, hovered: &bool, _, cx| view.hover_toast(*hovered, cx));
        Some(toast_layer(
            "app-toast",
            toast,
            |chip| chip.on_hover(hover),
            cx,
        ))
    }
}

#[cfg(not(feature = "native-test"))]
impl NativeView {
    /// Whether a native-test hold keeps the toast up.
    fn toast_held(&self) -> bool {
        false
    }
}
