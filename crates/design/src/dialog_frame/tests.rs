//! Dialogs framed at each size, opened over a GPUI test window.

use super::{DialogFrame, DialogSize};
use crate::{DialogFooter, DialogHeader};
use gpui_kit::component::{Root, WindowExt, dialog::Dialog, v_flex};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    App, AppContext, Context, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, TestAppContext, VisualTestContext, Window, div, px, size,
};

struct Host;

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
    }
}

/// A 1000 by 800 window with a dialog of `dialog_size` open, whose header has a close button.
fn open(cx: &mut TestAppContext, dialog_size: DialogSize) -> &mut VisualTestContext {
    open_framed(cx, move |dialog, cx| {
        let header =
            DialogHeader::new("Delete preset").close("dialog-close", "Close dialog", |_, _, _| {});
        DialogFrame::new(dialog_size)
            .header(header)
            .footer(DialogFooter::new())
            .apply(dialog, cx)
            .child("Its skills stay in the library.")
    })
}

/// A 1000 by 800 window with the dialog `build` frames open.
fn open_framed(
    cx: &mut TestAppContext,
    build: impl Fn(Dialog, &mut App) -> Dialog + 'static,
) -> &mut VisualTestContext {
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
    cx.update(|window, cx| window.open_dialog(cx, move |dialog, _, cx| build(dialog, cx)));
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

#[gpui_kit::test]
fn a_body_fills_the_dialog_and_its_list_scrolls_inside_it(cx: &mut TestAppContext) {
    let viewport = size(px(1000.), px(800.));
    let cx = open_framed(cx, move |dialog, cx| {
        let list = div()
            .id("list")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .child(div().h(px(2000.)));
        let under = div().debug_selector(|| "under".to_owned()).h(px(20.));
        let body = v_flex().flex_1().min_h_0().child(list).child(under);
        DialogFrame::new(DialogSize::Window(viewport))
            .header(DialogHeader::new("Add skill"))
            .body(body)
            .apply(dialog, cx)
    });
    let dialog = cx
        .debug_bounds("dialog-0")
        .unwrap_or_else(|| panic!("the dialog was not painted"));
    let under = cx
        .debug_bounds("under")
        .unwrap_or_else(|| panic!("the line under the list was not painted"));
    assert!(
        under.bottom() <= dialog.bottom(),
        "the line under the list ends at {:?}, below the dialog's {:?}",
        under.bottom(),
        dialog.bottom()
    );
}

#[gpui_kit::test]
fn a_flush_body_reaches_the_dialogs_side_edges(cx: &mut TestAppContext) {
    let viewport = size(px(1000.), px(800.));
    let cx = open_framed(cx, move |dialog, cx| {
        let body = div()
            .debug_selector(|| "body".to_owned())
            .flex_1()
            .min_h_0();
        DialogFrame::new(DialogSize::Window(viewport))
            .header(DialogHeader::new("Import skills"))
            .flush_body(body)
            .apply(dialog, cx)
    });
    let dialog = cx
        .debug_bounds("dialog-0")
        .unwrap_or_else(|| panic!("the dialog was not painted"));
    let body = cx
        .debug_bounds("body")
        .unwrap_or_else(|| panic!("the body was not painted"));
    // Inside the dialog's 1px border.
    let border = px(1.);
    assert_eq!(body.left(), dialog.left() + border);
    assert_eq!(body.right(), dialog.right() - border);
}
