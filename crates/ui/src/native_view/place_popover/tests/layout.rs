//! The popover's layout in a GPUI test window at the app's 1180 by 728 points: pills wrapping at a
//! line's trailing edge, and Kit's positioner opening the popover below its trigger, or above it
//! or its whole tile, from where the trigger was last painted.

use super::super::super::controls::Logo;
use super::super::super::library_places::place_pill;
use super::super::super::snapshot_query::fixture::id;
use super::super::{OFFSET, PlacePopover, WIDTH, anchor, place_line, placed};
use gpui_kit::test::{TestSupportExt, TestWindowExt};
use gpui_kit::{
    AnyWindowHandle, App, AppContext, Bounds, Context, Entity, FocusHandle, InteractiveElement,
    IntoElement, ParentElement, Pixels, Render, Styled, Subscription, TestAppContext, Window,
    WindowBounds, WindowOptions, canvas, deferred, div, point, prelude::FluentBuilder, px, size,
};
use skillbinder_theme::menu::WINDOW_MARGIN;

const WINDOW: (f32, f32) = (1180., 728.);

/// The popover's lines, 276 points wide: All agents, 97.74 points of label and switch, with four
/// pills and with eight, and docs-site's, whose 12 pills wrap beside its 95.73-point name.
fn popover_lines(cx: &App) -> impl IntoElement {
    let logo = Logo::Glyph("icons/layers.svg");
    let pills = |prefix: &'static str, count: usize| {
        (0..count)
            .map(move |index| place_pill(format!("{prefix}{index}"), logo, false, false, "", cx))
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

/// A 300-point popover `height` high, placed from a trigger at `trigger`, inside a tile from
/// `tile_top` in a grid.
fn popover(
    name: &'static str,
    trigger: (f32, f32, f32),
    height: f32,
    tile_top: Option<f32>,
) -> impl IntoElement {
    let (x, y, width) = trigger;
    let trigger = Bounds::new(point(px(x), px(y)), size(px(width), px(26.)));
    let tile =
        tile_top.map(|top| Bounds::from_corners(point(px(x), px(top)), trigger.bottom_right()));
    deferred(placed(
        anchor(trigger, tile, tile_top.is_some()),
        tile_top.is_some(),
        div().id(name).w(px(WIDTH)).h(px(height)).test_support(),
    ))
}

struct Gallery;

impl Render for Gallery {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .relative()
            .child(popover_lines(cx))
            .child(popover("row", (1013., 293., 136.), 285., None))
            .child(popover("low", (1090., 647., 59.), 285., None))
            .child(popover("tile", (955.5, 600., 189.66), 233., Some(470.)))
    }
}

fn options() -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            Default::default(),
            size(px(WINDOW.0), px(WINDOW.1)),
        ))),
        ..Default::default()
    }
}

