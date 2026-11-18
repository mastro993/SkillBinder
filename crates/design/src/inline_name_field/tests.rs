//! Name fields at each size, and Escape cancelling one, drawn in a GPUI test window.

use super::{InlineNameField, NameFieldSize};
use crate::test_window::{Drawn, named, node};
use gpui_kit::component::input::InputState;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, Role, Styled, TestAppContext,
    Window, div, px,
};

/// A field at each size, each over its own input state, and whether Escape cancelled one.
struct Fields {
    states: [Entity<InputState>; 2],
    cancelled: bool,
}

impl Render for Fields {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        div()
            .flex()
            .flex_col()
            .children(
                NameFieldSize::ALL
                    .into_iter()
                    .zip(&self.states)
                    .map(|(size, state)| {
                        let view = view.clone();
                        InlineNameField::new(format!("{size:?}"), state)
                            .size(size)
                            .w(px(160.))
                            .on_cancel(move |_, cx| {
                                view.update(cx, |view, _| view.cancelled = true)
                            })
                    }),
            )
    }
}

#[gpui_kit::test]
fn a_name_field_takes_its_size_and_escape_cancels_it(cx: &mut TestAppContext) {
    let drawn = Drawn::open(cx, |window, cx| Fields {
        states: [
            cx.new(|cx| InputState::new(window, cx)),
            cx.new(|cx| InputState::new(window, cx)),
        ],
        cancelled: false,
    });
    let frame = drawn.frame(cx);
    assert_eq!(
        named(&frame, Role::TextInput).len(),
        NameFieldSize::ALL.len()
    );
    let height = |id| node(&frame, id).map(|node| node.bounds().size.height);
    assert_eq!(height("Sm"), Some(px(24.)));
    assert_eq!(height("Md"), Some(px(32.)));
    let [state, _] = cx.read(|cx| drawn.view.read(cx).states.clone());
    drawn.step(cx, |window, cx| {
        state.update(cx, |state, cx| state.focus(window, cx));
    });
    drawn.step(cx, |window, cx| window.press("escape", cx));
    assert!(cx.read(|cx| drawn.view.read(cx).cancelled));
}
