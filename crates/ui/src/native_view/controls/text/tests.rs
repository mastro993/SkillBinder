use std::{cell::RefCell, rc::Rc};

use super::{Face, Fit, MEDIUM, MEDIUM_SHARE, Widths, between, exact, line_shortfall, shaped};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AnyWindowHandle, AppContext, Bounds, Context, FontFeatures, FontWeight, IntoElement,
    ParentElement, Render, Styled, StyledText, TestAppContext, TextLayout, Window, WindowBounds,
    WindowOptions, div, px, size,
};

#[test]
fn line_shortfall_is_what_device_snapping_takes_from_chromes_line_box() {
    // 14 px × 1.4: Chrome keeps 19.59375; GPUI snaps to 19.5 at 2x and 20 at 1x.
    assert_eq!(line_shortfall(14., 2.), 0.09375);
    assert_eq!(line_shortfall(14., 1.), -0.40625);
    // 12.5 px × 1.4 = 17.5 is whole device pixels at 2x and in 1/64 px.
    assert_eq!(line_shortfall(12.5, 2.), 0.);
}

#[test]
fn weight_500_is_laid_out_as_chrome_draws_it_from_the_wght_axis() {
    // wght 500 is 100 of the 110 between the Regular instance, 400, and the Medium one, 510.
    assert!((MEDIUM_SHARE - 100. / 110.).abs() < 1e-6);
    // "Clear filters" at 12 px: CoreText's Regular and Medium widths, then Chrome's at weight 500.
    assert!((between(65.250, 67.322) - 67.14).abs() < 0.01);
    // "Inactive" at 12.5 px, as CoreText shapes it at each weight GPUI draws.
    let shaped = |weight: FontWeight| {
        if weight == FontWeight::NORMAL {
            45.660
        } else if weight == MEDIUM {
            47.093
        } else {
            48.5
        }
    };
    let medium = Widths::at(MEDIUM, shaped);
    assert!((medium.laid - 46.97).abs() < 0.01, "Chrome's width");
    assert_eq!(medium.drawn, 47.093, "drawn with the Medium face");
    for weight in [FontWeight::NORMAL, FontWeight::SEMIBOLD] {
        let other = Widths::at(weight, shaped);
        assert_eq!(
            other.laid,
            shaped(weight),
            "{weight:?} keeps its shaped width"
        );
        assert_eq!(other.drawn, other.laid);
    }
}

/// The text of each run, as GPUI laid it out in the last frame.
#[derive(Default)]
struct Runs {
    laid: Rc<RefCell<Vec<TextLayout>>>,
}

const NAME: &str = "Inactive filters";

impl Render for Runs {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let face = Face::new(12., MEDIUM, FontFeatures::default());
        let drawn = shaped(&NAME.into(), &face, MEDIUM, cx);
        // A box a hair narrower than its glyphs, as weight 500's is.
        let widths = Widths {
            laid: drawn - 0.3,
            drawn,
        };
        let mut laid = self.laid.borrow_mut();
        laid.clear();
        let mut text = || {
            let text = StyledText::new(NAME);
            laid.push(text.layout().clone());
            text
        };
        let line = |text, fit| exact(widths, face.clone(), text, fit);
        div()
            .flex()
            .flex_col()
            // A run's text style inherits truncation from an ancestor.
            .child(div().flex().truncate().child(line(text(), Fit::Whole)))
            .child(div().flex().child(line(text(), Fit::Truncate)))
            .child(
                div()
                    .flex()
                    .w(px(drawn / 2.))
                    .child(line(text(), Fit::Truncate)),
            )
    }
}

fn open(cx: &mut TestAppContext) -> (AnyWindowHandle, Rc<RefCell<Vec<TextLayout>>>) {
    let runs = Runs::default();
    let laid = runs.laid.clone();
    let window = cx
        .update(|cx| {
            gpui_kit::init(cx);
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        Default::default(),
                        size(px(400.), px(200.)),
                    ))),
                    ..Default::default()
                },
                cx,
                |_, cx| cx.new(|_| runs),
            )
            .map(|(handle, _)| handle)
        })
        .unwrap_or_else(|error| panic!("open test window: {error}"));
    (window, laid)
}

#[gpui_kit::test]
fn a_run_wider_than_its_box_is_neither_wrapped_nor_ellipsized(cx: &mut TestAppContext) {
    let (window, laid) = open(cx);
    cx.update_window(window, |_, window, cx| window.render_frame(cx))
        .unwrap_or_else(|error| panic!("render test window: {error}"));
    let laid = laid.borrow();
    let texts: Vec<_> = laid.iter().map(TextLayout::wrapped_text).collect();
    assert_eq!(texts[0], NAME, "a whole run under a truncating ancestor");
    assert_eq!(texts[1], NAME, "a truncating run at its laid width");
    assert!(
        texts[2].ends_with('…') && texts[2].len() < NAME.len(),
        "a truncating run in half its width: {:?}",
        texts[2]
    );
}
