use super::super::custom_color::CustomColor;
use super::{MenuItem, OpenMenu, SUBMENU_CLOSE_DELAY, step};
use gpui_kit::component::{ColorName, Colorize};
use gpui_kit::test::{TestSupportExt, TestWindowExt};
use gpui_kit::{
    AnyWindowHandle, AppContext, Bounds, Context, FocusHandle, FrameObserver, Hsla,
    InteractiveElement, IntoElement, KeyBinding, MouseButton, MouseDownEvent, ParentElement,
    Render, StatefulInteractiveElement, Styled, Subscription, TestAppContext, Window, WindowBounds,
    WindowOptions, div, point, px, rgb, size,
};
use skillbinder_theme::menu::{PADDING, SUBMENU_OVERLAP};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

gpui_kit::actions!(
    test,
    [
        /// Stands in for a window shortcut such as Toggle Sidebar.
        Shortcut
    ]
);

#[test]
fn the_arrow_keys_skip_separators_and_disabled_rows_and_wrap() {
    let items = [
        MenuItem::new("rename", "Rename"),
        MenuItem::separator(),
        MenuItem::new("up", "Move Up").disabled(true),
        MenuItem::new("down", "Move Down"),
    ];
    assert_eq!(step(&items, None, true), Some(0));
    assert_eq!(step(&items, None, false), Some(3));
    assert_eq!(step(&items, Some(0), true), Some(3));
    assert_eq!(step(&items, Some(3), true), Some(0));
    assert_eq!(step(&items, Some(0), false), Some(3));
    assert_eq!(step(&[], None, true), None);
    assert_eq!(step(&[MenuItem::separator()], None, true), None);
}

type Log = Rc<RefCell<Vec<String>>>;

/// Stands in for the sidebar: a row whose menu records its choices, the clicks that reach the
/// row, and a field that goes away when it loses focus, as a Rename field does.
struct Sidebar {
    focus: FocusHandle,
    /// The focus of the panel's content.
    panel: FocusHandle,
    menu: Option<OpenMenu>,
    log: Log,
    row_clicks: usize,
    /// Runs of a window shortcut.
    shortcuts: usize,
    field: Option<(FocusHandle, Subscription)>,
}

/// Rename; a Color submenu of Gray, Red, and Blue, with Blue checked, then Custom, whose panel
/// holds the color picker; an Icon panel whose content picks with a click or Enter; Move Up,
/// disabled; Move Down; and Delete…, a danger row.
fn items(
    log: &Log,
    panel: FocusHandle,
    window: &mut Window,
    cx: &mut Context<Sidebar>,
) -> Vec<MenuItem> {
    let on = |name: &'static str| {
        let log = log.clone();
        move |_: &mut Window, _: &mut gpui_kit::App| log.borrow_mut().push(name.into())
    };
    let custom = CustomColor::new(
        Hsla::from(rgb(0x2563eb)),
        window,
        cx,
        |sidebar: &mut Sidebar, picked, done, window, cx| {
            if let Some(picked) = picked {
                let picked = format!("custom {}", picked.to_hex());
                sidebar.log.borrow_mut().push(picked);
            }
            if done && let Some(menu) = sidebar.menu.as_ref() {
                menu.close(window, cx);
            }
        },
    );
    let sidebar = cx.entity().downgrade();
    let pick = move |window: &mut Window, cx: &mut gpui_kit::App| {
        let _ = sidebar.update(cx, |sidebar, cx| {
            sidebar.log.borrow_mut().push("pick".into());
            if let Some(menu) = sidebar.menu.as_ref() {
                menu.close(window, cx);
            }
        });
    };
    let pick = Rc::new(pick);
    let content_focus = panel.clone();
    vec![
        MenuItem::new("rename", "Rename")
            .icon("pencil-edit")
            .on_select(on("rename")),
        MenuItem::new("color", "Color").icon("paint-board").submenu(
            ["gray", "red", "blue"]
                .into_iter()
                .map(|color| {
                    MenuItem::new(format!("color-{color}"), color)
                        .checked(color == "blue")
                        .on_select(on(color))
                })
                .chain([
                    MenuItem::separator(),
                    MenuItem::new("color-custom", "Custom")
                        .panel(custom.focus().clone(), move |window, cx| {
                            custom.panel(window, cx)
                        }),
                ])
                .collect(),
        ),
        MenuItem::new("icon", "Icon")
            .icon("star")
            .panel(panel, move |_, _| {
                let (click, key) = (pick.clone(), pick.clone());
                div()
                    .id("panel-content")
                    .test_support()
                    .track_focus(&content_focus)
                    .size(px(80.))
                    .on_click(move |_, window, cx| click(window, cx))
                    .on_key_down(move |event, window, cx| {
                        if event.keystroke.key == "enter" {
                            key(window, cx);
                        }
                    })
                    .into_any_element()
            }),
        MenuItem::separator(),
        MenuItem::new("move-up", "Move Up")
            .icon("arrow-up")
            .disabled(true)
            .on_select(on("move-up")),
        MenuItem::new("move-down", "Move Down")
            .icon("arrow-down")
            .on_select(on("move-down")),
        MenuItem::separator(),
        MenuItem::new("delete", "Delete…")
            .icon("delete")
            .danger()
            .on_select(on("delete")),
    ]
}

