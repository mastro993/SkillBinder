//! Dialogs framed at each size, opened over a GPUI test window.

use super::{DialogFrame, DialogSize};
use crate::{DialogFooter, DialogHeader};
use gpui_kit::component::{Root, WindowExt};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AppContext, Context, IntoElement, ParentElement, Render, Styled, TestAppContext,
    VisualTestContext, Window, div, px, size,
};

struct Host;

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
    }
}

/// A 1000 by 800 window with a dialog of `dialog_size` open, whose header has a close button.
fn open(cx: &mut TestAppContext, dialog_size: DialogSize) -> &mut VisualTestContext {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::init(cx);
        cx.set_reduce_motion(true);
    });
    let (_, cx) = cx.add_window_view(|window, cx| {
        let host = cx.new(|_| Host);
        Root::new(host, window, cx)
    });
    cx.simulate_resize(size(px(1000.), px(800.)));
    cx.update(|window, cx| {
        window.open_dialog(cx, move |dialog, _, cx| {
            let header = DialogHeader::new("Delete preset").close(
                "dialog-close",
                "Close dialog",
                |_, _, _| {},
            );
            DialogFrame::new(dialog_size)
                .header(header)
                .footer(DialogFooter::new())
                .apply(dialog, cx)
                .child("Its skills stay in the library.")
        })
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx
}

/// The width a dialog of `dialog_size` is drawn at in a 1000 by 800 window.
fn width(cx: &mut TestAppContext, dialog_size: DialogSize) -> f32 {
    let cx = open(cx, dialog_size);
    let bounds = cx
        .debug_bounds("dialog-0")
        .unwrap_or_else(|| panic!("the dialog was not painted"));
    f32::from(bounds.size.width)
}

#[gpui_kit::test]
fn a_framed_dialog_draws_only_its_headers_close_button(cx: &mut TestAppContext) {
    let cx = open(cx, DialogSize::Standard);
    cx.update(|window, _| {
        assert!(window.try_find("dialog-close").is_some());
        // Kit's own close control, which keeps the arrow cursor.
        assert!(window.try_find("close").is_none());
    });
}

#[gpui_kit::test]
fn a_compact_dialog_is_320_wide(cx: &mut TestAppContext) {
    assert_eq!(width(cx, DialogSize::Compact), 320.);
}

#[gpui_kit::test]
fn a_standard_dialog_is_420_wide(cx: &mut TestAppContext) {
    assert_eq!(width(cx, DialogSize::Standard), 420.);
}

#[gpui_kit::test]
fn a_window_dialog_takes_92_percent_of_the_window(cx: &mut TestAppContext) {
    let viewport = size(px(1000.), px(800.));
    assert_eq!(width(cx, DialogSize::Window(viewport)), 920.);
}
