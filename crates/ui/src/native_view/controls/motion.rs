//! CSS timing functions, so the app's own overlays move as the design's CSS moves them.

use gpui_kit::base::animation::cubic_bezier;

/// CSS `ease`, a transition's default timing.
pub(crate) fn ease() -> impl Fn(f32) -> f32 {
    cubic_bezier(0.25, 0.1, 0.25, 1.)
}

/// CSS `ease-out`.
pub(crate) fn ease_out() -> impl Fn(f32) -> f32 {
    cubic_bezier(0., 0., 0.58, 1.)
}
