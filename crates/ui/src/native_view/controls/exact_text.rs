//! One line of text as wide as its glyph advances. A GPUI text element rounds its width up to a
//! whole pixel, and layout rounds a measured size up again to a device pixel, where CSS keeps the
//! width fractional, so a pill, chip, or button that hugs its text grows by up to 1 px and moves
//! what follows it. This element authors its shaped width, which layout rounds to the nearest
//! device pixel. With `text_ellipsis` in its style it may shrink, and then ends in `…`. Given an
//! id, it is a label in the accessibility tree, as GPUI's own text is, so assistive technology
//! reads it and the native-test bridge reports its layout bounds.

use gpui_kit::{
    App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement, LayoutId,
    Pixels, Role, ShapedLine, SharedString, Size, Style, TextOverflow, TextRun, TruncateFrom,
    Window, accesskit, px,
};
use std::panic::Location;

/// `text` on one line, sized to its shaped width in the inherited text style.
pub(crate) fn exact_text(text: impl Into<SharedString>) -> ExactText {
    ExactText {
        text: text.into(),
        id: None,
    }
}

pub(crate) struct ExactText {
    text: SharedString,
    id: Option<ElementId>,
}

impl ExactText {
    /// The text as an accessible label with id `id`.
    pub(crate) fn named(self, id: impl Into<ElementId>) -> Self {
        Self {
            id: Some(id.into()),
            ..self
        }
    }
}

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
        self.id.clone()
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn a11y_role(&self) -> Option<Role> {
        self.id.as_ref().map(|_| Role::Label)
    }

    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        node.set_value(self.text.to_string());
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
        let line = window.text_system().shape_line(
            self.text.clone(),
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
        let shrunk = shrunk(
            measured.line.width,
            bounds.size.width,
            window.scale_factor(),
        );
        let cut = measured.affix.as_ref().filter(|_| shrunk);
        let line = match cut {
            None => measured.line.clone(),
            Some(affix) => {
                let mut wrapper = cx
                    .text_system()
                    .line_wrapper(measured.run.font.clone(), measured.font_size);
                let (text, runs) = wrapper.truncate_line(
                    self.text.clone(),
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

/// Whether a line `width` wide was given less `room` than its own width. Layout rounds the authored
/// width to the nearest device pixel at `scale`, halves toward zero, so a line whose room is that
/// rounded width still fits even when it is narrower than the line.
fn shrunk(width: Pixels, room: Pixels, scale: f32) -> bool {
    let device = f32::from(width) * scale;
    let rounded = (device - 0.5).ceil().max(0.) / scale;
    room < px(rounded)
}

#[cfg(test)]
mod tests {
    use super::{exact_text, shrunk};
    use gpui_kit::{Element, ElementId, Role, accesskit, px};

    #[test]
    fn a_named_line_is_an_accessible_label_with_its_text() {
        let named = exact_text("Switching…").named("activation-strip-label");
        assert_eq!(named.id(), Some(ElementId::from("activation-strip-label")));
        assert_eq!(named.a11y_role(), Some(Role::Label));
        let mut node = accesskit::Node::new(Role::Label);
        named.write_a11y_info(&mut node);
        assert_eq!(node.value(), Some("Switching…"));
        let unnamed = exact_text("Switching…");
        assert_eq!(unnamed.id(), None);
        assert_eq!(unnamed.a11y_role(), None);
    }

    #[test]
    fn a_line_keeps_its_text_in_its_own_rounded_width() {
        // "Switched to Frontend" shapes to 129.72 px; layout gives it 129.5 at 2x.
        assert!(!shrunk(px(129.72), px(129.5), 2.));
        assert!(
            !shrunk(px(129.75), px(129.5), 2.),
            "a half rounds toward zero"
        );
        assert!(shrunk(px(129.72), px(129.), 2.));
        assert!(!shrunk(px(129.72), px(200.), 2.));
    }
}
