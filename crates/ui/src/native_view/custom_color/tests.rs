//! The Custom picker as a menu panel: a shade, the held color, or a hex value picks, and the
//! pick closes the menu.

use super::CustomColor;
use gpui_kit::component::{ColorName, Colorize};
use gpui_kit::test::{TestSupportExt, TestWindowExt};
use gpui_kit::{
    AnyWindowHandle, AppContext, Bounds, Context, FocusHandle, Hsla, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, ParentElement, Render, Styled, TestAppContext,
    Window, WindowBounds, WindowOptions, div, point, px, rgb, size,
};
use skillbinder_design::menu::PADDING;
use skillbinder_design::{MenuItem, OpenMenu};

/// The color the picker starts from.
const HELD: u32 = 0x2563eb;

/// A row whose menu's one row, Custom, opens the picker, recording what it picks.
struct Row {
    focus: FocusHandle,
    menu: Option<OpenMenu>,
    picked: Vec<String>,
}

impl Render for Row {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("view")
            .test_support()
            .track_focus(&self.focus)
            .size_full()
            .child(
                div()
                    .id("row")
                    .test_support()
                    .h(px(32.))
                    .w(px(200.))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|row, event: &MouseDownEvent, window, cx| {
                            let custom = CustomColor::new(
                                Hsla::from(rgb(HELD)),
                                window,
                                cx,
                                |row: &mut Row, picked, done, window, cx| {
                                    if let Some(picked) = picked {
                                        row.picked.push(picked.to_hex());
                                    }
                                    if done && let Some(menu) = row.menu.as_ref() {
                                        menu.close(window, cx);
                                    }
                                },
                            );
                            let item = MenuItem::new("custom", "Custom")
                                .panel(custom.focus().clone(), move |window, cx| {
                                    custom.panel(window, cx)
                                });
                            row.menu = Some(OpenMenu::new(
                                "row",
                                vec![item],
                                event.position,
                                row.focus.clone(),
                                window,
                                cx,
                                |row: &mut Row| &mut row.menu,
                            ));
                        }),
                    ),
            )
            .children(self.menu.as_ref().map(OpenMenu::render))
    }
}

/// The window, its menu open with the picker's panel beside it, and the panel's bounds.
fn open(
    cx: &mut TestAppContext,
) -> (
    AnyWindowHandle,
    gpui_kit::Entity<Row>,
    Bounds<gpui_kit::Pixels>,
) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        cx.set_reduce_motion(true);
    });
    let (handle, row) = cx
        .update(|cx| {
            let bounds = Bounds::new(Default::default(), size(px(900.), px(700.)));
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| {
                    let focus = cx.focus_handle();
                    window.focus(&focus, cx);
                    Row {
                        focus,
                        menu: None,
                        picked: Vec::new(),
                    }
                })
            })
        })
        .unwrap_or_else(|error| panic!("open test window: {error}"));
    step(cx, handle, |window, _| window.activate_window());
    step(cx, handle, |window, cx| window.right_click("row", cx));
    step(cx, handle, |window, cx| {
        window.hover("menu-item-custom", cx)
    });
    let mut panel = Bounds::default();
    check(cx, handle, |window| {
        panel = window.find("menu-panel").bounds()
    });
    (handle, row, panel)
}

/// Draws a frame, runs `act`, then lets deferred work finish.
fn step(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    act: impl FnOnce(&mut Window, &mut gpui_kit::App),
) {
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        act(window, cx);
    })
    .unwrap_or_else(|error| panic!("update test window: {error}"));
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| window.render_frame(cx))
        .unwrap_or_else(|error| panic!("update test window: {error}"));
}

fn check(cx: &mut TestAppContext, handle: AnyWindowHandle, check: impl FnOnce(&mut Window)) {
    cx.update_window(handle, |_, window, _| check(window))
        .unwrap_or_else(|error| panic!("read test window: {error}"));
}

fn press(cx: &mut TestAppContext, handle: AnyWindowHandle, keys: &[&str]) {
    for key in keys {
        step(cx, handle, |window, cx| window.press(key, cx));
    }
}

/// What the picker picked, once the menu has closed.
fn picked(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    row: &gpui_kit::Entity<Row>,
) -> Vec<String> {
    check(cx, handle, |window| {
        assert!(window.try_find("menu").is_none())
    });
    cx.update(|cx| row.read(cx).picked.clone())
}

/// The centre of the shade in column `column` of row `row`: swatches are 20 pt apart by 4.
fn swatch(column: f32, row: f32) -> gpui_kit::Point<gpui_kit::Pixels> {
    let at = |index: f32| px(PADDING + index * 24. + 10.);
    point(at(column), at(row))
}

#[gpui_kit::test]
fn a_shade_picks_its_color(cx: &mut TestAppContext) {
    let (handle, row, _) = open(cx);
    // Red's row, the second, at its sixth shade, 500.
    step(cx, handle, |window, cx| {
        window.click_at("menu-panel", swatch(5., 1.), cx)
    });
    let red = ColorName::Red.scale(500).to_hex();
    assert_eq!(picked(cx, handle, &row), [red]);
}

#[gpui_kit::test]
fn leaving_a_shade_restores_the_held_color_and_enter_picks_it(cx: &mut TestAppContext) {
    let (handle, row, panel) = open(cx);
    step(cx, handle, |window, cx| {
        window.simulate_mouse_move(panel.origin + swatch(5., 5.), cx)
    });
    // Off the shades, onto the panel's padding.
    step(cx, handle, |window, cx| {
        window.simulate_mouse_move(panel.origin + point(px(2.), px(2.)), cx)
    });
    // Entering the hex field now picks the held color, not the shade, even though it is the
    // color the picker started from.
    press(cx, handle, &["right", "enter"]);
    assert_eq!(picked(cx, handle, &row), [Hsla::from(rgb(HELD)).to_hex()]);
}

#[gpui_kit::test]
fn a_hex_value_picks_it(cx: &mut TestAppContext) {
    let (handle, row, _) = open(cx);
    // Into the panel, which focuses the hex field; its own keys stay in it.
    press(
        cx,
        handle,
        &["right", "secondary-a", "backspace", "left", "up"],
    );
    step(cx, handle, |window, cx| window.input("#3366FF", cx));
    check(cx, handle, |window| {
        assert!(window.try_find("menu-panel").is_some())
    });
    press(cx, handle, &["enter"]);
    assert_eq!(
        picked(cx, handle, &row),
        [Hsla::from(rgb(0x3366ff)).to_hex()]
    );
}
