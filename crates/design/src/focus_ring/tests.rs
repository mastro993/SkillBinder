//! The focus ring follows keyboard focus only, drawn in a GPUI test window.

use super::{FOCUS_RING, focus_ring};
use crate::test_window::Drawn;
use gpui_kit::test::{TestSupportExt, TestWindowExt};
use gpui_kit::{
    AppContext, Context, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, TestAppContext, Window, div, px,
};

/// Two focusable controls, each with the app's focus ring.
struct Controls([FocusHandle; 2]);

impl Render for Controls {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .gap_2()
            .children(
                ["first", "second"]
                    .into_iter()
                    .zip(&self.0)
                    .map(|(id, focus)| {
                        let control = div()
                            .id(id)
                            .size(px(20.))
                            .track_focus(&focus.clone().tab_stop(true))
                            .on_click(|_, _, _| {})
                            .test_support();
                        focus_ring(control, focus, window, cx)
                    }),
            )
    }
}

#[gpui_kit::test]
fn only_focus_from_the_keyboard_shows_the_ring(cx: &mut TestAppContext) {
    let drawn = Drawn::open(cx, |_, cx| Controls([cx.focus_handle(), cx.focus_handle()]));
    drawn.step(cx, |window, cx| window.click("first", cx));
    let first = cx.update(|cx| drawn.view.read(cx).0[0].clone());
    cx.update_window(drawn.handle, |_, window, _| {
        assert!(first.is_focused(window))
    })
    .unwrap_or_else(|error| panic!("read test window: {error}"));
    assert_eq!(drawn.bounds(cx, FOCUS_RING), None, "a click shows no ring");
    drawn.step(cx, |window, cx| window.press("tab", cx));
    let ring = drawn.bounds(cx, FOCUS_RING);
    // Tab moved focus to the second control, after the first's 20 points.
    assert!(
        ring.is_some_and(|ring| ring.origin.x > px(20.)),
        "Tab shows the ring on the second control: {ring:?}"
    );
}
