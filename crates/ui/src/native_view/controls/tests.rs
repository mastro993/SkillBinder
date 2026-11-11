//! The logo an agent or a skill directory gets, and how wide a tooltip grows.

use super::super::library_copies::Reader;
use super::frames::{Drawn, node};
use super::{Logo, TOOLTIP_LINE, tooltip};
use gpui_kit::test::{TestSupportExt, TestWindowExt};
use gpui_kit::{
    AppContext, Context, InteractiveElement, IntoElement, Pixels, Render,
    StatefulInteractiveElement, Styled, TestAppContext, Window, div, px,
};
use std::time::Duration;

#[test]
fn agents_take_their_color_logo_then_their_monochrome_one_then_the_terminal_glyph() {
    assert_eq!(
        Logo::agent("claude-code"),
        Logo::Color("agents/claude-code-color.svg")
    );
    assert_eq!(Logo::agent("cursor"), Logo::Mono("agents/cursor.svg"));
    assert_eq!(
        Logo::agent("dexto"),
        Logo::Glyph("icons/square-terminal.svg")
    );
    assert_eq!(
        Logo::reader(&Reader::Universal(Vec::new())),
        Logo::Glyph("icons/layers.svg")
    );
}

/// An element with the tooltip reading the text it holds.
struct Tipped(&'static str);

impl Render for Tipped {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("tipped")
            .size(px(20.))
            .tooltip(tooltip(self.0))
            .test_support()
    }
}

/// The width and height of the box of the tooltip reading `text`, shown as GPUI shows it once
/// the pointer rests on its element.
fn tooltip_box(cx: &mut TestAppContext, text: &'static str) -> (Pixels, Pixels) {
    let drawn = Drawn::open(cx, move |_, _| Tipped(text));
    cx.update_window(drawn.handle, |_, window, cx| window.hover("tipped", cx))
        .unwrap_or_else(|error| panic!("hover test window: {error}"));
    cx.executor().advance_clock(Duration::from_secs(1));
    cx.run_until_parked();
    let frame = drawn.frame(cx);
    let tip = node(&frame, "tooltip-popup").unwrap_or_else(|| panic!("no tooltip was drawn"));
    let bounds = tip.bounds();
    (bounds.size.width, bounds.size.height)
}

#[gpui_kit::test]
fn a_long_tooltip_line_wraps_at_the_widest_a_line_grows(cx: &mut TestAppContext) {
    let (_, line) = tooltip_box(cx, "Used by Claude Code");
    let (width, height) = tooltip_box(
        cx,
        "~/.agents/skills\nUsed by Amp, Cline, Codex, Cursor, Gemini CLI, GitHub Copilot, Kimi \
         Code CLI, OpenCode and Warp",
    );
    // Kit pads the box 8 points on either side of its lines, inside a 1-point border.
    assert!(width <= px(TOOLTIP_LINE + 18.), "{width:?}");
    assert!(height > line * 2., "the long line wraps: {height:?}");
}
