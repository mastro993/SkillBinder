//! A 2 px progress line along the bottom of a fully rounded surface, with no track. GPUI clips
//! only to rectangles, so the line is a pill-shaped quad as tall as the surface, cut down to its
//! bottom 2 px and to the share done: its left end then follows the surface's curve, as a line
//! clipped by a rounded CSS box does. Kit `Progress` draws a track and rounded ends. A new share
//! grows the line over 200 ms, ease-out, as the design's CSS width transition does.

use super::motion::ease_out;
use gpui_kit::{
    Animation, AnimationExt, AnyElement, Div, Hsla, IntoElement, ParentElement, Pixels, Styled,
    div, px, relative,
};
use std::time::Duration;

/// Thickness of the line.
const THICKNESS: f32 = 2.;
/// How long the line takes to reach a new share.
const GROWTH: Duration = Duration::from_millis(200);

/// The line along the bottom of a `height` pill in `color`, growing from `from` to `to` percent,
/// or `None` at 0%. The pill must be `relative()`.
pub(crate) fn pill_progress(from: u8, to: u8, height: Pixels, color: Hsla) -> Option<AnyElement> {
    let share = |percent: u8| f32::from(percent.min(100)) / 100.;
    let (start, end) = (share(from), share(to));
    if end <= 0. {
        return None;
    }
    if start == end {
        return Some(line(end, height, color).into_any_element());
    }
    let grown = line(start, height, color).with_animation(
        ("pill-progress", u64::from(to)),
        Animation::new(GROWTH).with_easing(ease_out()),
        move |_, grown| line(start + (end - start) * grown, height, color),
    );
    Some(grown.into_any_element())
}

/// The line at `share` of the pill's width.
fn line(share: f32, height: Pixels, color: Hsla) -> Div {
    let band = div()
        .absolute()
        .left_0()
        .bottom_0()
        .h(px(THICKNESS))
        .w(relative(share))
        .overflow_hidden();
    if share <= 0. {
        return band;
    }
    band.child(
        div()
            .absolute()
            .left_0()
            .bottom_0()
            .h(height)
            .w(relative(1. / share))
            .rounded_full()
            .bg(color),
    )
}
