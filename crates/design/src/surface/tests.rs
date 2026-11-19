use super::{Level, Surface};
use crate::tokens::{elevation, radius, space};
use gpui_kit::component::{ActiveTheme, Theme, ThemeMode};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AbsoluteLength, Context, Fill, InteractiveElement, IntoElement, ParentElement, Render, Styled,
    TestAppContext, Window, div, px, rgb,
};

/// A drawn surface's side.
const SIDE: f32 = space::XXXL * 2.;

/// Every level side by side, each findable by its debug selector.
struct Levels;

impl Render for Levels {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .flex()
            .gap(px(space::XL))
            .p(px(space::XL))
            .bg(theme.sidebar)
            .children(Level::ALL.map(|level| {
                Surface::new(level).apply(
                    div()
                        .debug_selector(move || format!("surface-{level:?}"))
                        .size(px(SIDE)),
                    theme,
                )
            }))
    }
}

const SELECTORS: [&str; 4] = [
    "surface-Flat",
    "surface-Raised",
    "surface-Overlay",
    "surface-Modal",
];

#[gpui_kit::test]
fn every_level_draws_in_both_modes(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::init(cx);
    });
    let (_, cx) = cx.add_window_view(|_, _| Levels);
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        cx.update(|window, cx| {
            Theme::change(mode, None, cx);
            window.render_frame(cx);
        });
        for selector in SELECTORS {
            let bounds = cx.debug_bounds(selector);
            assert!(
                bounds.is_some_and(|bounds| bounds.size.width == px(SIDE)),
                "{selector} in {mode:?}"
            );
        }
    }
}

#[gpui_kit::test]
fn each_level_sets_its_fill_corners_and_layers(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::init(cx);
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            Theme::change(mode, None, cx);
            let theme = cx.theme();
            for (level, fill, corner, layers) in [
                (Level::Flat, theme.background, radius::LG, elevation::FLAT),
                (
                    Level::Raised,
                    theme.background,
                    radius::LG,
                    elevation::RAISED,
                ),
                (
                    Level::Overlay,
                    theme.popover,
                    radius::XL,
                    elevation::OVERLAY,
                ),
                (Level::Modal, theme.popover, radius::XL, elevation::MODAL),
            ] {
                let mut element = Surface::new(level).apply(div(), theme);
                let style = element.style();
                assert_eq!(style.background, Some(Fill::from(fill)), "{level:?}");
                assert_eq!(
                    style.corner_radii.top_left,
                    Some(AbsoluteLength::from(px(corner))),
                    "{level:?}"
                );
                let shadows = style.box_shadow.clone().unwrap_or_default();
                assert_eq!(shadows.len(), layers.len(), "{level:?}");
            }
        }
    });
}

#[gpui_kit::test]
fn a_flat_outline_is_an_inner_border_and_dark_mode_lifts_the_page(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::init(cx);
        let theme = cx.theme();
        let flat = Surface::new(Level::Flat);
        let mut card = flat.apply(div(), theme);
        let style = card.style();
        assert_eq!(style.border_widths.top, Some(AbsoluteLength::from(px(1.))));
        assert_eq!(style.border_color, Some(theme.border));
        assert_eq!(flat.inner_radius(), radius::LG - 1.);
        assert_eq!(Surface::new(Level::Raised).inner_radius(), radius::LG);
        Theme::change(ThemeMode::Dark, None, cx);
        let theme = cx.theme();
        // Dark mode separates by lightness: canvas, page, popover.
        assert_eq!(theme.sidebar, rgb(0x111113).into());
        assert_eq!(Level::Raised.fill(theme), rgb(0x1B1B1F).into());
        assert_eq!(Level::Overlay.fill(theme), rgb(0x222227).into());
    });
}

#[gpui_kit::test]
fn a_fill_overrides_the_level(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::init(cx);
        let theme = cx.theme();
        let accent = theme.sidebar_accent;
        let mut pill = Surface::new(Level::Raised).fill(accent).apply(div(), theme);
        assert_eq!(pill.style().background, Some(Fill::from(accent)));
    });
}
