//! The checkbox checked, inactive, and as a tile's indicator, drawn in a GPUI test window.

use crate::Checkbox;
use crate::test_window::{Drawn, node, read};
use gpui_kit::{
    InteractiveElement, IntoElement, ParentElement, Role, StatefulInteractiveElement, Styled,
    TestAppContext, Toggled, div,
};

#[gpui_kit::test]
fn a_checkbox_reads_its_label_and_state_and_inactive_disables_it(cx: &mut TestAppContext) {
    let frame = Drawn::show(cx, |_, _| {
        div()
            .flex()
            .flex_col()
            .child(Checkbox::new("on").checked(true).label("Active"))
            .child(Checkbox::new("off").label("Inactive").inactive(true))
            .into_any_element()
    })
    .frame(cx);
    assert_eq!(
        read(&frame, "on"),
        (Role::CheckBox, Some("Active".into()), false)
    );
    assert_eq!(
        read(&frame, "off"),
        (Role::CheckBox, Some("Inactive".into()), true)
    );
    let toggled = |id| {
        let node = node(&frame, id)?;
        frame.accessibility_node(node)?.toggled()
    };
    assert_eq!(toggled("on"), Some(Toggled::True));
    assert_eq!(toggled("off"), Some(Toggled::False));
}

#[gpui_kit::test]
fn an_indicator_leaves_its_tile_the_only_checkbox(cx: &mut TestAppContext) {
    let frame = Drawn::show(cx, |_, _| {
        div()
            .id("tile")
            .role(Role::CheckBox)
            .aria_label("Cursor")
            .child(Checkbox::new("tile-check").checked(true).indicator())
            .on_click(|_, _, _| {})
            .into_any_element()
    })
    .frame(cx);
    assert_eq!(
        read(&frame, "tile"),
        (Role::CheckBox, Some("Cursor".into()), false)
    );
    let boxes = frame
        .nodes()
        .filter_map(|(_, node)| frame.accessibility_node(node))
        .filter(|node| node.role() == Role::CheckBox)
        .count();
    assert_eq!(boxes, 1, "the indicator is no checkbox of its own");
}
