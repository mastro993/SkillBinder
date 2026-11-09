//! The prototypes' type: every line box is 1.4 times its font size, since GPUI's default line
//! height (about 1.618) would make rows taller than the prototypes', and counts use tabular
//! figures.

use std::sync::Arc;

use gpui_kit::{FontFeatures, FontWeight, Styled, px};

/// The prototypes' line height, as a multiple of the font size.
const LINE: f32 = 1.4;

/// The prototypes' weight 500. font-kit reads the system font's Medium face as weight 530, so a
/// request for 500 falls back to Regular under CSS font matching; any weight above 500 and at
/// most 530 picks Medium.
pub(crate) const MEDIUM: FontWeight = FontWeight(520.);

/// Sets `size` px text on a line box of 1.4 times it.
pub(crate) fn sized<E: Styled>(element: E, size: f32) -> E {
    element.text_size(px(size)).line_height(px(size * LINE))
}

/// How much shorter GPUI draws the line box of `size` px text than the prototypes do at `scale`.
/// Chrome keeps a line box in 1/64 px units, so 1.4 × 14 px stays 19.59375, while GPUI snaps it to
/// the device pixel, 19.5 at 2x. Callers pad by it so what stacks under a line lands where the
/// prototypes put it.
pub(crate) fn line_shortfall(size: f32, scale: f32) -> f32 {
    let line = size * LINE;
    let chrome = (line * 64.).floor() / 64.;
    // GPUI rounds a device length half toward zero.
    let device = ((line * scale).abs() - 0.5).ceil();
    chrome - device / scale
}

/// Tabular figures, so counts keep their width as they change.
pub(crate) fn tabular() -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".into(), 1)]))
}

#[cfg(test)]
mod tests {
    use super::line_shortfall;

    #[test]
    fn line_shortfall_is_what_device_snapping_takes_from_chromes_line_box() {
        // 14 px × 1.4: Chrome keeps 19.59375; GPUI snaps to 19.5 at 2x and 20 at 1x.
        assert_eq!(line_shortfall(14., 2.), 0.09375);
        assert_eq!(line_shortfall(14., 1.), -0.40625);
        // 12.5 px × 1.4 = 17.5 is whole device pixels at 2x and in 1/64 px.
        assert_eq!(line_shortfall(12.5, 2.), 0.);
    }
}
