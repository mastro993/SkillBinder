//! One line of text as wide as its glyph advances. A GPUI text element rounds its width up to a
//! whole pixel, and layout rounds a measured size up again to a device pixel, where CSS keeps the
//! width fractional, so a pill, chip, or button that hugs its text grows by up to 1 px and moves
//! what follows it. This element authors its shaped width, which layout rounds to the nearest
//! device pixel. With `text_ellipsis` in its style it may shrink, and then ends in `…`.

use gpui_kit::{
    App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement, LayoutId,
    Pixels, ShapedLine, SharedString, Size, Style, TextOverflow, TextRun, TruncateFrom, Window, px,
};
use std::panic::Location;

/// `text` on one line, sized to its shaped width in the inherited text style.
pub(crate) fn exact_text(text: impl Into<SharedString>) -> ExactText {
    ExactText(text.into())
}

pub(crate) struct ExactText(SharedString);

/// What layout measured, for painting.
pub(crate) struct Measured {
    line: ShapedLine,
    run: TextRun,
    font_size: Pixels,
    line_height: Pixels,
    /// The affix that ends a line cut to fit, when the style asks for one.
    affix: Option<SharedString>,
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
        None
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
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
        let run = style.to_run(self.0.len());
        let line = window.text_system().shape_line(
            self.0.clone(),
            font_size,
            std::slice::from_ref(&run),
            None,
        );
        let affix = style.text_overflow.map(|overflow| match overflow {
            TextOverflow::Truncate(affix)
            | TextOverflow::TruncateStart(affix)
            | TextOverflow::TruncateMiddle(affix) => affix,
        });
        // An authored width rounds to the nearest device pixel; a measured one would round up.
        let defaults = Style::default();
        let layout = Style {
            size: Size {
                width: line.width.into(),
                height: line_height.into(),
            },
            // Text that may end in an ellipsis shrinks to the room it has; other text keeps
            // its width, as unwrapped CSS text does.
            min_size: Size {
                width: affix
                    .as_ref()
                    .map_or(defaults.min_size.width, |_| px(0.).into()),
                ..defaults.min_size
            },
            flex_shrink: if affix.is_some() { 1. } else { 0. },
            ..defaults
        };
        let layout = window.request_layout(layout, [], cx);
        let measured = Measured {
            line,
            run,
            font_size,
            line_height,
            affix,
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
        // Layout rounds the authored width to a device pixel, possibly down; only less room than
        // that rounded width means the line was shrunk.
        let fits = window.pixel_snap(measured.line.width) <= bounds.size.width;
        let cut = measured.affix.as_ref().filter(|_| !fits);
        let line = match cut {
            None => measured.line.clone(),
            Some(affix) => {
                let mut wrapper = cx
                    .text_system()
                    .line_wrapper(measured.run.font.clone(), measured.font_size);
                let (text, runs) = wrapper.truncate_line(
                    self.0.clone(),
                    bounds.size.width,
                    affix,
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
