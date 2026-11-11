//! One line of text as wide as Chrome lays it out, the app's one element for a line whose box
//! must land where the prototypes put it. A GPUI text element rounds its width up to a whole
//! pixel, and layout rounds every authored length to the device pixel, where CSS keeps the width
//! fractional, so a pill, chip, or button that hugs its text would grow by up to 1 px and move
//! what follows it. This element derives its width from an aspect ratio, which layout keeps to
//! the fraction, and paints the shaped line itself, so it never wraps. Weight 500 takes Chrome's
//! width, not GPUI's, as [`width`] describes.
//!
//! The line takes its size, weight, features, and line height from the inherited text style; the
//! prototypes' type is [`sized`]. It keeps its width, or with [`ExactText::truncate`] shrinks like
//! a CSS flex item and then ends in `…`.
//!
//! Every line has an element id, so the native-test bridge reports its layout box and text in
//! every build. It is a label in the accessibility tree holding its text, unless it is
//! [`ExactText::presentational`]: an enclosing element's accessible name reads the same text,
//! as a button's does its label, and assistive technology must not read it twice.

mod width;

#[cfg(test)]
mod tests;

use std::{panic::Location, sync::Arc};

use gpui_kit::{
    App, Bounds, Element, ElementId, FontFeatures, FrameNodeData, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Length, Pixels, Role, ShapedLine, SharedString,
    Size, Style, Styled, TextRun, TruncateFrom, Window, accesskit, px,
};

pub(crate) use width::width;
use width::{Widths, measure};

/// The prototypes' line height, as a multiple of the font size.
const LINE: f32 = 1.4;
/// What ends a line cut to fit, as CSS `text-overflow: ellipsis` draws it.
const ELLIPSIS: &str = "…";

/// The line `text`, reachable as `id`, sized to the width Chrome gives it in the inherited text
/// style; a label in the accessibility tree.
pub(crate) fn exact_text(id: impl Into<ElementId>, text: impl Into<SharedString>) -> ExactText {
    ExactText {
        id: id.into(),
        text: text.into(),
        fit: Fit::Whole,
        reading: Reading::Label,
    }
}

pub(crate) struct ExactText {
    id: ElementId,
    text: SharedString,
    fit: Fit,
    reading: Reading,
}

/// What a line does once its parent gives it less room than its width.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Fit {
    /// Nothing: it keeps its width, as unwrapped CSS text does.
    Whole,
    /// Its box shrinks like a CSS flex item, and the text ends in an ellipsis.
    Truncate,
}

/// How assistive technology reads a line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Reading {
    /// As a label holding its text.
    Label,
    /// Not at all: an enclosing element's accessible name reads the same text.
    Presentational,
}

impl ExactText {
    /// Lets the line shrink below its width, ending it in `…`, as a skill's name does.
    pub(crate) fn truncate(self) -> Self {
        Self {
            fit: Fit::Truncate,
            ..self
        }
    }

    /// Leaves the line out of the accessibility tree, for text an enclosing element's accessible
    /// name already reads; the native-test bridge still reports it.
    pub(crate) fn presentational(self) -> Self {
        Self {
            reading: Reading::Presentational,
            ..self
        }
    }

    /// The room the glyphs of a line of `widths` take once layout gives its box only `room` at
    /// `scale`, so it is cut to fit and ends in `…`; `None` while it shows whole. The glyphs are
    /// as much wider than the box as the drawn width is than the laid one.
    fn cut(&self, widths: Widths, room: Pixels, scale: f32) -> Option<Pixels> {
        (self.fit == Fit::Truncate && shrunk(widths.laid, room, scale))
            .then(|| room * widths.overhang())
    }
}

/// What layout measured, for painting.
pub(crate) struct Measured {
    line: ShapedLine,
    run: TextRun,
    font_size: Pixels,
    line_height: Pixels,
    widths: Widths,
}

impl IntoElement for ExactText {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for ExactText {
    type RequestLayoutState = Measured;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn a11y_role(&self) -> Option<Role> {
        match self.reading {
            Reading::Label => Some(Role::Label),
            Reading::Presentational => None,
        }
    }

    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        node.set_value(self.text.to_string());
    }

