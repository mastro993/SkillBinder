//! The close preset, drawn in a GPUI test window.

use crate::Button;
use crate::test_window::{Drawn, node, read};
use gpui_kit::{IntoElement, Role, TestAppContext, px};

#[gpui_kit::test]
fn the_close_button_is_a_small_button_named_by_its_label(cx: &mut TestAppContext) {
    let frame = Drawn::show(cx, |_, _| {
        Button::close("dialog-close", "Close dialog").into_any_element()
    })
    .frame(cx);
    assert_eq!(
        read(&frame, "dialog-close"),
        (Role::Button, Some("Close dialog".into()), false)
    );
    let height = node(&frame, "dialog-close").map(|node| node.bounds().size.height);
    assert_eq!(height, Some(px(24.)));
}
