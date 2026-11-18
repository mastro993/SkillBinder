//! The spinner at each size, drawn in a GPUI test window.

use super::Spinner;
use crate::IconSize;
use crate::test_window::Drawn;
use gpui_kit::{InteractiveElement, IntoElement, ParentElement, Styled, TestAppContext, div, px};

#[gpui_kit::test]
fn a_spinner_takes_its_size_and_any_glyph(cx: &mut TestAppContext) {
    let drawn = Drawn::show(cx, |_, _| {
        div()
            .flex()
            .items_start()
            .child(
                div()
                    .flex_none()
                    .debug_selector(|| "loading".to_owned())
                    .child(Spinner::new(IconSize::Sm)),
            )
            .child(
                div()
                    .flex_none()
                    .debug_selector(|| "refresh".to_owned())
                    .child(Spinner::new(IconSize::Xs).glyph("refresh-03")),
            )
            .into_any_element()
    });
    drawn.frame(cx);
    let mut width = |selector| drawn.bounds(cx, selector).map(|bounds| bounds.size.width);
    assert_eq!(width("loading"), Some(px(IconSize::Sm.points())));
    assert_eq!(width("refresh"), Some(px(IconSize::Xs.points())));
}
