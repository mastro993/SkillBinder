//! A skill card's accessible name, its footer's control, a click on the card, and its height,
//! drawn in a GPUI test window.

use super::SkillCard;
use crate::library::TILE_MIN_HEIGHT;
use crate::test_window::{Drawn, read};
use crate::{Button, Text, tokens::space};
use gpui_kit::base::TestSupportExt;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Context, InteractiveElement, IntoElement, ParentElement, Render, Role,
    StatefulInteractiveElement, Styled, TestAppContext, Window, div, point, px,
};

/// A card that opens on click, counting its clicks.
#[derive(Default)]
struct Card {
    opened: usize,
}

impl Render for Card {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().id("wrapper").test_support().w(px(240.)).child(
            SkillCard::new("card")
                .aria_label("pdf-tools")
                .title(Text::new("pdf-tools"))
                .description(
                    "description",
                    "Extracts text, tables, and form fields from PDF files.",
                )
                .faded(true)
                .footer(Button::new("footer").label("Northwind Labs").small())
                .hover(true)
                .on_click(cx.listener(|card, _, _, _| card.opened += 1)),
        )
    }
}

#[gpui_kit::test]
fn a_card_reads_its_name_and_footer_and_takes_clicks(cx: &mut TestAppContext) {
    let drawn = Drawn::open(cx, |_, _| Card::default());
    let frame = drawn.frame(cx);
    assert_eq!(read(&frame, "card").1, Some("pdf-tools".into()));
    assert_eq!(
        read(&frame, "footer"),
        (Role::Button, Some("Northwind Labs".into()), false)
    );
    drawn.step(cx, |window, cx| {
        window.click_at("wrapper", point(px(space::XS), px(space::XS)), cx)
    });
    assert_eq!(cx.read(|cx| drawn.view.read(cx).opened), 1);
}

#[gpui_kit::test]
fn a_card_is_at_least_a_tile_high(cx: &mut TestAppContext) {
    let drawn = Drawn::show(cx, |_, _| {
        div()
            .w(px(240.))
            .child(
                div()
                    .debug_selector(|| "card".to_owned())
                    .child(SkillCard::new("card").title("pdf-tools")),
            )
            .into_any_element()
    });
    drawn.frame(cx);
    let height = drawn.bounds(cx, "card").map(|bounds| bounds.size.height);
    assert_eq!(height, Some(px(TILE_MIN_HEIGHT)));
}
