//! The menu and the submenu or panel beside one of its rows.

use gpui_kit::test::TestWindowExt;
use gpui_kit::{TestAppContext, point, px};
use skillbinder_theme::menu::{PADDING, SUBMENU_OVERLAP};

use super::Harness;

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
