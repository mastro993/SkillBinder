//! A dialog header's title, description or detail, and close button, drawn in a GPUI test
//! window.

use super::DialogHeader;
use crate::Text;
use crate::test_window::{Drawn, labels, read};
use gpui_kit::{InteractiveElement, IntoElement, ParentElement, Role, Styled, TestAppContext, div};

#[gpui_kit::test]
fn a_header_reads_its_title_description_and_close_button(cx: &mut TestAppContext) {
    let frame = Drawn::show(cx, |_, _| {
        div()
            .flex()
            .flex_col()
            .child(
                DialogHeader::new("Remove workspace")
                    .description("Files in the folder are not touched.")
                    .close("close", "Close dialog", |_, _, _| {}),
            )
            .child(DialogHeader::custom(div().child("Add skill")))
            .child(
                // Its own scope, since a second header's title has the same id.
                div().id("update").child(
                    DialogHeader::new("Update pdf")
                        .detail(Text::labeled("detail", "2 files changed")),
                ),
            )
            .into_any_element()
    })
    .frame(cx);
    assert_eq!(labels(&frame, "Remove workspace"), 1);
    assert_eq!(labels(&frame, "Files in the folder are not touched."), 1);
    assert_eq!(labels(&frame, "2 files changed"), 1);
    assert_eq!(
        read(&frame, "close"),
        (Role::Button, Some("Close dialog".into()), false)
    );
}
