//! A dialog footer's note and its buttons in order, drawn in a GPUI test window.

use super::DialogFooter;
use crate::test_window::{Drawn, labels, named};
use crate::{Button, Text};
use gpui_kit::{IntoElement, Role, TestAppContext};

#[gpui_kit::test]
fn a_footer_reads_its_note_then_its_secondary_and_primary_buttons(cx: &mut TestAppContext) {
    let frame = Drawn::show(cx, |_, _| {
        DialogFooter::new()
            .leading(Text::labeled("note", "Import at most 50 at a time."))
            .secondary(Button::new("cancel").label("Cancel"))
            .primary(Button::new("remove").label("Remove").danger())
            .into_any_element()
    })
    .frame(cx);
    assert_eq!(labels(&frame, "Import at most 50 at a time."), 1);
    assert_eq!(
        named(&frame, Role::Button),
        vec![Some("Cancel".into()), Some("Remove".into())]
    );
}
