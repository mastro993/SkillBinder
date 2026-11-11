//! A 1 px dashed outline around a fully rounded box, drawn as Chrome draws the prototypes': 16
//! dashes of 3 px with even gaps. GPUI's dashed border draws 2 px dashes with 1 px gaps.

use gpui_kit::{Bounds, Canvas, Hsla, PathBuilder, Pixels, Point, Styled, canvas, point, px};

/// How many dashes go around the box.
const DASHES: f32 = 16.;
/// One dash's length.
const DASH: f32 = 3.;
/// The outline's width.
const WIDTH: f32 = 1.;

/// The outline, covering its parent, which must be `relative`.
pub(crate) fn outline(color: Hsla) -> Canvas<()> {
    canvas(
        |_, _, _| {},
        move |bounds, (), window, _| {
            if let Some(path) = path(bounds) {
                window.paint_path(path, color);
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// The dashed stroke along the box's edge, half the width inside it, starting where the top
/// edge leaves the leading curve and running clockwise.
fn path(bounds: Bounds<Pixels>) -> Option<gpui_kit::Path<Pixels>> {
    let inset = WIDTH / 2.;
    let (left, top) = (
        f32::from(bounds.left()) + inset,
        f32::from(bounds.top()) + inset,
    );
    let (width, height) = (
        f32::from(bounds.size.width) - WIDTH,
        f32::from(bounds.size.height) - WIDTH,
    );
    let radius = width.min(height) / 2.;
    let straight_x = width - 2. * radius;
    let straight_y = height - 2. * radius;
    let perimeter = 2. * (straight_x + straight_y) + std::f32::consts::TAU * radius;
    let gap = perimeter / DASHES - DASH;
    let at = |x: f32, y: f32| -> Point<Pixels> { point(px(left + x), px(top + y)) };
    let arc = |builder: &mut PathBuilder, to: Point<Pixels>| {
        builder.arc_to(point(px(radius), px(radius)), px(0.), false, true, to);
    };
    let mut builder = PathBuilder::stroke(px(WIDTH)).dash_array(&[px(DASH), px(gap)]);
    builder.move_to(at(radius, 0.));
    builder.line_to(at(width - radius, 0.));
    arc(&mut builder, at(width, radius));
    builder.line_to(at(width, height - radius));
    arc(&mut builder, at(width - radius, height));
    builder.line_to(at(radius, height));
    arc(&mut builder, at(0., height - radius));
    builder.line_to(at(0., radius));
    arc(&mut builder, at(radius, 0.));
    builder.build().ok()
}
