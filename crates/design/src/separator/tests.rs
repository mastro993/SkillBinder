//! Dividers across and down, inset from their ends, drawn in a GPUI test window.

use super::Separator;
use crate::test_window::Drawn;
use gpui_kit::{InteractiveElement, IntoElement, ParentElement, Styled, TestAppContext, div, px};

#[gpui_kit::test]
fn a_divider_is_one_point_thick_across_and_down(cx: &mut TestAppContext) {
    let drawn = Drawn::show(cx, |_, _| {
        div()
            .flex()
            .flex_col()
            .w(px(200.))
            .child(
                div()
                    .debug_selector(|| "across".to_owned())
                    .child(Separator::horizontal().inset(12.)),
            )
            .child(
                div().flex().h(px(40.)).child(
                    div()
                        .debug_selector(|| "down".to_owned())
                        .h_full()
                        .child(Separator::vertical()),
                ),
            )
            .into_any_element()
    });
    drawn.frame(cx);
    let across = drawn.bounds(cx, "across").map(|bounds| bounds.size.height);
    assert_eq!(across, Some(px(1.)));
    let down = drawn.bounds(cx, "down").map(|bounds| bounds.size.width);
    assert_eq!(down, Some(px(1.)));
}
