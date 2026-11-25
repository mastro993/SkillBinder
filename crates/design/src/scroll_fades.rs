//! Atom: the fades at a scroll view's top and bottom edges, each shown while content continues
//! past that edge, so scrolled content fades into the fill behind it instead of being cut off.

use crate::tokens::space;
use gpui_kit::{
    Bounds, Corners, Hsla, IntoElement, Pixels, RenderOnce, ScrollHandle, Styled, canvas, fill,
    linear_color_stop, linear_gradient, point, px, size,
};

/// How tall each fade is.
pub const FADE: f32 = space::XL;

/// The fades over a scroll view, placed over its whole box.
#[derive(IntoElement)]
pub struct ScrollFades {
    handle: ScrollHandle,
    fill: Hsla,
    radius: f32,
}

impl ScrollFades {
    /// Fades into `fill` over the view `handle` tracks.
    pub fn new(handle: &ScrollHandle, fill: Hsla) -> Self {
        Self {
            handle: handle.clone(),
            fill,
            radius: 0.,
        }
    }

    /// Rounds the fades' outer corners to the view's own `radius`.
    pub fn radius(self, radius: f32) -> Self {
        Self { radius, ..self }
    }
}

/// Whether content continues above and below the view `handle` tracks.
pub fn scroll_edges(handle: &ScrollHandle) -> (bool, bool) {
    edges(-handle.offset().y, handle.max_offset().y)
}

fn edges(offset: Pixels, max: Pixels) -> (bool, bool) {
    (offset > px(0.), offset < max)
}

impl RenderOnce for ScrollFades {
    fn render(self, _: &mut gpui_kit::Window, _: &mut gpui_kit::App) -> impl IntoElement {
        let Self {
            handle,
            fill: color,
            radius,
        } = self;
        // Read at paint, once the view has laid out its content: the handle may have tracked
        // other content when this rendered.
        canvas(
            |_, _, _| {},
            move |bounds, (), window, _| {
                let (above, below) = scroll_edges(&handle);
                let height = px(FADE).min(bounds.size.height / 2.);
                let strip = size(bounds.size.width, height);
                let (solid, clear) = (color, color.opacity(0.));
                let radius = px(radius);
                if above {
                    window.paint_quad(
                        fill(
                            Bounds::new(bounds.origin, strip),
                            linear_gradient(
                                180.,
                                linear_color_stop(solid, 0.),
                                linear_color_stop(clear, 1.),
                            ),
                        )
                        .corner_radii(Corners {
                            top_left: radius,
                            top_right: radius,
                            ..Corners::default()
                        }),
                    );
                }
                if below {
                    let origin = point(bounds.origin.x, bounds.bottom() - height);
                    window.paint_quad(
                        fill(
                            Bounds::new(origin, strip),
                            linear_gradient(
                                180.,
                                linear_color_stop(clear, 0.),
                                linear_color_stop(solid, 1.),
                            ),
                        )
                        .corner_radii(Corners {
                            bottom_left: radius,
                            bottom_right: radius,
                            ..Corners::default()
                        }),
                    );
                }
            },
        )
        .absolute()
        .inset_0()
    }
}

#[cfg(test)]
mod tests {
    use super::edges;
    use gpui_kit::px;

    #[test]
    fn a_fade_shows_only_where_content_continues() {
        assert_eq!(edges(px(0.), px(0.)), (false, false), "nothing to scroll");
        assert_eq!(edges(px(0.), px(80.)), (false, true), "at the top");
        assert_eq!(edges(px(40.), px(80.)), (true, true), "in between");
        assert_eq!(edges(px(80.), px(80.)), (true, false), "at the bottom");
    }
}
