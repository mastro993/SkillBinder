//! A search field at a fixed width and filling its container, drawn in a GPUI test window.

use super::SearchField;
use crate::test_window::{Drawn, named, node};
use gpui_kit::component::input::InputState;
use gpui_kit::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, Role, Styled, TestAppContext,
    Window, div, px,
};

/// Two fields over one input state.
struct Fields(Entity<InputState>);

impl Render for Fields {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(400.))
            .flex()
            .flex_col()
            .child(SearchField::new("fixed", &self.0).width(200.))
            .child(SearchField::new("fill", &self.0).accessibility_label("Search skills"))
    }
}

#[gpui_kit::test]
fn a_search_field_is_a_named_text_input_at_its_width(cx: &mut TestAppContext) {
    let drawn = Drawn::open(cx, |window, cx| {
        Fields(cx.new(|cx| InputState::new(window, cx)))
    });
    let frame = drawn.frame(cx);
    let inputs = named(&frame, Role::TextInput);
    assert_eq!(
        inputs,
        vec![Some("Search".into()), Some("Search skills".into())]
    );
    let width = |id| node(&frame, id).map(|node| node.bounds().size.width);
    assert_eq!(width("fixed"), Some(px(200.)));
    assert_eq!(width("fill"), Some(px(400.)));
}
