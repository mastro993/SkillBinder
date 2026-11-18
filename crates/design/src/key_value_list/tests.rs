//! A key-value list with text, monospace, and element values, drawn in a GPUI test window.

use super::KeyValueList;
use crate::Button;
use crate::test_window::{Drawn, labels, read};
use gpui_kit::{IntoElement, Role, TestAppContext};

#[gpui_kit::test]
fn each_row_reads_its_label_and_value(cx: &mut TestAppContext) {
    let frame = Drawn::show(cx, |_, _| {
        KeyValueList::new("facts")
            .mono("Managed library", "~/Library/SkillBinder")
            .row("Storage writable", "Yes")
            .element("Logs", Button::new("reveal").small().label("Reveal logs"))
            .into_any_element()
    })
    .frame(cx);
    for text in [
        "Managed library",
        "~/Library/SkillBinder",
        "Storage writable",
        "Yes",
        "Logs",
    ] {
        assert_eq!(labels(&frame, text), 1, "{text}");
    }
    assert_eq!(
        read(&frame, "reveal"),
        (Role::Button, Some("Reveal logs".into()), false)
    );
}
