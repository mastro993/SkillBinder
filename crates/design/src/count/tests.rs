//! A count's figures and label, drawn in a GPUI test window.

use super::format_count;
use crate::Text;
use crate::test_window::{Drawn, labels, readings};
use gpui_kit::{
    InteractiveElement, IntoElement, ParentElement, Role, StatefulInteractiveElement, Styled,
    TestAppContext, div,
};

#[test]
fn counts_group_thousands() {
    assert_eq!(format_count(0), "0");
    assert_eq!(format_count(999), "999");
    assert_eq!(format_count(5423), "5,423");
    assert_eq!(format_count(1_234_567), "1,234,567");
}

#[gpui_kit::test]
fn a_labeled_count_is_one_label_and_a_plain_one_is_read_by_its_row(cx: &mut TestAppContext) {
    let frame = Drawn::show(cx, |_, _| {
        div()
            .flex()
            .child(Text::labeled_count("count", 1234))
            .child(
                div()
                    .id("row")
                    .role(Role::Button)
                    .aria_label("Frontend, 12 skills")
                    .child(Text::count(12)),
            )
            .into_any_element()
    })
    .frame(cx);
    assert_eq!(labels(&frame, "1,234"), 1);
    assert_eq!(readings(&frame, "12"), 0, "the row's name reads the count");
}
