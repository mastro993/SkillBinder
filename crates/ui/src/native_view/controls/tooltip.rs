//! The app's tooltip: 12 px text, one line per `\n`, in a bordered box centred 6 px below its
//! element, or above it when the window's bottom is too close, 350 ms after the pointer rests on
//! it. Kit's tooltip uses 14 px text, prefers the top, and waits 500 ms.
//!
//! GPUI shows a tooltip at the pointer, so the element records where it was painted and the tooltip
//! draws its box at the app's place from there, measured with the text system before it is laid
//! out.

use std::{cell::Cell, rc::Rc, time::Duration};

use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    AnyView, App, AppContext, Bounds, Context, Div, IntoElement, ParentElement, Pixels, Point,
    Render, SharedString, Size, Stateful, StatefulInteractiveElement, Styled, TextRun, Window,
    canvas, div, font, hsla, point, px, size,
};

use super::shadow;

/// How long the pointer rests on an element before its tooltip shows.
pub(crate) const SHOW_DELAY: Duration = Duration::from_millis(350);
const FONT_SIZE: f32 = 12.;
const LINE_HEIGHT: f32 = FONT_SIZE * 1.35;
const PADDING_X: f32 = 8.;
const PADDING_Y: f32 = 4.;
const BORDER: f32 = 1.;
const MAX_WIDTH: f32 = 260.;
/// The gap between the element and the tooltip.
const GAP: f32 = 6.;
/// The closest the tooltip comes to the window's edges.
const MARGIN: f32 = 8.;

/// Gives `element` the app tooltip `text`.
pub(crate) fn tip(element: Stateful<Div>, text: impl Into<SharedString>) -> Stateful<Div> {
    let text = text.into();
    let anchor = Rc::new(Cell::new(None::<Bounds<Pixels>>));
    let record = anchor.clone();
    element
        .child(
            canvas(
                move |bounds, _, _| record.set(Some(bounds)),
                |_, (), _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
        .tooltip_show_delay(SHOW_DELAY)
        .tooltip(move |window, cx| build(&text, anchor.get(), window, cx))
}

struct Tip {
    lines: Vec<SharedString>,
    /// The box, relative to where GPUI places the tooltip.
    frame: Bounds<Pixels>,
}

fn build(
    text: &SharedString,
    anchor: Option<Bounds<Pixels>>,
    window: &mut Window,
    cx: &mut App,
) -> AnyView {
    let lines: Vec<SharedString> = text
        .split('\n')
        .map(|line| line.to_owned().into())
        .collect();
    let run = |len| TextRun {
        len,
        font: font(skillbinder_theme::system_font()),
        color: hsla(0., 0., 0., 1.),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let widths: Vec<f32> = lines
        .iter()
        .map(|line| {
            let shaped = window.text_system().shape_line(
                line.clone(),
                px(FONT_SIZE),
                &[run(line.len())],
                None,
            );
            f32::from(shaped.width)
        })
        .collect();
    let inner = MAX_WIDTH - 2. * (PADDING_X + BORDER);
    let widest = widths.iter().copied().fold(0., f32::max).min(inner);
    // A line wider than the box wraps; rows stay few, so the cast is exact.
    #[expect(clippy::cast_precision_loss)]
    let rows: f32 = widths
        .iter()
        .map(|width| (width / inner).ceil().max(1.))
        .sum::<f32>()
        .max(lines.len() as f32);
    let tip = size(
        px(widest.ceil() + 2. * (PADDING_X + BORDER)),
        px(rows * LINE_HEIGHT + 2. * (PADDING_Y + BORDER)),
    );
    let mouse = window.mouse_position();
    let anchor = anchor.unwrap_or_else(|| Bounds::new(mouse, size(px(0.), px(0.))));
    let origin = place(anchor, tip, window.viewport_size());
    // GPUI draws the tooltip one pixel below and right of the pointer.
    let frame = Bounds::new(origin - mouse - point(px(1.), px(1.)), tip);
    cx.new(|_| Tip { lines, frame }).into()
}

/// The tooltip's origin: centred below `anchor`, above it when the bottom is too close, and kept
/// `MARGIN` inside the window.
fn place(anchor: Bounds<Pixels>, tip: Size<Pixels>, window: Size<Pixels>) -> Point<Pixels> {
    let (width, height) = (f32::from(tip.width), f32::from(tip.height));
    let (window_w, window_h) = (f32::from(window.width), f32::from(window.height));
    let center = f32::from(anchor.center().x);
    let mut top = f32::from(anchor.bottom()) + GAP;
    let above = f32::from(anchor.top()) - GAP - height;
    if top + height > window_h - MARGIN && above >= MARGIN {
        top = above;
    }
    let left = (center - width / 2.)
        .min(window_w - width - MARGIN)
        .max(MARGIN);
    let top = top.min(window_h - height - MARGIN).max(MARGIN);
    point(px(left), px(top))
}

impl Render for Tip {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let frame = self.frame;
        // A 1 px root never reaches the window's edge, so GPUI keeps it at the pointer.
        div().size(px(1.)).child(
            div()
                .absolute()
                .left(frame.origin.x)
                .top(frame.origin.y)
                .w(frame.size.width)
                .px(px(PADDING_X))
                .py(px(PADDING_Y))
                .border_1()
                .border_color(theme.border)
                .rounded(px(6.))
                .bg(theme.popover)
                .shadow(vec![
                    shadow::css(0., 2., 4., -2., hsla(0., 0., 0., 0.1)),
                    shadow::css(0., 4., 6., -1., hsla(0., 0., 0., 0.1)),
                ])
                .font_family(skillbinder_theme::system_font())
                .text_size(px(FONT_SIZE))
                .line_height(px(LINE_HEIGHT))
                .text_color(theme.popover_foreground)
                .children(self.lines.iter().map(|line| div().child(line.clone()))),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        Bounds::new(point(px(x), px(y)), size(px(w), px(h)))
    }

    #[test]
    fn the_tooltip_sits_centred_below_and_flips_above_near_the_bottom() {
        let window = size(px(1180.), px(728.));
        let tip = size(px(100.), px(26.));
        assert_eq!(
            place(rect(500., 100., 40., 20.), tip, window),
            point(px(470.), px(126.))
        );
        assert_eq!(
            place(rect(500., 690., 40., 20.), tip, window),
            point(px(470.), px(658.)),
            "too close to the bottom, so above"
        );
        assert_eq!(
            place(rect(1160., 100., 20., 20.), tip, window),
            point(px(1072.), px(126.)),
            "kept 8 inside the right edge"
        );
    }
}
