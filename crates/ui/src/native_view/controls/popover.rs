//! A popover anchored to a trigger: the menu surface with its hairline ring and drop shadow, and
//! the element that places it 4 px below the trigger, or above it when the window's bottom is too
//! close, kept 8 px inside the window. Kit's popover keeps 4 px from the window's edges, so the
//! app places its own.

use std::rc::Rc;

use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    AnyElement, App, Bounds, Div, Element, ElementId, GlobalElementId, InspectorElementId,
    InteractiveElement, IntoElement, LayoutId, ParentElement, Pixels, Point, Position, Size,
    Stateful, Style, Styled, Window, div, hsla, point, px,
};
use skillbinder_theme::menu::{LINE_INK, RADIUS, WINDOW_MARGIN, surface as menu_surface};

use super::shadow;

/// The space between the trigger and the popover.
const OFFSET: f32 = 4.;
/// The hairline ring around the surface.
const RING: f32 = 1.;

/// Which edge of the trigger the popover lines up with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Align {
    /// Its leading edge, as in a grid card.
    Start,
    /// Its trailing edge, as in a list row.
    End,
}

/// Where a popover opens from, in window coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Anchor {
    pub trigger: Bounds<Pixels>,
    /// The edge an upward popover sits above: the trigger's top, or its whole card's.
    pub above: Pixels,
    pub align: Align,
}

/// The popover's origin for its `size`: below the trigger, else above, else beside it, kept
/// inside the window's margin.
fn origin(anchor: Anchor, size: Size<Pixels>, window: Size<Pixels>) -> Point<Pixels> {
    let trigger = anchor.trigger;
    let (width, height) = (f32::from(size.width), f32::from(size.height));
    let (window_w, window_h) = (f32::from(window.width), f32::from(window.height));
    let (left_edge, right_edge) = (f32::from(trigger.left()), f32::from(trigger.right()));
    let (top_edge, bottom_edge) = (f32::from(trigger.top()), f32::from(trigger.bottom()));
    let mut left = match anchor.align {
        Align::Start => left_edge,
        Align::End => right_edge - width,
    };
    let above = f32::from(anchor.above) - OFFSET - height;
    let top = if bottom_edge + OFFSET + height <= window_h - WINDOW_MARGIN {
        bottom_edge + OFFSET
    } else if above >= WINDOW_MARGIN {
        above
    } else {
        left = if left_edge - WINDOW_MARGIN - width >= WINDOW_MARGIN {
            left_edge - WINDOW_MARGIN - width
        } else {
            right_edge + WINDOW_MARGIN
        };
        (top_edge + bottom_edge) / 2. - height / 2.
    };
    point(
        px(left
            .min(window_w - width - WINDOW_MARGIN)
            .max(WINDOW_MARGIN)),
        px(top
            .min(window_h - height - WINDOW_MARGIN)
            .max(WINDOW_MARGIN)),
    )
}

/// The popover box `width` wide, with `padding`, on the menu surface. Its 1 px ring and drop
/// shadow lie outside it, as CSS draws them; the caller adds handlers to the returned ring.
pub(crate) fn surface(
    id: impl Into<ElementId>,
    width: f32,
    padding: f32,
    content: impl IntoElement,
    cx: &App,
) -> Stateful<Div> {
    let theme = cx.theme();
    let ink = hsla(0., 0., 0., 0.1);
    div()
        .id(id)
        .p(px(RING))
        .rounded(px(RADIUS + RING))
        .bg(theme.foreground.alpha(LINE_INK))
        // CSS `0 10px 15px -3px`, then `0 4px 6px -4px`, from the box the ring surrounds.
        .shadow(vec![
            shadow::css(0., 4., 6., -4. - RING, ink),
            shadow::css(0., 10., 15., -3. - RING, ink),
        ])
        .child(
            div()
                .w(px(width))
                .p(px(padding))
                .rounded(px(RADIUS))
                .bg(menu_surface(theme))
                .text_color(theme.foreground)
                .child(content),
        )
}

/// Places its child at [`origin`] for the anchor `anchor` returns when the child is laid out;
/// draws nothing while there is none. Wrap it in `deferred` so it draws above the page.
pub(crate) fn placed(
    anchor: impl Fn() -> Option<Anchor> + 'static,
    child: impl IntoElement,
) -> Placed {
    Placed {
        anchor: Rc::new(anchor),
        child: child.into_any_element(),
    }
}

pub(crate) struct Placed {
    anchor: Rc<dyn Fn() -> Option<Anchor>>,
    child: AnyElement,
}

impl IntoElement for Placed {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Placed {
    type RequestLayoutState = LayoutId;
    /// Whether the child was placed, and so is painted.
    type PrepaintState = bool;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, LayoutId) {
        let child = self.child.request_layout(window, cx);
        let style = Style {
            position: Position::Absolute,
            ..Style::default()
        };
        (window.request_layout(style, [child], cx), child)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut LayoutId,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        let Some(anchor) = (self.anchor)() else {
            return false;
        };
        let ringed = window.layout_bounds(*child).size;
        let ring = px(RING);
        let size = Size {
            width: ringed.width - ring * 2.,
            height: ringed.height - ring * 2.,
        };
        let at = origin(anchor, size, window.viewport_size()) - point(ring, ring);
        window.with_element_offset(at - bounds.origin, |window| {
            self.child.prepaint(window, cx);
        });
        true
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut LayoutId,
        placed: &mut bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        if *placed {
            self.child.paint(window, cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::size;

    fn anchor(x: f32, y: f32, w: f32, h: f32, align: Align) -> Anchor {
        Anchor {
            trigger: Bounds::new(point(px(x), px(y)), size(px(w), px(h))),
            above: px(y),
            align,
        }
    }

    const WINDOW: Size<Pixels> = Size {
        width: px(1180.),
        height: px(728.),
    };

    /// `at` matches `(x, y)` within float rounding.
    fn at(anchor: Anchor, height: f32, x: f32, y: f32) {
        let at = origin(anchor, size(px(300.), px(height)), WINDOW);
        let near = |a: Pixels, b: f32| (f32::from(a) - b).abs() < 0.01;
        assert!(near(at.x, x) && near(at.y, y), "{at:?} is not ({x}, {y})");
    }

    #[test]
    fn a_row_popover_opens_below_aligned_to_the_trigger_end() {
        at(
            anchor(1012.59, 292.83, 136.41, 26., Align::End),
            285.,
            849.,
            322.83,
        );
    }

    #[test]
    fn near_the_bottom_it_opens_above_and_a_grid_one_keeps_8_from_the_edge() {
        at(
            anchor(1090., 647.39, 59., 26., Align::End),
            285.,
            849.,
            358.39,
        );
        at(
            anchor(955.5, 179.59, 189.66, 26., Align::Start),
            233.,
            872.,
            209.59,
        );
    }

    #[test]
    fn with_no_room_above_or_below_it_opens_beside() {
        at(anchor(1090., 300., 59., 26., Align::End), 700., 782., 8.);
    }
}
