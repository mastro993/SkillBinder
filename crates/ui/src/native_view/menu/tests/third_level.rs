//! The panel a submenu row opens beside it, the menu's third level.

use gpui_kit::component::{ColorName, Colorize};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{Bounds, Hsla, TestAppContext, point, px, rgb};
use skillbinder_theme::menu::{PADDING, SUBMENU_OVERLAP};

use super::Harness;

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
fn leaving_a_shade_restores_the_held_color_and_enter_picks_it(cx: &mut TestAppContext) {
    let harness = Harness::new(cx);
    harness.open(cx);
    harness.step(cx, |window, cx| window.hover("menu-item-color", cx));
    harness.step(cx, |window, cx| window.hover("menu-item-color-custom", cx));
    let mut panel = Bounds::default();
    harness.check(cx, |window| panel = window.find("menu-panel").bounds());
    let swatch = px(PADDING + 5. * 24. + 10.);
    harness.step(cx, |window, cx| {
        window.simulate_mouse_move(panel.origin + point(swatch, swatch), cx)
    });
    // Off the shades, onto the panel's padding.
    harness.step(cx, |window, cx| {
        window.simulate_mouse_move(panel.origin + point(px(2.), px(2.)), cx)
    });
    // Entering the hex field now picks the held color, not the shade, even though it is the
    // color the picker started from.
    harness.press(cx, &["right", "enter"]);

    harness.assert_closed(cx);
    let held = Hsla::from(rgb(0x2563eb)).to_hex();
    assert_eq!(harness.log(cx), [format!("custom {held}")]);
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
    harness.press(cx, &["secondary-a", "backspace", "left", "up"]);
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
