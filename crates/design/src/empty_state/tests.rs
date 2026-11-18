//! Empty states with a glyph, a description, and an action, drawn in a GPUI test window.

use super::EmptyState;
use crate::Button;
use crate::test_window::{Drawn, labels, read};
use gpui_kit::{IntoElement, ParentElement, Role, Styled, TestAppContext, div};

#[gpui_kit::test]
fn an_empty_state_reads_its_title_description_and_action(cx: &mut TestAppContext) {
    let frame = Drawn::show(cx, |_, _| {
        div()
            .flex()
            .flex_col()
            .child(EmptyState::new("bare", "No skills match"))
            .child(
                EmptyState::new("full", "No skills in frontend")
                    .icon("folder")
                    .description("No agents here yet")
                    .action(Button::new("add").small().primary().label("Add skill")),
            )
            .into_any_element()
    })
    .frame(cx);
    assert_eq!(labels(&frame, "No skills match"), 1);
    assert_eq!(labels(&frame, "No skills in frontend"), 1);
    assert_eq!(labels(&frame, "No agents here yet"), 1);
    assert_eq!(
        read(&frame, "add"),
        (Role::Button, Some("Add skill".into()), false)
    );
}
