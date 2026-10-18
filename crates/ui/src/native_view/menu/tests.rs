use super::super::custom_color::CustomColor;
use super::{MenuItem, OpenMenu, SUBMENU_CLOSE_DELAY, step};
use gpui_kit::component::Colorize;
use gpui_kit::test::{TestSupportExt, TestWindowExt};
use gpui_kit::{
    AnyWindowHandle, AppContext, Bounds, Context, FocusHandle, FrameObserver, Hsla,
    InteractiveElement, IntoElement, KeyBinding, MouseButton, MouseDownEvent, ParentElement,
    Render, StatefulInteractiveElement, Styled, Subscription, TestAppContext, Window, WindowBounds,
    WindowOptions, div, px, rgb, size,
};
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

mod menu_levels;
mod third_level;
