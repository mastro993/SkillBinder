//! Static chips in every variant and a toggle chip on and off, drawn in a GPUI test window.

use super::{Chip, ChipVariant, ToggleChip};
use crate::Text;
use crate::test_window::{Drawn, labels, node};
use crate::tokens::{color, radius};
use gpui_kit::component::{ActiveTheme, Theme};
use gpui_kit::test::{TestSupportExt, TestWindowExt};
use gpui_kit::{
    Context, Fill, InteractiveElement, IntoElement, ParentElement, Render, Styled, TestAppContext,
    Toggled, Window, div,
};

#[gpui_kit::test]
fn each_variant_fills_and_outlines_as_specified(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::init(cx);
        let theme = cx.theme();
        let fill = |variant: ChipVariant, theme: &Theme| {
            let mut chip = variant.apply(div(), theme);
            chip.style().background.clone()
        };
        assert_eq!(
            fill(ChipVariant::Neutral, theme),
            Some(Fill::from(theme.muted))
        );
        assert_eq!(
            fill(ChipVariant::Accent, theme),
            Some(Fill::from(color::ACCENT_SOFT.of(theme)))
        );
        assert_eq!(
            fill(ChipVariant::Outline, theme),
            Some(Fill::from(theme.background))
        );
        let mut outline = ChipVariant::Outline.apply(div(), theme);
        assert_eq!(outline.style().border_color, Some(theme.border));
        assert_eq!(
            outline.style().corner_radii.top_left,
            Some(gpui_kit::AbsoluteLength::from(gpui_kit::px(radius::FULL)))
        );
    });
}

#[gpui_kit::test]
fn a_static_chip_holds_its_label(cx: &mut TestAppContext) {
    let frame = Drawn::show(cx, |_, _| {
        div()
            .flex()
            .children(ChipVariant::ALL.map(|variant| {
                Chip::new(variant).child(Text::labeled(format!("{variant:?}"), "frontend"))
            }))
            .into_any_element()
    })
    .frame(cx);
    assert_eq!(labels(&frame, "frontend"), ChipVariant::ALL.len());
}

/// A toggle chip that turns on and off.
#[derive(Default)]
struct Updates(bool);

impl Render for Updates {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        // The click lands on the chip inside an element the test can find.
        div()
            .id("host")
            .child(
                ToggleChip::new("updates", self.0)
                    .accessibility_label("Updates")
                    .on_change(move |on, _, _, cx| {
                        view.update(cx, |view, cx| {
                            view.0 = on;
                            cx.notify();
                        })
                    })
                    .child("3"),
            )
            .test_support()
    }
}

#[gpui_kit::test]
fn a_toggle_chip_is_a_named_toggle_a_press_turns_on(cx: &mut TestAppContext) {
    let drawn = Drawn::open(cx, |_, _| Updates::default());
    let state = |cx: &mut TestAppContext| {
        let frame = drawn.frame(cx);
        let node = node(&frame, "updates").and_then(|node| frame.accessibility_node(node));
        node.map(|node| (node.label().map(str::to_owned), node.toggled()))
    };
    assert_eq!(
        state(cx),
        Some((Some("Updates".into()), Some(Toggled::False)))
    );
    drawn.step(cx, |window, cx| window.click("host", cx));
    assert_eq!(
        state(cx),
        Some((Some("Updates".into()), Some(Toggled::True)))
    );
}
