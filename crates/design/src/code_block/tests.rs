//! A code block's command and its Copy, which puts the command on the clipboard, drawn in a
//! GPUI test window.

use super::CodeBlock;
use crate::test_window::{Drawn, labels, named};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{Context, IntoElement, Render, Role, TestAppContext, Window};

const COMMAND: &str = "sudo apt install git";

/// A code block counting its copies.
#[derive(Default)]
struct Command(usize);

impl Render for Command {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        CodeBlock::new("command-box", COMMAND)
            .on_copy(move |_, cx| view.update(cx, |view, _| view.0 += 1))
    }
}

#[gpui_kit::test]
fn copy_puts_the_command_on_the_clipboard(cx: &mut TestAppContext) {
    let drawn = Drawn::open(cx, |_, _| Command::default());
    let frame = drawn.frame(cx);
    assert_eq!(labels(&frame, COMMAND), 1);
    assert_eq!(named(&frame, Role::Button), vec![Some("Copy".into())]);
    drawn.step(cx, |window, cx| {
        window.within("command-box").click("copy", cx)
    });
    assert_eq!(cx.read(|cx| drawn.view.read(cx).0), 1);
    let copied = cx.read_from_clipboard().and_then(|item| item.text());
    assert_eq!(copied.as_deref(), Some(COMMAND));
}
