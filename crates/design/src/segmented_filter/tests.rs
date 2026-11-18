//! A segmented filter's named segments with their counts, and a click choosing one, drawn in a
//! GPUI test window.

use super::SegmentedFilter;
use crate::test_window::{Drawn, named, readings};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{Context, IntoElement, Render, Role, TestAppContext, Window};

/// A filter remembering the segment a click chose.
#[derive(Default)]
struct Filter(usize);

impl Render for Filter {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        SegmentedFilter::new("filter", [("All", 1200), ("Active", 3), ("Inactive", 0)])
            .selected(self.0)
            .on_select(move |index, _, cx| {
                view.update(cx, |view, cx| {
                    view.0 = index;
                    cx.notify();
                })
            })
    }
}

#[gpui_kit::test]
fn each_segment_reads_its_label_and_count_and_a_click_chooses_it(cx: &mut TestAppContext) {
    let drawn = Drawn::open(cx, |_, _| Filter::default());
    let frame = drawn.frame(cx);
    assert_eq!(
        named(&frame, Role::Tab),
        vec![
            Some("All".into()),
            Some("Active".into()),
            Some("Inactive".into())
        ]
    );
    assert_eq!(readings(&frame, "1,200"), 1);
    drawn.step(cx, |window, cx| window.within("filter").click(2usize, cx));
    assert_eq!(cx.read(|cx| drawn.view.read(cx).0), 2);
}
