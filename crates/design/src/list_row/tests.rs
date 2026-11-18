//! A list row's title, subtitle, and trailing control, and a click on that control staying out of
//! the row, drawn in a GPUI test window.

use super::ListRow;
use crate::Button;
use crate::test_window::{Drawn, labels, read};
use crate::{Text, tokens::space};
use gpui_kit::base::TestSupportExt;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Context, InteractiveElement, IntoElement, ParentElement, Render, Role,
    StatefulInteractiveElement, Styled, TestAppContext, Window, div, point, px,
};

/// A row counting its own clicks and its trailing button's.
#[derive(Default)]
struct Row {
    rows: usize,
    ends: usize,
}

impl Render for Row {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().id("wrapper").test_support().w(px(400.)).child(
            ListRow::new("row")
                .title(Text::labeled("title", "research"))
                .subtitle(Text::labeled("subtitle", "Find sources"))
                .trailing(
                    Button::new("end")
                        .label("Remove")
                        .small()
                        .on_click(cx.listener(|row, _, _, _| row.ends += 1)),
                )
                .hover(true)
                .divider(true)
                .on_click(cx.listener(|row, _, _, _| row.rows += 1)),
        )
    }
}

#[gpui_kit::test]
fn a_row_reads_its_lines_and_its_trailing_control_keeps_its_clicks(cx: &mut TestAppContext) {
    let drawn = Drawn::open(cx, |_, _| Row::default());
    let frame = drawn.frame(cx);
    assert_eq!(labels(&frame, "research"), 1);
    assert_eq!(labels(&frame, "Find sources"), 1);
    assert_eq!(
        read(&frame, "end"),
        (Role::Button, Some("Remove".into()), false)
    );
    drawn.step(cx, |window, cx| window.click("end", cx));
    drawn.step(cx, |window, cx| {
        window.click_at("wrapper", point(px(space::XS), px(space::XS)), cx)
    });
    let (rows, ends) = cx.read(|cx| {
        let row = drawn.view.read(cx);
        (row.rows, row.ends)
    });
    assert_eq!((rows, ends), (1, 1));
}

#[gpui_kit::test]
fn a_row_is_at_least_the_list_row_height(cx: &mut TestAppContext) {
    let drawn = Drawn::show(cx, |_, _| {
        div()
            .w(px(300.))
            .child(
                div()
                    .debug_selector(|| "row".to_owned())
                    .child(ListRow::new("row").title("research").dimmed(true)),
            )
            .into_any_element()
    });
    drawn.frame(cx);
    let height = drawn.bounds(cx, "row").map(|bounds| bounds.size.height);
    assert_eq!(height, Some(px(space::LIST_ROW)));
}
