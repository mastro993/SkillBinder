//! Dialogs framed at each size, opened over a GPUI test window.

use super::{DialogFrame, DialogSize};
use crate::{DialogFooter, DialogHeader};
use gpui_kit::component::{Root, WindowExt};
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

/// The width a dialog of `dialog_size` is drawn at in a 1000 by 800 window.
fn width(cx: &mut TestAppContext, dialog_size: DialogSize) -> f32 {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::init(cx);
        cx.set_reduce_motion(true);
    });
    let (_, cx) = cx.add_window_view(|window, cx| {
        let host = cx.new(|_| Host);
        Root::new(host, window, cx)
    });
    let cx: &mut VisualTestContext = cx;
    cx.simulate_resize(size(px(1000.), px(800.)));
    cx.update(|window, cx| {
        window.open_dialog(cx, move |dialog, _, cx| {
            DialogFrame::new(dialog_size)
                .header(DialogHeader::new("Delete preset"))
                .footer(DialogFooter::new())
                .apply(dialog, cx)
                .child("Its skills stay in the library.")
        })
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let bounds = cx
        .debug_bounds("dialog-0")
        .unwrap_or_else(|| panic!("the dialog was not painted"));
    f32::from(bounds.size.width)
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
