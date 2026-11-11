//! The controls laid out in a real GPUI window at the app's 1180 by 728 points: their sizes, and
//! the popover placed from where its trigger was painted in the same frame.

use std::{cell::Cell, rc::Rc};

use super::super::library_copies::Reader;
use super::super::place_popover::place_line;
use super::Logo;
use super::popover::{Align, Anchor, placed, surface};
use super::{pill, switch};
use gpui_kit::base::TestSupportExt;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AnyWindowHandle, App, AppContext, Bounds, Context, InteractiveElement, IntoElement,
    ParentElement, Pixels, Render, Styled, TestAppContext, Window, WindowBounds, WindowOptions,
    canvas, deferred, div, px, size,
};

struct Gallery {
    trigger: Rc<Cell<Option<Bounds<Pixels>>>>,
}

impl Render for Gallery {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let record = self.trigger.clone();
        let anchor = self.trigger.clone();
        let logo = Logo::Glyph("icons/layers.svg");
        div()
            .size_full()
            .relative()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(pill::pill("pill-on", logo, true, false, cx).test_support())
                    .child(pill::pill("pill-off", logo, false, false, cx).test_support())
                    .child(switch::switch("switch", true, cx).test_support()),
            )
            .child(popover_lines(logo, cx))
            .child(
                div()
                    .absolute()
                    .left(px(1013.))
                    .top(px(293.))
                    .w(px(136.))
                    .h(px(26.))
                    .child(
                        canvas(
                            move |bounds, _, _| record.set(Some(bounds)),
                            |_, (), _, _| {},
                        )
                        .size_full(),
                    ),
            )
            .child(deferred(placed(
                move || {
                    let trigger = anchor.get()?;
                    Some(Anchor {
                        trigger,
                        above: trigger.top(),
                        align: Align::End,
                    })
                },
                surface("popover", 300., 12., div().h(px(261.)), cx).test_support(),
            )))
    }
}

/// The popover's lines, 276 points wide: All agents, 97.74 points of label and switch, with four
/// pills and with eight, and docs-site's, whose 12 pills wrap beside its 95.73-point name.
fn popover_lines(logo: Logo, cx: &App) -> impl IntoElement {
    let pills = |prefix: &'static str, count: usize| {
        (0..count).map(move |index| {
            pill::pill(format!("{prefix}{index}"), logo, false, false, cx).test_support()
        })
    };
    let lead = |id: &'static str, width: f32| {
        div()
            .id(id)
            .child(div().w(px(width)).h(px(18.2)))
            .test_support()
    };
    div()
        .absolute()
        .top(px(100.))
        .left(px(100.))
        .w(px(276.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            place_line(lead("all-row", 97.74), pills("agent-", 4))
                .id("agents-line")
                .test_support(),
        )
        .child(
            place_line(lead("all-row-many", 97.74), pills("many-", 8))
                .id("many-line")
                .test_support(),
        )
        .child(
            place_line(lead("docs-name", 95.73), pills("dir-", 12))
                .id("docs-line")
                .test_support(),
        )
}

fn open(cx: &mut TestAppContext) -> AnyWindowHandle {
    cx.update(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    Default::default(),
                    size(px(1180.), px(728.)),
                ))),
                ..Default::default()
            },
            cx,
            |_, cx| {
                cx.new(|_| Gallery {
                    trigger: Rc::default(),
                })
            },
        )
        .map(|(handle, _)| handle)
    })
    .unwrap_or_else(|error| panic!("open test window: {error}"))
}

fn bounds(cx: &mut TestAppContext, window: AnyWindowHandle, id: &'static str) -> Bounds<Pixels> {
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
        window.find(id).bounds()
    })
    .unwrap_or_else(|error| panic!("read test window: {error}"))
}

fn sized(bounds: Bounds<Pixels>) -> (f32, f32) {
    (f32::from(bounds.size.width), f32::from(bounds.size.height))
}

#[gpui_kit::test]
fn controls_take_the_prototypes_sizes(cx: &mut TestAppContext) {
    let window = open(cx);
    assert_eq!(sized(bounds(cx, window, "pill-on")), (26., 24.));
    assert_eq!(sized(bounds(cx, window, "pill-off")), (26., 24.));
    assert_eq!(sized(bounds(cx, window, "switch")), (30., 18.));
}

#[gpui_kit::test]
fn popover_pills_sit_at_the_trailing_edge_and_wrap_beside_the_name(cx: &mut TestAppContext) {
    let window = open(cx);
    // Layout snaps to device pixels, half a point apart at 2x.
    let near = |a: Pixels, b: f32| (f32::from(a) - b).abs() <= 0.5;
    assert_eq!(bounds(cx, window, "agents-line").size.height, px(28.));
    // Four pills end at the line's trailing edge: the first at +160, +172 in the popover.
    let first = bounds(cx, window, "agent-0");
    assert!(near(first.origin.x, 260.), "{first:?}");
    assert!(
        near(bounds(cx, window, "all-row").size.width, 152.),
        "All agents takes the free width"
    );
    // Eight agents wrap five and three beside All agents, which keeps its width.
    let many = bounds(cx, window, "all-row-many");
    assert!(f32::from(many.size.width) >= 97.74 - 0.5, "{many:?}");
    assert_eq!(bounds(cx, window, "many-line").size.height, px(52.));
    // docs-site's pills: 5, 5, and 2 on three lines.
    let docs = bounds(cx, window, "docs-name");
    let last = bounds(cx, window, "dir-11");
    assert!(near(docs.size.width, 95.73), "{docs:?}");
    assert_eq!(bounds(cx, window, "docs-line").size.height, px(80.));
    // The last line's two pills keep to the trailing edge.
    let line = bounds(cx, window, "docs-line");
    assert!(
        near(last.right(), 376.) && near(last.origin.y, f32::from(line.origin.y) + 56.),
        "{last:?}"
    );
}

#[gpui_kit::test]
fn the_popover_opens_below_its_trigger_painted_in_the_same_frame(cx: &mut TestAppContext) {
    let window = open(cx);
    let popover = bounds(cx, window, "popover");
    let near = |a: Pixels, b: f32| (f32::from(a) - b).abs() < 0.01;
    // The ring lies 1 point outside the 300-point box at (849, 323): right edges aligned, 4 below.
    assert!(
        near(popover.origin.x, 848.) && near(popover.origin.y, 322.),
        "{popover:?}"
    );
    assert_eq!(sized(popover), (302., 287.));
}

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
