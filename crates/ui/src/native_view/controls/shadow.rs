//! CSS box shadows in GPUI: GPUI's `blur_radius` is the gaussian's deviation, half the CSS blur,
//! so a shadow copied across as-is spreads twice as far.

use gpui_kit::{BoxShadow, Hsla, px};

/// The CSS shadow `x y blur spread color`.
pub(crate) fn css(x: f32, y: f32, blur: f32, spread: f32, color: Hsla) -> BoxShadow {
    BoxShadow::new(px(x), px(y), color)
        .blur_radius(px(blur / 2.))
        .spread_radius(px(spread))
}
