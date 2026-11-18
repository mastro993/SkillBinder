//! A card of rows, and the empty card, drawn in a GPUI test window.

use super::CardList;
use crate::test_window::{Drawn, labels};
use crate::{ListRow, Text};
use gpui_kit::{IntoElement, ParentElement, Styled, TestAppContext, div, px};

#[gpui_kit::test]
fn a_card_shows_its_rows_or_else_its_empty_line(cx: &mut TestAppContext) {
    let frame = Drawn::show(cx, |_, _| {
        div()
            .flex()
            .flex_col()
            .w(px(300.))
            .child(
                CardList::new("full")
                    .child(Text::labeled("header", "Your agents"))
                    .rows(
                        ["research", "review"]
                            .map(|slug| ListRow::new(slug).title(Text::labeled("slug", slug))),
                    )
                    .empty(Text::labeled("unused", "Nothing here")),
            )
            .child(
                CardList::new("none")
                    .h(px(120.))
                    .scrolls()
                    .empty(Text::labeled("empty", "No agents found")),
            )
            .into_any_element()
    })
    .frame(cx);
    assert_eq!(labels(&frame, "Your agents"), 1);
    assert_eq!(labels(&frame, "research"), 1);
    assert_eq!(labels(&frame, "review"), 1);
    assert_eq!(labels(&frame, "Nothing here"), 0);
    assert_eq!(labels(&frame, "No agents found"), 1);
}
