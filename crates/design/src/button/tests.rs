//! Every button variant and size, inactive, and its accessible name, drawn in a GPUI test window.

use crate::test_window::{Drawn, read};
use crate::{Button, ButtonVariant};
use gpui_kit::{
    InteractiveElement, IntoElement, ParentElement, Role, Styled, TestAppContext, div, px,
};

const IDS: [&str; 4] = ["primary", "secondary", "ghost", "danger"];

#[gpui_kit::test]
fn every_variant_is_a_named_button_at_its_size(cx: &mut TestAppContext) {
    let drawn =
        Drawn::show(cx, |_, _| {
            let row =
                |small: bool| {
                    div()
                        .flex()
                        .gap_2()
                        .children(ButtonVariant::ALL.into_iter().zip(IDS).map(
                            move |(variant, id)| {
                                let id = if small {
                                    format!("{id}-sm")
                                } else {
                                    id.to_owned()
                                };
                                let button = Button::new(id).variant(variant).label("Save");
                                if small { button.small() } else { button }
                            },
                        ))
                };
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(row(false))
                .child(row(true))
                .into_any_element()
        });
    let frame = drawn.frame(cx);
    for id in IDS {
        for (id, height) in [(id.to_owned(), 32.), (format!("{id}-sm"), 24.)] {
            assert_eq!(
                read(&frame, &id),
                (Role::Button, Some("Save".into()), false),
                "{id}"
            );
            let node = crate::test_window::node(&frame, &id).map(|node| node.bounds().size.height);
            assert_eq!(node, Some(px(height)), "{id}");
        }
    }
}

#[gpui_kit::test]
fn an_inactive_button_is_disabled_and_an_icon_button_takes_its_name(cx: &mut TestAppContext) {
    let frame = Drawn::show(cx, |_, _| {
        div()
            .flex()
            .child(
                Button::new("add")
                    .primary()
                    .label("Add skill")
                    .inactive(true),
            )
            .child(
                Button::new("refresh")
                    .ghost()
                    .small()
                    .icon(crate::icon("refresh-04"))
                    .accessibility_label("Check for updates"),
            )
            .into_any_element()
    })
    .frame(cx);
    assert_eq!(
        read(&frame, "add"),
        (Role::Button, Some("Add skill".into()), true)
    );
    assert_eq!(
        read(&frame, "refresh"),
        (Role::Button, Some("Check for updates".into()), false)
    );
}

#[gpui_kit::test]
fn a_trailing_child_follows_the_label(cx: &mut TestAppContext) {
    let drawn = Drawn::show(cx, |_, _| {
        div()
            .flex()
            .child(
                Button::new("targets")
                    .debug_selector(|| "targets".to_owned())
                    .label("All agents")
                    .child(div().debug_selector(|| "caret".to_owned()).size(px(8.))),
            )
            .into_any_element()
    });
    drawn.frame(cx);
    let (Some(button), Some(caret)) = (drawn.bounds(cx, "targets"), drawn.bounds(cx, "caret"))
    else {
        panic!("the button and its caret are drawn");
    };
    assert!(
        caret.center().x > button.center().x,
        "the caret draws after the label: {caret:?} in {button:?}"
    );
}
