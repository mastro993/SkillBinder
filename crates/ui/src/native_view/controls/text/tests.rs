//! How assistive technology and the native-test bridge reach the app's lines of text, drawn in a
//! GPUI test window.

use std::sync::Arc;

use super::super::frames::{Drawn, labels, node, readings};
use super::super::text_button;
use gpui_kit::component::{Size, button::ButtonVariants};
use gpui_kit::{
    AccessibilityFrame, Context, InteractiveElement, IntoElement, ParentElement, Render, Role,
    StatefulInteractiveElement, Styled, TestAppContext, Text, Window, div,
};

/// A line with an id, a row whose accessible name reads its plain-string name, and a text button.
struct Lines;

impl Render for Lines {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_start()
            .child(Text::new(
                "toast-message".into(),
                "Switched to Frontend".into(),
            ))
            .child(
                div()
                    .id("skill-row-pdf-tools")
                    .role(Role::Button)
                    .aria_label("pdf-tools")
                    .child("pdf-tools"),
            )
            .child(text_button("review", "Review", Size::Small).primary())
    }
}

/// Draws [`Lines`] and returns its frame as the bridge observes it.
fn draw(cx: &mut TestAppContext) -> Arc<AccessibilityFrame> {
    Drawn::open(cx, |_, _| Lines).frame(cx)
}

#[gpui_kit::test]
fn a_line_with_an_id_reaches_the_bridge_as_one_label_holding_its_text(cx: &mut TestAppContext) {
    let frame = draw(cx);
    let Some(line) = node(&frame, "toast-message") else {
        panic!("the bridge has no toast-message");
    };
    assert_eq!(line.content_text(), "Switched to Frontend");
    let label = frame.accessibility_node(line);
    assert_eq!(label.map(|label| label.role()), Some(Role::Label));
    assert_eq!(
        label.and_then(|label| label.value()),
        Some("Switched to Frontend")
    );
    assert_eq!(labels(&frame, "Switched to Frontend"), 1);
    assert_eq!(readings(&frame, "Switched to Frontend"), 1);
}

#[gpui_kit::test]
fn text_an_enclosing_name_reads_is_read_once(cx: &mut TestAppContext) {
    let frame = draw(cx);
    let named = |id: &str| {
        let node = node(&frame, id)?;
        Some((
            node.content_text().to_owned(),
            frame.accessibility_node(node)?.label()?.to_owned(),
        ))
    };
    let row = named("skill-row-pdf-tools");
    assert_eq!(
        row.as_ref().map(|(_, name)| name.as_str()),
        Some("pdf-tools")
    );
    assert_eq!(labels(&frame, "pdf-tools"), 0, "the row's name is no label");
    assert_eq!(readings(&frame, "pdf-tools"), 1);
    let button = named("review");
    assert_eq!(button, Some(("Review".to_owned(), "Review".to_owned())));
    assert_eq!(labels(&frame, "Review"), 0, "the button's text is no label");
    assert_eq!(readings(&frame, "Review"), 1);
}
