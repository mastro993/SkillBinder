//! A popover panel placed below what opened it, drawn in a GPUI test window.

use super::PopoverPanel;
use crate::Text;
use crate::test_window::{Drawn, labels};
use gpui_kit::base::{Align, Placement};
use gpui_kit::{
    Bounds, InteractiveElement, IntoElement, ParentElement, Styled, TestAppContext, div, point, px,
    size,
};

#[gpui_kit::test]
fn a_panel_opens_below_its_anchor_lined_up_with_its_start(cx: &mut TestAppContext) {
    let drawn = Drawn::show(cx, |_, _| {
        let anchor = Bounds::new(point(px(40.), px(20.)), size(px(100.), px(30.)));
        div()
            .size_full()
            .child(
                PopoverPanel::new("panel")
                    .anchor(anchor, Placement::Bottom, Align::Start)
                    .w(px(200.))
                    .debug_selector(|| "panel".to_owned())
                    .child(Text::labeled("body", "Where research is on")),
            )
            .into_any_element()
    });
    let frame = drawn.frame(cx);
    assert_eq!(labels(&frame, "Where research is on"), 1);
    let bounds = drawn
        .bounds(cx, "panel")
        .unwrap_or_else(|| panic!("the panel was not drawn"));
    assert_eq!(
        bounds.origin,
        point(px(40.), px(50. + PopoverPanel::OFFSET))
    );
    assert_eq!(bounds.size.width, px(200.));
}