impl Render for Sidebar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("sidebar")
            .test_support()
            .track_focus(&self.focus)
            .on_action(cx.listener(|sidebar, _: &Shortcut, _, _| sidebar.shortcuts += 1))
            .size_full()
            .child(
                div()
                    .id("row")
                    .test_support()
                    .tab_index(0)
                    .h(px(32.))
                    .w(px(200.))
                    .child("Research")
                    .on_click(cx.listener(|sidebar, _, _, _| sidebar.row_clicks += 1))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|sidebar, event: &MouseDownEvent, window, cx| {
                            let items = items(&sidebar.log, sidebar.panel.clone(), window, cx);
                            sidebar.menu = Some(OpenMenu::new(
                                "row",
                                items,
                                event.position,
                                sidebar.focus.clone(),
                                window,
                                cx,
                                |sidebar: &mut Sidebar| &mut sidebar.menu,
                            ));
                        }),
                    ),
            )
            .children(self.field.as_ref().map(|(focus, _)| {
                div()
                    .id("field")
                    .test_support()
                    .track_focus(focus)
                    .h(px(32.))
                    .w(px(200.))
            }))
            .children(self.menu.as_ref().map(OpenMenu::render))
    }
}

/// Turns on the window's accessibility tree, which panics in debug builds when two elements
/// claim focus, or two rows claim the highlight, in one frame.
struct Accessibility;
impl FrameObserver for Accessibility {}

struct Harness {
    handle: AnyWindowHandle,
    sidebar: gpui_kit::Entity<Sidebar>,
    _accessibility: Arc<Accessibility>,
}

impl Harness {
    fn new(cx: &mut TestAppContext) -> Self {
        cx.update(|cx| {
            gpui_kit::init(cx);
            // The opening fade would otherwise stop at its first frame, as tests draw no
            // frames on their own.
            cx.set_reduce_motion(true);
            cx.bind_keys([KeyBinding::new("cmd-b", Shortcut, None)]);
        });
        let (handle, sidebar) = cx
            .update(|cx| {
                gpui_kit::open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                            Default::default(),
                            size(px(900.), px(700.)),
                        ))),
                        ..Default::default()
                    },
                    cx,
                    |window, cx| {
                        cx.new(|cx| {
                            let focus = cx.focus_handle();
                            window.focus(&focus, cx);
                            let field = cx.focus_handle();
                            let ends =
                                cx.on_blur(&field, window, |sidebar: &mut Sidebar, _, cx| {
                                    sidebar.field = None;
                                    cx.notify();
                                });
                            Sidebar {
                                focus,
                                panel: cx.focus_handle(),
                                menu: None,
                                log: Rc::default(),
                                row_clicks: 0,
                                shortcuts: 0,
                                field: Some((field, ends)),
                            }
                        })
                    },
                )
            })
            .unwrap_or_else(|error| panic!("open test window: {error}"));
        let accessibility = Arc::new(Accessibility);
        let harness = Self {
            handle,
            sidebar,
            _accessibility: accessibility.clone(),
        };
        // Focus events only reach an active window.
        harness.step(cx, move |window, _| {
            window.observe_frames(&accessibility);
            window.activate_window();
        });
        harness
    }

    /// Draws a frame, runs `act`, then lets deferred work such as the menu's focus finish.
    fn step(&self, cx: &mut TestAppContext, act: impl FnOnce(&mut Window, &mut gpui_kit::App)) {
        cx.update_window(self.handle, |_, window, cx| {
            window.render_frame(cx);
            act(window, cx);
        })
        .unwrap_or_else(|error| panic!("update test window: {error}"));
        cx.run_until_parked();
        cx.update_window(self.handle, |_, window, cx| window.render_frame(cx))
            .unwrap_or_else(|error| panic!("update test window: {error}"));
    }

    fn check(&self, cx: &mut TestAppContext, check: impl FnOnce(&mut Window)) {
        cx.update_window(self.handle, |_, window, _| check(window))
            .unwrap_or_else(|error| panic!("read test window: {error}"));
    }

    fn press(&self, cx: &mut TestAppContext, keys: &[&str]) {
        for key in keys {
            self.step(cx, |window, cx| window.press(key, cx));
        }
    }

    fn log(&self, cx: &mut TestAppContext) -> Vec<String> {
        cx.update(|cx| self.sidebar.read(cx).log.borrow().clone())
    }

    fn wait(&self, cx: &mut TestAppContext) {
        cx.executor().advance_clock(SUBMENU_CLOSE_DELAY);
        self.step(cx, |_, _| {});
    }

    fn shows(&self, cx: &mut TestAppContext, id: &'static str) -> bool {
        let mut shown = false;
        self.check(cx, |window| shown = window.try_find(id).is_some());
        shown
    }

    fn open(&self, cx: &mut TestAppContext) {
        self.step(cx, |window, cx| window.right_click("row", cx));
        self.check(cx, |window| {
            assert_eq!(window.find("menu").focused(), Some(true));
        });
    }

    /// The menu is gone and focus is back where it was before the menu opened.
    fn assert_closed(&self, cx: &mut TestAppContext) {
        self.check(cx, |window| {
            assert!(window.try_find("menu").is_none());
            assert_eq!(window.find("sidebar").focused(), Some(true));
        });
    }
}

