//! A section header with and without its count and action, drawn in a GPUI test window.

use super::SectionHeader;
use crate::Button;
use crate::test_window::{Drawn, labels, read, readings};
use gpui_kit::{IntoElement, ParentElement, Role, Styled, TestAppContext, div};

#[gpui_kit::test]
fn a_header_reads_its_label_count_and_action(cx: &mut TestAppContext) {
    let frame = Drawn::show(cx, |_, _| {
        div()
            .flex()
            .flex_col()
            .child(SectionHeader::new("plain", "Workspaces"))
            .child(
                SectionHeader::new("counted", "Selected")
                    .count(1234)
                    .action(Button::new("add").small().label("Add")),
            )
            .child(SectionHeader::custom(div().child("Presets")))
            .into_any_element()
    })
    .frame(cx);
    assert_eq!(labels(&frame, "Workspaces"), 1);
    assert_eq!(labels(&frame, "Selected"), 1);
    // The count reads with thousands separators.
    assert_eq!(readings(&frame, "1,234"), 1);
    assert_eq!(
        read(&frame, "add"),
        (Role::Button, Some("Add".into()), false)
    );
}