    // A frame node of its own, apart from any accessibility node, so observers such as the
    // native-test bridge report a presentational line too.
    fn frame_node(&self) -> Option<FrameNodeData> {
        Some(FrameNodeData {
            fallback_role: Role::Label,
            ..FrameNodeData::default()
        })
    }

    fn frame_text(&self) -> Option<&str> {
        Some(&self.text)
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Measured) {
        let style = window.text_style();
        let rem = window.rem_size();
        let font_size = style.font_size.to_pixels(rem);
        let line_height = window.pixel_snap(style.line_height.to_pixels(font_size.into(), rem));
        let run = style.to_run(self.text.len());
        let (line, widths) = measure(
            &self.text,
            &run,
            font_size,
            window.text_system(),
            cx.text_system(),
        );
        let layout = window.request_layout(layout(widths.laid, line_height, self.fit), [], cx);
        let measured = Measured {
            line,
            run,
            font_size,
            line_height,
            widths,
        };
        (layout, measured)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Measured,
        _: &mut Window,
        _: &mut App,
    ) {
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        measured: &mut Measured,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let cut = self.cut(measured.widths, bounds.size.width, window.scale_factor());
        let line = match cut {
            None => measured.line.clone(),
            Some(room) => {
                let mut wrapper = cx
                    .text_system()
                    .line_wrapper(measured.run.font.clone(), measured.font_size);
                let (text, runs) = wrapper.truncate_line(
                    self.text.clone(),
                    room,
                    ELLIPSIS,
                    std::slice::from_ref(&measured.run),
                    TruncateFrom::End,
                );
                window
                    .text_system()
                    .shape_line(text, measured.font_size, &runs, None)
            }
        };
        let align = window.text_style().text_align;
        // A glyph that cannot be rasterized only leaves the line unpainted.
        let _painted = line.paint(bounds.origin, measured.line_height, align, None, window, cx);
    }
}

/// The box of a line `laid` wide and `line_height` high. Layout rounds an authored width to the
/// device pixel but keeps a width it derives from an aspect ratio to the fraction, as Chrome
/// does; the height is whole device pixels already, so the ratio gives back `laid`.
fn layout(laid: f32, line_height: Pixels, fit: Fit) -> Style {
    let defaults = Style::default();
    let height = f32::from(line_height);
    let (min_width, flex_shrink) = match fit {
        Fit::Whole => (defaults.min_size.width, 0.),
        Fit::Truncate => (px(0.).into(), 1.),
    };
    Style {
        size: Size {
            width: Length::Auto,
            height: line_height.into(),
        },
        aspect_ratio: (height > 0.).then(|| laid / height),
        min_size: Size {
            width: min_width,
            ..defaults.min_size
        },
        flex_shrink,
        ..defaults
    }
}

/// Whether layout gave a line `laid` wide less `room` than its width. Layout rounds each edge of
/// the box to the device pixel at `scale`, so a box it did not squeeze keeps at least the whole
/// device pixels of its width.
fn shrunk(laid: f32, room: Pixels, scale: f32) -> bool {
    room < px((laid * scale).floor() / scale)
}

/// Sets `size` px text on a line box 1.4 times as high, the prototypes' type, where GPUI's own
/// line height (about 1.618) would make rows taller than theirs.
pub(crate) fn sized<E: Styled>(element: E, size: f32) -> E {
    element.text_size(px(size)).line_height(px(size * LINE))
}

/// How much shorter GPUI draws the line box of `size` px text than the prototypes do at `scale`.
/// Chrome keeps a line box in 1/64 px units, so 1.4 × 14 px stays 19.59375, while GPUI snaps it to
/// the device pixel, 19.5 at 2x. Callers pad by it so what stacks under a line lands where the
/// prototypes put it.
pub(crate) fn line_shortfall(size: f32, scale: f32) -> f32 {
    let line = size * LINE;
    let chrome = (line * 64.).floor() / 64.;
    // GPUI rounds a device length half toward zero.
    let device = ((line * scale).abs() - 0.5).ceil();
    chrome - device / scale
}

/// Tabular figures, so counts keep their width as they change.
pub(crate) fn tabular() -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".into(), 1)]))
}