#[gpui_kit::test]
fn choosing_a_row_closes_the_menu_and_runs_it(cx: &mut TestAppContext) {
    let harness = Harness::new(cx);
    for item in ["rename", "move-down", "delete"] {
        harness.open(cx);
        harness.step(cx, |window, cx| {
            window.click(format!("menu-item-{item}"), cx)
        });
        harness.assert_closed(cx);
    }

    // A disabled row does nothing.
    harness.open(cx);
    harness.check(cx, |window| {
        assert_eq!(window.find("menu-item-move-up").disabled(), Some(true));
    });
    harness.step(cx, |window, cx| window.click("menu-item-move-up", cx));
    assert!(harness.shows(cx, "menu"));

    assert_eq!(harness.log(cx), ["rename", "move-down", "delete"]);
}

#[gpui_kit::test]
fn a_submenu_opens_on_hover_beside_its_row_and_checks_its_rows(cx: &mut TestAppContext) {
    let harness = Harness::new(cx);
    harness.open(cx);
    assert!(!harness.shows(cx, "menu-submenu"));

    harness.step(cx, |window, cx| window.hover("menu-item-color", cx));
    harness.check(cx, |window| {
        assert_eq!(window.find("menu-item-color").expanded(), Some(true));
        assert_eq!(window.find("menu-item-color-blue").checked(), Some(true));
        assert_eq!(window.find("menu-item-color-red").checked(), Some(false));
        // It opens to the right, overlapping the menu's edge, with its first row lined up
        // with the row it opened from.
        let (menu, submenu) = (
            window.find("menu").bounds(),
            window.find("menu-submenu").bounds(),
        );
        assert_eq!(submenu.left(), menu.right() - px(SUBMENU_OVERLAP));
        assert_eq!(
            window.find("menu-item-color-gray").bounds().top(),
            window.find("menu-item-color").bounds().top()
        );
    });
    harness.step(cx, |window, cx| window.click("menu-item-color-red", cx));

    harness.assert_closed(cx);
    assert_eq!(harness.log(cx), ["red"]);
}

#[gpui_kit::test]
fn a_panel_opens_on_hover_beside_its_row_and_its_content_picks(cx: &mut TestAppContext) {
    let harness = Harness::new(cx);
    harness.open(cx);
    harness.step(cx, |window, cx| window.hover("menu-item-icon", cx));
    harness.check(cx, |window| {
        assert_eq!(window.find("menu-item-icon").expanded(), Some(true));
        assert!(window.try_find("menu-submenu").is_none());
        let (menu, panel) = (
            window.find("menu").bounds(),
            window.find("menu-panel").bounds(),
        );
        assert_eq!(panel.left(), menu.right() - px(SUBMENU_OVERLAP));
        assert_eq!(
            panel.top() + px(PADDING),
            window.find("menu-item-icon").bounds().top()
        );
    });
    harness.step(cx, |window, cx| window.click("panel-content", cx));

    harness.assert_closed(cx);
    assert_eq!(harness.log(cx), ["pick"]);
}

