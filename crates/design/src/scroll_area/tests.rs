//! A scroll view holding more rows than it shows, drawn in a GPUI test window.

use super::ScrollArea;
use crate::Text;
use crate::test_window::{Drawn, labels, readings};
use gpui_kit::{
    IntoElement, ParentElement, StatefulInteractiveElement, Styled, TestAppContext, px,
};

#[gpui_kit::test]
fn a_view_scrolls_its_rows_under_its_name(cx: &mut TestAppContext) {
    let frame = Drawn::show(cx, |_, _| {
        ScrollArea::new("found")
            .aria_label("Skills found")
            .w(px(200.))
            .h(px(60.))
            .children((0..10).map(|row| Text::labeled(format!("row-{row}"), format!("row {row}"))))
            .into_any_element()
    })
    .frame(cx);
    assert_eq!(readings(&frame, "Skills found"), 1);
    assert_eq!(labels(&frame, "row 0"), 1);
}
