//! The panel a submenu row opens beside it, the menu's third level.

use gpui_kit::TestAppContext;
use gpui_kit::px;
use gpui_kit::test::TestWindowExt;

use super::super::{PADDING, SUBMENU_OVERLAP};
use super::Harness;

#[gpui_kit::test]
fn a_submenu_row_opens_a_third_level_panel_and_its_content_picks(cx: &mut TestAppContext) {
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
    harness.step(cx, |window, cx| window.click("custom-content", cx));

    harness.assert_closed(cx);
    assert_eq!(harness.log(cx), ["custom"]);
}

#[gpui_kit::test]
fn the_keyboard_opens_the_third_level_and_its_content_picks(cx: &mut TestAppContext) {
    let harness = Harness::new(cx);
    harness.open(cx);
    // Rename, Color, into the submenu on Gray, up to Custom, its last row, and into its panel,
    // which focuses its content.
    harness.press(cx, &["down", "down", "right", "up", "right"]);
    harness.check(cx, |window| {
        assert_eq!(window.find("custom-content").focused(), Some(true));
    });
    // The panel's own keys stay in it.
    harness.press(cx, &["left", "up"]);
    assert!(harness.shows(cx, "menu-panel"));
    harness.press(cx, &["enter"]);

    harness.assert_closed(cx);
    assert_eq!(harness.log(cx), ["custom"]);
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
