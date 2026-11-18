//! Logos at a size from the icon scale, drawn in a GPUI test window.

use super::Logo;
use crate::IconSize;
use crate::test_window::Drawn;
use gpui_kit::{InteractiveElement, IntoElement, ParentElement, Styled, TestAppContext, div, px};

#[gpui_kit::test]
fn a_glyph_and_a_color_logo_draw_at_the_size_given(cx: &mut TestAppContext) {
    let drawn = Drawn::show(cx, |_, _| {
        div()
            .flex()
            .items_start()
            .child(
                div()
                    .flex_none()
                    .debug_selector(|| "glyph".to_owned())
                    .child(Logo::Glyph("icons/layers.svg").element(IconSize::Md, false)),
            )
            .child(
                div()
                    .flex_none()
                    .debug_selector(|| "color".to_owned())
                    .child(Logo::Color("agents/claude-code-color.svg").element(IconSize::Lg, true)),
            )
            .into_any_element()
    });
    drawn.frame(cx);
    let mut width = |selector| drawn.bounds(cx, selector).map(|bounds| bounds.size.width);
    assert_eq!(width("glyph"), Some(px(IconSize::Md.points())));
    assert_eq!(width("color"), Some(px(IconSize::Lg.points())));
    assert!(Logo::Color("agents/x.svg").is_color());
    assert!(!Logo::Mono("agents/x.svg").is_color());
}
