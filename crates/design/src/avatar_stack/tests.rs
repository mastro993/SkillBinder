//! Avatar stacks within and over their limit, drawn in a GPUI test window.

use super::{AvatarStack, OVERLAP, SIZE};
use crate::Logo;
use crate::test_window::{Drawn, labels, node};
use gpui_kit::{InteractiveElement, IntoElement, ParentElement, Styled, TestAppContext, div, px};

const GLYPH: Logo = Logo::Glyph("icons/robot.svg");

#[gpui_kit::test]
fn a_stack_past_its_limit_shows_plus_the_rest(cx: &mut TestAppContext) {
    let drawn = Drawn::show(cx, |_, _| {
        div()
            .flex()
            .flex_col()
            .items_start()
            .gap(px(8.))
            .child(
                div()
                    .debug_selector(|| "one".to_owned())
                    .flex()
                    .child(AvatarStack::new([GLYPH])),
            )
            .child(AvatarStack::new([GLYPH; 6]).names(
                "named",
                ["Claude", "Codex", "Cursor", "Gemini", "Kiro", "Zed"],
            ))
            .child(
                div()
                    .debug_selector(|| "limited".to_owned())
                    .flex()
                    .child(AvatarStack::new([GLYPH; 3]).limit(2)),
            )
            .into_any_element()
    });
    let frame = drawn.frame(cx);
    // Two hidden past four, one past two.
    assert_eq!(labels(&frame, "+2"), 1);
    assert_eq!(labels(&frame, "+1"), 1);
    assert!(node(&frame, "named").is_some());
    let width = drawn.bounds(cx, "one").map(|bounds| bounds.size.width);
    assert_eq!(width, Some(px(SIZE)));
    // Two avatars and `+1`: as wide as three circles, less their overlaps.
    let width = drawn.bounds(cx, "limited").map(|bounds| bounds.size.width);
    assert_eq!(width, Some(px(SIZE * 3. - OVERLAP * 2.)));
}