fn gallery(cx: &mut TestAppContext) -> AnyWindowHandle {
    cx.update(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(options(), cx, |_, cx| cx.new(|_| Gallery)).map(|(handle, _)| handle)
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

/// How many rows the pills `ids`, in drawing order, sit on.
fn rows(cx: &mut TestAppContext, window: AnyWindowHandle, ids: &[&'static str]) -> usize {
    let mut tops: Vec<i32> = ids
        .iter()
        .map(|id| f32::from(bounds(cx, window, id).origin.y).round() as i32)
        .collect();
    tops.dedup();
    tops.len()
}

#[gpui_kit::test]
fn popover_pills_sit_at_the_trailing_edge_and_wrap_beside_the_name(cx: &mut TestAppContext) {
    let window = gallery(cx);
    // Layout snaps to device pixels, half a point apart at 2x.
    let near = |a: Pixels, b: Pixels| (f32::from(a) - f32::from(b)).abs() <= 0.5;
    // Four pills fit on one line, ending at its trailing edge; All agents takes the free width.
    let line = bounds(cx, window, "agents-line");
    let (lead, first) = (bounds(cx, window, "all-row"), bounds(cx, window, "agent-0"));
    assert_eq!(
        rows(cx, window, &["agent-0", "agent-1", "agent-2", "agent-3"]),
        1
    );
    assert!(near(bounds(cx, window, "agent-3").right(), line.right()));
    assert!(
        near(lead.right() + px(8.), first.left()),
        "{lead:?} {first:?}"
    );
    // Eight wrap onto two lines beside All agents, which keeps its width.
    assert_eq!(
        rows(cx, window, &["many-0", "many-4", "many-5", "many-7"]),
        2
    );
    let lead = bounds(cx, window, "all-row-many");
    assert!(f32::from(lead.size.width) >= 97.74 - 0.5, "{lead:?}");
    // docs-site's twelve wrap onto three lines beside its name, the last line at the trailing
    // edge too.
    let line = bounds(cx, window, "docs-line");
    assert_eq!(rows(cx, window, &["dir-0", "dir-5", "dir-10", "dir-11"]), 3);
    assert!(near(bounds(cx, window, "dir-11").right(), line.right()));
    let name = bounds(cx, window, "docs-name");
    assert!(f32::from(name.size.width) >= 95.73 - 0.5, "{name:?}");
}

#[gpui_kit::test]
fn the_popover_opens_below_its_trigger_else_above_it_or_its_tile(cx: &mut TestAppContext) {
    let window = gallery(cx);
    let margin = px(WINDOW_MARGIN);
    let inside = |popover: Bounds<Pixels>| {
        popover.left() >= margin
            && popover.top() >= margin
            && popover.right() <= px(WINDOW.0) - margin
            && popover.bottom() <= px(WINDOW.1) - margin
    };
    // A row's opens below its trigger, lined up with its trailing edge.
    let row = bounds(cx, window, "row");
    assert_eq!(row.top(), px(293. + 26. + OFFSET), "{row:?}");
    assert_eq!(row.right(), px(1013. + 136.), "{row:?}");
    // Near the window's bottom it opens above the trigger.
    let low = bounds(cx, window, "low");
    assert!(low.bottom() <= px(647.), "{low:?}");
    // A grid tile's opens above the whole tile when there is no room below its trigger.
    let tile = bounds(cx, window, "tile");
    assert!(tile.bottom() <= px(470.), "{tile:?}");
    for popover in [row, low, tile] {
        assert!(inside(popover), "{popover:?} keeps the window's margin");
    }
}

/// Draws the open popover as Library does, from where its trigger, while there is one at `trigger`
/// points from the top, was last painted; the trigger records where it is only while the popover
/// is open. Like the Library view, it has focus until the popover takes it, and it notes whenever
/// nothing in the window has focus.
struct Stage {
    focus: FocusHandle,
    popover: Option<PlacePopover>,
    trigger: Option<f32>,
    lost: bool,
    _lost: Subscription,
}

impl Stage {
    fn close(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.popover = None;
    }
}

impl Render for Stage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let stage = div().size_full().track_focus(&self.focus);
        let Some(popover) = &self.popover else {
            return stage;
        };
        let surface = popover.begin_frame().map(|anchor| {
            let content = div().h(px(200.)).into_any_element();
            let label = "Where review is on".to_owned();
            deferred(
                popover.place(
                    anchor,
                    popover
                        .surface(label, content, Self::close, cx)
                        .into_any_element(),
                ),
            )
        });
        let cells = popover.cells.clone();
        stage
            .when_some(self.trigger, |stage, top| {
                stage.child(
                    div()
                        .absolute()
                        .left(px(400.))
                        .top(px(top))
                        .w(px(136.))
                        .h(px(26.))
                        .child(
                            canvas(
                                move |bounds, window, _| cells.record_trigger(bounds, window),
                                |_, (), _, _| {},
                            )
                            .size_full(),
                        ),
                )
            })
            .children(surface)
    }
}

/// A window with a trigger at 100 points from the top, then `review`'s popover opened from it.
fn stage(cx: &mut TestAppContext) -> (AnyWindowHandle, Entity<Stage>) {
    let (window, stage): (AnyWindowHandle, Entity<Stage>) = cx
        .update(|cx| {
            gpui_kit::init(cx);
            gpui_kit::open_window(options(), cx, |window, cx| {
                let focus = cx.focus_handle();
                window.focus(&focus, cx);
                cx.new(|cx| Stage {
                    focus,
                    popover: None,
                    trigger: Some(100.),
                    lost: false,
                    _lost: cx.on_focus_lost(window, |stage: &mut Stage, _, _| stage.lost = true),
                })
            })
        })
        .unwrap_or_else(|error| panic!("open test window: {error}"));
    cx.update_window(window, |_, window, cx| {
        stage.update(cx, |stage, cx| {
            stage.popover = Some(PlacePopover::open(id("review"), false, window, cx));
        });
    })
    .unwrap_or_else(|error| panic!("open the popover: {error}"));
    (window, stage)
}

#[gpui_kit::test]
fn the_popover_keeps_its_focus_while_opening_then_follows_its_trigger(cx: &mut TestAppContext) {
    let (window, stage) = stage(cx);
    // Delivers the frames asked for, then draws one: how many were asked for, the popover's
    // bounds, and whether it has focus.
    let frame = |cx: &mut TestAppContext| {
        cx.update_window(window, |_, window, cx| {
            let asked = window.simulate_next_frame(cx);
            window.render_frame(cx);
            let popover = window.try_find("place-popover");
            let focused = popover.as_ref().and_then(|popover| popover.focused());
            (asked, popover.map(|popover| popover.bounds()), focused)
        })
        .unwrap_or_else(|error| panic!("draw test window: {error}"))
    };
    let move_trigger = |cx: &mut TestAppContext, top: Option<f32>| {
        cx.update(|cx| stage.update(cx, |stage, _| stage.trigger = top));
    };
    let lost = |cx: &mut TestAppContext| cx.update(|cx| stage.read(cx).lost);
    // Opening, it is drawn unseen until its trigger has been painted, so it keeps its focus.
    frame(cx);
    let (_, popover, focused) = frame(cx);
    assert_eq!(focused, Some(true));
    assert!(!lost(cx), "the popover never lets go of its focus");
    // Placed 4 points below its trigger, which asks for no frame while it stays put.
    let top = |popover: Option<Bounds<Pixels>>| popover.map(|popover| popover.top());
    assert_eq!(top(popover), Some(px(126. + OFFSET)));
    assert_eq!(frame(cx).0, 0, "a trigger that stays put asks for nothing");
    // A trigger painted elsewhere asks for the frame that moves the popover under it.
    move_trigger(cx, Some(300.));
    let (first, ..) = frame(cx);
    let (second, popover, _) = frame(cx);
    assert!(first + second > 0, "the moved trigger asks for a frame");
    assert_eq!(top(popover), Some(px(326. + OFFSET)));
    // Once its trigger is no longer painted, it hides from the next frame on.
    move_trigger(cx, None);
    frame(cx);
    assert_eq!(frame(cx).1, None);
}