#[gpui_kit::test]
fn the_keyboard_walks_the_menu_its_submenu_and_its_panel(cx: &mut TestAppContext) {
    let harness = Harness::new(cx);
    harness.open(cx);
    // Rename, Color, then into the submenu, which highlights Gray; one down is Red.
    harness.press(cx, &["down", "down", "right", "down", "enter"]);
    harness.assert_closed(cx);

    // Going up wraps to Delete and skips the disabled Move Up to reach Icon, whose panel takes
    // focus; its own keys stay in it, and Enter picks.
    harness.open(cx);
    harness.press(cx, &["up", "up", "up", "right"]);
    harness.check(cx, |window| {
        assert_eq!(window.find("panel-content").focused(), Some(true));
    });
    harness.press(cx, &["up", "left", "enter"]);
    harness.assert_closed(cx);

    assert_eq!(harness.log(cx), ["red", "pick"]);
}

#[gpui_kit::test]
fn escape_closes_the_panel_then_the_menu(cx: &mut TestAppContext) {
    let harness = Harness::new(cx);
    harness.open(cx);
    harness.press(cx, &["down", "down", "down", "right", "escape"]);
    harness.check(cx, |window| {
        assert!(window.try_find("menu-panel").is_none());
        assert_eq!(window.find("menu").focused(), Some(true));
    });
    // Up to Color: Left and Escape close its submenu too, and Escape then the menu.
    harness.press(cx, &["up"]);
    for key in ["left", "escape"] {
        harness.press(cx, &["right", key]);
        assert!(!harness.shows(cx, "menu-submenu"));
        assert!(harness.shows(cx, "menu"));
    }
    harness.press(cx, &["escape"]);
    harness.assert_closed(cx);
    assert!(harness.log(cx).is_empty());
}

#[gpui_kit::test]
fn a_press_outside_the_menu_only_closes_it(cx: &mut TestAppContext) {
    let harness = Harness::new(cx);
    harness.open(cx);
    // Over the row, left of the menu, which opened at the row's centre.
    harness.step(cx, |window, cx| {
        window.click_at("menu-backdrop", point(px(20.), px(16.)), cx)
    });

    harness.assert_closed(cx);
    assert!(harness.log(cx).is_empty());
    let row_clicks = cx.update(|cx| harness.sidebar.read(cx).row_clicks);
    assert_eq!(row_clicks, 0);
}

#[gpui_kit::test]
fn a_submenu_or_panel_closes_a_moment_after_the_pointer_leaves_its_row(cx: &mut TestAppContext) {
    let harness = Harness::new(cx);
    harness.open(cx);
    // Onto an enabled row, then onto the disabled Move Up.
    for (opener, surface) in [("color", "menu-submenu"), ("icon", "menu-panel")] {
        for row in ["menu-item-rename", "menu-item-move-up"] {
            harness.step(cx, |window, cx| {
                window.hover(format!("menu-item-{opener}"), cx)
            });
            harness.step(cx, |window, cx| window.hover(row, cx));
            assert!(harness.shows(cx, surface));
            harness.wait(cx);
            assert!(!harness.shows(cx, surface));
        }
    }

    // Reaching the submenu in time keeps it open.
    harness.step(cx, |window, cx| window.hover("menu-item-color", cx));
    harness.step(cx, |window, cx| window.hover("menu-item-rename", cx));
    harness.step(cx, |window, cx| window.hover("menu-item-color-red", cx));
    harness.wait(cx);
    assert!(harness.shows(cx, "menu-submenu"));
}

#[gpui_kit::test]
fn a_shortcut_runs_and_closes_the_menu(cx: &mut TestAppContext) {
    let harness = Harness::new(cx);
    harness.open(cx);
    harness.press(cx, &["cmd-b"]);

    harness.assert_closed(cx);
    let shortcuts = cx.update(|cx| harness.sidebar.read(cx).shortcuts);
    assert_eq!(shortcuts, 1);
}

#[gpui_kit::test]
fn tab_closes_the_menu(cx: &mut TestAppContext) {
    let harness = Harness::new(cx);
    harness.open(cx);
    harness.press(cx, &["tab"]);
    assert!(!harness.shows(cx, "menu"));
}

#[gpui_kit::test]
fn focus_returns_to_the_view_when_what_had_it_is_gone(cx: &mut TestAppContext) {
    let harness = Harness::new(cx);
    harness.step(cx, |window, cx| window.click("field", cx));
    harness.check(cx, |window| {
        assert_eq!(window.find("field").focused(), Some(true))
    });
    // Taking focus ends the field.
    harness.open(cx);
    assert!(!harness.shows(cx, "field"));

    harness.press(cx, &["escape"]);
    harness.assert_closed(cx);
}

