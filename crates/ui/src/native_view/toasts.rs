//! The toast that confirms a change: Kit's notification with the check-circle glyph at the bottom
//! center of the window. A newer one replaces the shown one, and Kit closes it after its time,
//! which the pointer pauses while it rests on it. A change ends from a timer that has no window
//! at hand, so the toast waits for the next frame, which pushes it.

use super::NativeView;
use super::controls::app_icon;
use gpui_kit::component::{WindowExt, notification::Notification};
use gpui_kit::{
    Anchor, Context, IntoElement, ParentElement, SharedString, Styled, Text, Window, div,
};

/// The id every change's toast shares, so a newer one replaces the shown one.
struct ChangeToast;

/// A change's toast waiting for the next frame, where the window it shows in is at hand.
#[derive(Default)]
pub(super) struct PendingToast(Option<SharedString>);

impl PendingToast {
    /// Shows `message` from the next frame of the view `cx` draws on, in place of any shown toast.
    pub(super) fn show<V: 'static>(&mut self, message: SharedString, cx: &mut Context<V>) {
        self.0 = Some(message);
        cx.notify();
    }

    /// The waiting toast as Kit's notification, with its message, once.
    pub(super) fn take(&mut self) -> Option<(Notification, SharedString)> {
        let message = self.0.take()?;
        let text = message.clone();
        // Kit's message is plain text; content keeps the message a label holding its text.
        let toast = Notification::new()
            .id::<ChangeToast>()
            .icon(app_icon("check-circle"))
            .content(move |_, _, _| {
                div()
                    .text_sm()
                    .child(Text::new("toast-message".into(), text.clone()))
                    .into_any_element()
            })
            .placement(Anchor::BottomCenter);
        Some((toast, message))
    }
}

impl NativeView {
    /// Shows `message` from the next frame on, in place of any shown toast.
    pub(super) fn show_toast(&mut self, message: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.toast.show(message.into(), cx);
    }

    /// Pushes the toast waiting for a frame, if any, onto the window's notifications.
    pub(super) fn push_toast(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((toast, message)) = self.toast.take() else {
            return;
        };
        let toast = self.hold_toast(toast, message, cx);
        window.push_notification(toast, cx);
    }
}

#[cfg(not(feature = "native-test"))]
impl NativeView {
    /// The toast as a native-test hold keeps it.
    fn hold_toast(
        &mut self,
        toast: Notification,
        _: SharedString,
        _: &mut Context<Self>,
    ) -> Notification {
        toast
    }
}

#[cfg(test)]
mod tests;
