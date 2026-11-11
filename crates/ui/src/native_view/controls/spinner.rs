//! The progress spinner: the retained `loading` glyph turning once every 0.9 s at a steady speed.
//! Kit `Spinner` turns in 0.8 s with easing and has no speed setting.

use super::app_icon;
use gpui_kit::{
    Animation, AnimationExt, ElementId, Hsla, IntoElement, Pixels, Styled, Transformation,
    percentage,
};
use std::time::Duration;

/// One full turn.
const TURN: Duration = Duration::from_millis(900);

/// A `size` spinner in `color`, animated under `id`.
pub(crate) fn spinner(id: impl Into<ElementId>, size: Pixels, color: Hsla) -> impl IntoElement {
    app_icon("loading")
        .size(size)
        .text_color(color)
        .with_animation(id, Animation::new(TURN).repeat(), |icon, turned| {
            icon.transform(Transformation::rotate(percentage(turned)))
        })
}