#[gpui_kit::test]
fn a_submenu_row_opens_a_third_level_panel_and_a_shade_picks_it(cx: &mut TestAppContext) {
    let harness = Harness::new(cx);
    harness.open(cx);
    harness.step(cx, |window, cx| window.hover("menu-item-color", cx));
    harness.step(cx, |window, cx| window.hover("menu-item-color-custom", cx));
    harness.check(cx, |window| {
        let custom = window.find("menu-item-color-custom");
        assert_eq!(custom.expanded(), Some(true));
        // It opens to the right of the submenu, overlapping its edge, level with its row.
        let (submenu, panel) = (
            window.find("menu-submenu").bounds(),
            window.find("menu-panel").bounds(),
        );
        assert_eq!(panel.left(), submenu.right() - px(SUBMENU_OVERLAP));
        assert_eq!(panel.top() + px(PADDING), custom.bounds().top());
    });
    // Red's row, the second, at its sixth shade, 500: swatches are 20 pt apart by 4.
    let red = ColorName::Red.scale(500);
    let swatch = |index: f32| px(PADDING + index * 24. + 10.);
    harness.step(cx, |window, cx| {
        window.click_at("menu-panel", point(swatch(5.), swatch(1.)), cx)
    });

    harness.assert_closed(cx);
    assert_eq!(harness.log(cx), [format!("custom {}", red.to_hex())]);
}

#[gpui_kit::test]
fn the_keyboard_opens_the_third_level_and_a_hex_value_picks_it(cx: &mut TestAppContext) {
    let harness = Harness::new(cx);
    harness.open(cx);
    // Rename, Color, into the submenu on Gray, up to Custom, its last row, and into its panel,
    // which focuses the hex field.
    harness.press(cx, &["down", "down", "right", "up", "right"]);
    assert!(harness.shows(cx, "menu-panel"));
    // The field's own keys stay in it, and editing does not close the menu.
    harness.press(cx, &["cmd-a", "backspace", "left", "up"]);
    harness.step(cx, |window, cx| window.input("#3366FF", cx));
    assert!(harness.shows(cx, "menu-panel"));
    harness.press(cx, &["enter"]);

    harness.assert_closed(cx);
    let picked = Hsla::from(rgb(0x3366ff)).to_hex();
    assert_eq!(harness.log(cx), [format!("custom {picked}")]);
}

#[gpui_kit::test]
fn escape_closes_the_third_level_then_the_submenu_then_the_menu(cx: &mut TestAppContext) {
    let harness = Harness::new(cx);
    harness.open(cx);
    harness.press(cx, &["down", "down", "right", "up", "right", "escape"]);
    harness.check(cx, |window| {
        assert!(window.try_find("menu-panel").is_none());
        assert!(window.try_find("menu-submenu").is_some());
        assert_eq!(window.find("menu").focused(), Some(true));
    });
    harness.press(cx, &["escape"]);
    assert!(!harness.shows(cx, "menu-submenu"));
    assert!(harness.shows(cx, "menu"));
    harness.press(cx, &["escape"]);
    harness.assert_closed(cx);
    assert!(harness.log(cx).is_empty());
}

#[gpui_kit::test]
fn the_third_level_closes_a_moment_after_the_pointer_leaves_its_row(cx: &mut TestAppContext) {
    let harness = Harness::new(cx);
    harness.open(cx);
    harness.step(cx, |window, cx| window.hover("menu-item-color", cx));
    harness.step(cx, |window, cx| window.hover("menu-item-color-custom", cx));
    harness.step(cx, |window, cx| window.hover("menu-item-color-red", cx));
    assert!(harness.shows(cx, "menu-panel"));
    harness.wait(cx);
    assert!(!harness.shows(cx, "menu-panel"));
    assert!(harness.shows(cx, "menu-submenu"));

    // Reaching the panel in time keeps it open.
    harness.step(cx, |window, cx| window.hover("menu-item-color-custom", cx));
    harness.step(cx, |window, cx| window.hover("menu-item-color-red", cx));
    harness.step(cx, |window, cx| window.hover("menu-panel", cx));
    harness.wait(cx);
    assert!(harness.shows(cx, "menu-panel"));
}

#[gpui_kit::test]
fn the_menu_keeps_focus_when_the_third_level_closes_while_holding_it(cx: &mut TestAppContext) {
    let harness = Harness::new(cx);
    harness.open(cx);
    harness.press(cx, &["down", "down", "right", "up", "right"]);
    harness.step(cx, |window, cx| window.hover("menu-item-color-gray", cx));
    harness.wait(cx);
    harness.check(cx, |window| {
        assert!(window.try_find("menu-panel").is_none());
        assert_eq!(window.find("menu").focused(), Some(true));
    });
}
