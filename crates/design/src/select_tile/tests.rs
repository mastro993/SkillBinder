//! A tile as a checkbox, a radio, and with a corner, drawn in a GPUI test window: each reads its
//! name and state, and only a control is one.

use crate::test_window::{Drawn, named, node, read};
use crate::{SelectTile, Text};
use gpui_kit::{IntoElement, ParentElement, Role, Styled, TestAppContext, Toggled, div};

#[gpui_kit::test]
fn a_tile_is_a_checkbox_or_a_radio_named_after_it(cx: &mut TestAppContext) {
    let frame = Drawn::show(cx, |_, _| {
        div()
            .flex()
            .flex_col()
            .child(
                SelectTile::checkbox("cursor", "Cursor")
                    .selected(true)
                    .detail(Text::new("~/.cursor/skills"))
                    .on_select(|_, _| {}),
            )
            .child(SelectTile::checkbox("zed", "Zed").on_select(|_, _| {}))
            .child(SelectTile::radio("laptop", "Laptop").selected(true))
            .child(SelectTile::radio("here", "This device"))
            .into_any_element()
    })
    .frame(cx);
    assert_eq!(
        read(&frame, "cursor"),
        (Role::CheckBox, Some("Cursor".into()), false)
    );
    assert_eq!(
        read(&frame, "laptop"),
        (Role::RadioButton, Some("Laptop".into()), false)
    );
    let toggled = |id| {
        let node = node(&frame, id)?;
        frame.accessibility_node(node)?.toggled()
    };
    assert_eq!(toggled("cursor"), Some(Toggled::True));
    assert_eq!(toggled("zed"), Some(Toggled::False));
    assert_eq!(toggled("laptop"), Some(Toggled::True));
    assert_eq!(toggled("here"), Some(Toggled::False));
}

#[gpui_kit::test]
fn a_tile_with_a_corner_is_no_control(cx: &mut TestAppContext) {
    let frame = Drawn::show(cx, |_, _| {
        div()
            .w_full()
            .child(
                SelectTile::checkbox("claude", "Claude Code")
                    .selected(true)
                    .corner(Text::new("Connected")),
            )
            .into_any_element()
    })
    .frame(cx);
    assert!(named(&frame, Role::CheckBox).is_empty());
    assert!(node(&frame, "claude-name").is_some());
}

thread_local! {
    static SELECTED: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

#[gpui_kit::test]
fn enter_or_space_selects_a_focused_tile(cx: &mut TestAppContext) {
    let drawn = Drawn::show(cx, |_, _| {
        div()
            .size_full()
            .child(
                SelectTile::checkbox("cursor", "Cursor")
                    .on_select(|_, _| SELECTED.with(|n| n.set(n.get() + 1))),
            )
            .into_any_element()
    });
    drawn.step(cx, |window, cx| window.focus_next(cx));
    let visual = &mut gpui_kit::VisualTestContext::from_window(drawn.handle, cx);
    visual.update(|window, cx| {
        assert!(window.focused(cx).is_some(), "Tab focuses the tile");
    });
    for key in ["enter", "space"] {
        let keystroke = gpui_kit::Keystroke::parse(key).unwrap_or_else(|_| unreachable!());
        visual.simulate_event(gpui_kit::KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        visual.simulate_event(gpui_kit::KeyUpEvent { keystroke });
    }
    assert_eq!(SELECTED.with(std::cell::Cell::get), 2);
}
