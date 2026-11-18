//! The icon scale, drawn in a GPUI test window.

use super::{IconSize, icon};
use crate::test_window::Drawn;
use gpui_kit::component::Sizable;
use gpui_kit::{InteractiveElement, IntoElement, ParentElement, Styled, TestAppContext, div, px};

const SELECTORS: [&str; 4] = ["icon-Xs", "icon-Sm", "icon-Md", "icon-Lg"];

#[gpui_kit::test]
fn each_size_draws_at_its_points(cx: &mut TestAppContext) {
    let drawn = Drawn::show(cx, |_, _| {
        div()
            .flex()
            .items_start()
            .children(
                IconSize::ALL
                    .into_iter()
                    .zip(SELECTORS)
                    .map(|(size, selector)| {
                        div()
                            .flex_none()
                            .debug_selector(move || selector.to_owned())
                            .child(icon("close").with_size(size))
                    }),
            )
            .into_any_element()
    });
    drawn.frame(cx);
    for (size, selector) in IconSize::ALL.into_iter().zip(SELECTORS) {
        let bounds = drawn.bounds(cx, selector);
        assert_eq!(
            bounds.map(|bounds| bounds.size.width),
            Some(px(size.points())),
            "{selector}"
        );
    }
}

#[test]
fn the_scale_is_twelve_fourteen_sixteen_twenty() {
    let points = IconSize::ALL.map(IconSize::points);
    assert_eq!(points, [12., 14., 16., 20.]);
}
