//! A 2 px progress line along the bottom of a fully rounded surface, with no track. GPUI clips
//! only to rectangles, so the line is a pill-shaped quad as tall as the surface, cut down to its
//! bottom 2 px and to the share done: its left end then follows the surface's curve, as a line
//! clipped by a rounded CSS box does. Kit `Progress` draws a track and rounded ends.

use gpui_kit::{Div, Hsla, ParentElement, Pixels, Styled, div, px, relative};

/// Thickness of the line.
const THICKNESS: f32 = 2.;

/// The line for `percent` along the bottom of a `height` pill, in `color`, or `None` at 0%. The
/// pill must be `relative()`.
pub(crate) fn pill_progress(percent: u8, height: Pixels, color: Hsla) -> Option<Div> {
    let share = f32::from(percent.min(100)) / 100.;
    (share > 0.).then(|| {
        div()
            .absolute()
            .left_0()
            .bottom_0()
            .h(px(THICKNESS))
            .w(relative(share))
            .overflow_hidden()
            .child(
                div()
                    .absolute()
                    .left_0()
                    .bottom_0()
                    .h(height)
                    .w(relative(1. / share))
                    .rounded_full()
                    .bg(color),
            )
    })
}
