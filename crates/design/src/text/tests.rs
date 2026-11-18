//! How assistive technology and the native-test bridge reach a line of text, and the line box
//! each level sets, drawn in a GPUI test window.

use crate::test_window::{Drawn, labels, node, readings};
use crate::tokens::text;
use crate::{Button, Text, TypeLevel};
use gpui_kit::{
    AbsoluteLength, DefiniteLength, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled, TestAppContext, div, px,
};

/// A labeled line, a row whose accessible name reads its plain-string name, and a text button.
fn lines(cx: &mut TestAppContext) -> std::sync::Arc<gpui_kit::AccessibilityFrame> {
    Drawn::show(cx, |_, _| {
        div()
            .flex()
            .flex_col()
            .items_start()
            .child(Text::labeled("toast-message", "Switched to Frontend").muted())
            .child(
                div()
                    .id("skill-row-pdf-tools")
                    .role(Role::Button)
                    .aria_label("pdf-tools")
                    .child(Text::new("pdf-tools").level(text::TITLE_SM)),
            )
            .child(Button::new("review").label("Review").small().primary())
            .into_any_element()
    })
    .frame(cx)
}

#[gpui_kit::test]
fn a_labeled_line_reaches_the_bridge_as_one_label_holding_its_text(cx: &mut TestAppContext) {
    let frame = lines(cx);
    let Some(line) = node(&frame, "toast-message") else {
        panic!("the bridge has no toast-message");
    };
    assert_eq!(line.content_text(), "Switched to Frontend");
    let label = frame.accessibility_node(line);
    assert_eq!(label.map(|label| label.role()), Some(Role::Label));
    assert_eq!(labels(&frame, "Switched to Frontend"), 1);
    assert_eq!(readings(&frame, "Switched to Frontend"), 1);
}

#[gpui_kit::test]
fn text_an_enclosing_name_reads_is_read_once(cx: &mut TestAppContext) {
    let frame = lines(cx);
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

/// The line height `level` sets on an element.
fn line_height(level: TypeLevel) -> Option<DefiniteLength> {
    let mut element = level.apply(div());
    element.text_style().line_height
}

#[test]
fn every_level_sets_its_line_box() {
    for (name, level) in text::ALL {
        let expected = px(level.size * level.line_height);
        assert_eq!(
            line_height(*level),
            Some(DefiniteLength::Absolute(AbsoluteLength::Pixels(expected))),
            "{name}"
        );
    }
    // Titles and the avatar overflow are tighter than the 1.4 the other levels use.
    assert_eq!(text::TITLE_LG.line_height, 1.3);
    assert_eq!(text::MICRO.line_height, 1.2);
    assert_eq!(text::BODY_MD.line_height, 1.4);
}

#[test]
fn the_medium_weight_is_520_so_gpui_draws_sf_pro_medium() {
    for level in [text::LABEL_MD, text::LABEL_SM, text::CAPTION_SM] {
        assert_eq!(level.weight.0, 520.);
    }
}
