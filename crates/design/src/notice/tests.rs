//! Notices muted, in each status and size, turning, and blocking, drawn in a GPUI test window.

use super::{Notice, NoticeSize};
use crate::Status;
use crate::test_window::{Drawn, labels, named, node};
use gpui_kit::{IntoElement, ParentElement, Role, Styled, TestAppContext, div, px};

#[gpui_kit::test]
fn a_notice_reads_its_text_in_every_status_and_size(cx: &mut TestAppContext) {
    let frame = Drawn::show(cx, |_, _| {
        let statuses = Status::ALL
            .map(|status| Notice::new(format!("{status:?}"), "The list changed").status(status));
        div()
            .flex()
            .flex_col()
            .w(px(300.))
            .children(statuses)
            .children(NoticeSize::ALL.map(|size| {
                Notice::new(format!("muted-{size:?}"), "Scanning…")
                    .turning("loading")
                    .size(size)
            }))
            .into_any_element()
    })
    .frame(cx);
    assert_eq!(labels(&frame, "The list changed"), Status::ALL.len());
    assert_eq!(labels(&frame, "Scanning…"), NoticeSize::ALL.len());
    // A line's height is its level's line box: caption 12 × 1.4, body-sm 13 × 1.4.
    let height = |id| node(&frame, id).map(|node| node.bounds().size.height);
    assert!(height("muted-Sm") < height("muted-Md"));
}

#[gpui_kit::test]
fn a_blocking_notice_is_an_alert(cx: &mut TestAppContext) {
    let frame = Drawn::show(cx, |_, _| {
        Notice::blocking(
            "error",
            Status::Danger,
            "Couldn't save",
            "Check the disk (Diagnostic 7)",
        )
        .into_any_element()
    })
    .frame(cx);
    assert_eq!(named(&frame, Role::Alert).len(), 1);
}
