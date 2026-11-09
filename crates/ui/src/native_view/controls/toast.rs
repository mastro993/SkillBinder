//! The app's toast: an inverted chip centered at the bottom of the window, sized to its text, that
//! closes by itself. Kit `Notification` keeps a fixed width, a popover card at the top right, a
//! close button, and its own timing, so the app draws this one. It rises 8 px into place while it
//! fades in over 200 ms, and leaves dropping 6 px while it fades out over 180 ms.

use super::app_icon;
use super::exact_text::exact_text;
use super::motion::{ease, ease_out};
use super::shadow::{self, Corners};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    Animation, AnimationExt, AnyElement, App, Div, InteractiveElement, IntoElement, ParentElement,
    SharedString, Stateful, Styled, deferred, div, prelude::FluentBuilder, px,
};
use std::time::{Duration, Instant};

/// How long a toast stays.
pub(crate) const SHOWN: Duration = Duration::from_millis(4000);
/// How long a toast stays after the pointer leaves it; hovering keeps it.
pub(crate) const AFTER_LEAVE: Duration = Duration::from_millis(1500);
/// How long a toast takes to leave.
pub(crate) const LEAVING: Duration = Duration::from_millis(180);
/// How long a toast takes to rise into place.
const RISING: Duration = Duration::from_millis(200);
/// Above the menus and the dialog layer, so a toast shows over an open dialog.
const PRIORITY: usize = gpui_kit::base::POPUP_PRIORITY + 10;

/// One toast and when it closes.
#[derive(Clone, Debug)]
pub(crate) struct Toast {
    pub message: SharedString,
    /// A retained glyph drawn in the text color before the message.
    pub icon: Option<&'static str>,
    /// Which toast this is, so one that replaces another rises anew.
    pub serial: u64,
    /// When it closes; `None` while the pointer is over it or once it is leaving.
    closes: Option<Instant>,
    /// It is leaving, and goes once it has faded out.
    leaving: bool,
}

impl Toast {
    /// Toast number `serial`, shown at `now`, closing [`SHOWN`] later.
    pub(crate) fn new(
        message: impl Into<SharedString>,
        icon: Option<&'static str>,
        serial: u64,
        now: Instant,
    ) -> Self {
        Self {
            message: message.into(),
            icon,
            serial,
            closes: Some(now + SHOWN),
            leaving: false,
        }
    }

    /// Pauses it while the pointer is over it, or gives it [`AFTER_LEAVE`] more from `now` once the
    /// pointer leaves; returns when it then closes. A leaving toast keeps leaving.
    pub(crate) fn hover(&mut self, hovered: bool, now: Instant) -> Option<Instant> {
        if self.leaving {
            return None;
        }
        self.closes = (!hovered).then(|| now + AFTER_LEAVE);
        self.closes
    }

    /// Whether a timer set for `deadline` closes it: a later hover or a newer toast moved it.
    pub(crate) fn expires_at(&self, deadline: Instant) -> bool {
        !self.leaving && self.closes == Some(deadline)
    }

    /// Starts it leaving.
    pub(crate) fn leave(&mut self) {
        self.closes = None;
        self.leaving = true;
    }

    /// Whether it is leaving.
    pub(crate) fn leaving(&self) -> bool {
        self.leaving
    }
}

/// The toast chip with id `id` over the window, centered 16 px above its bottom edge; the caller
/// adds its hover listener to the chip through `chip`.
pub(crate) fn toast_layer(
    id: &'static str,
    toast: &Toast,
    chip: impl FnOnce(Stateful<Div>) -> Stateful<Div>,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let ink = theme.background;
    let body = div()
        .id(id)
        .relative()
        .occlude()
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(10.))
        .min_h(px(36.))
        .py(px(6.))
        .px(px(12.))
        .rounded(px(8.))
        .bg(theme.foreground)
        .text_color(ink)
        .text_size(px(13.))
        .line_height(px(18.2))
        .whitespace_nowrap()
        .when_some(toast.icon, |chip, icon| {
            chip.child(app_icon(icon).size(px(15.)).flex_none().text_color(ink))
        })
        .child(
            div()
                .flex()
                .min_w_0()
                .text_ellipsis()
                .child(exact_text(toast.message.clone())),
        );
    let chip = moving(
        shadow::under_large(chip(body), 0., Corners::Radius(8.)),
        toast,
    );
    deferred(
        div()
            .absolute()
            .left_0()
            .right_0()
            .bottom(px(16.))
            .px(px(16.))
            .flex()
            .justify_center()
            .child(chip),
    )
    .with_priority(PRIORITY)
    .into_any_element()
}

/// `chip` rising into place as it fades in, or dropping as it fades out once leaving.
fn moving(chip: Div, toast: &Toast) -> AnyElement {
    let serial = toast.serial;
    if toast.leaving {
        let leaving = Animation::new(LEAVING).with_easing(ease());
        return chip
            .with_animation(("toast-leave", serial), leaving, |chip, left| {
                chip.top(px(6. * left)).opacity(1. - left)
            })
            .into_any_element();
    }
    let rising = Animation::new(RISING).with_easing(ease_out());
    chip.with_animation(("toast-rise", serial), rising, |chip, risen| {
        chip.top(px(8. * (1. - risen))).opacity(risen)
    })
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_toast_closes_after_its_time_unless_hovered() {
        let now = Instant::now();
        let mut toast = Toast::new("Switched to Work", Some("check-circle"), 0, now);
        let first = now + SHOWN;
        assert!(toast.expires_at(first));
        assert_eq!(toast.hover(true, now), None);
        assert!(!toast.expires_at(first), "hovering keeps it");
        let left = now + Duration::from_secs(10);
        let second = toast.hover(false, left);
        assert_eq!(second, Some(left + AFTER_LEAVE));
        assert!(
            !toast.expires_at(first),
            "the first timer no longer closes it"
        );
        assert!(toast.expires_at(left + AFTER_LEAVE));
        toast.leave();
        assert!(toast.leaving());
        assert!(!toast.expires_at(left + AFTER_LEAVE), "it closes once");
        assert_eq!(
            toast.hover(false, left),
            None,
            "a leaving toast keeps leaving"
        );
    }
}
