//! The prototypes' type: every line box is 1.4 times its font size, since GPUI's default line
//! height (about 1.618) would make rows taller than the prototypes', and counts use tabular
//! figures.

use std::sync::Arc;

use gpui_kit::{
    App, Div, FontFeatures, FontWeight, ParentElement, SharedString, Styled, TextRun,
    WindowTextSystem, div, font, hsla, px,
};

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

/// One run of `size` px text in `weight`, with tabular figures when `tabular`, that advances its
/// line by its shaped width rounded to the device pixel, as Chrome lays the prototypes out, where
/// GPUI sizes a text box to the next whole point and moves whatever follows it. The run's box
/// takes the shaped width, which GPUI rounds to the device pixel; the text, up to a pixel wider,
/// paints past it on one line. Its parent sets its color and line height.
pub(crate) fn run(
    text: impl Into<SharedString>,
    size: f32,
    weight: FontWeight,
    tabular: bool,
    cx: &App,
) -> Div {
    let text = text.into();
    let features = if tabular {
        self::tabular()
    } else {
        FontFeatures::default()
    };
    let width = width(&text, size, weight, features.clone(), cx);
    div()
        .flex_none()
        .w(px(width))
        .whitespace_nowrap()
        .text_size(px(size))
        .font_weight(weight)
        .font_features(features)
        .child(text)
}

/// How much less than its whole-point text box `text` advances, as a negative trailing margin for
/// a text box that must keep GPUI's width, such as one that truncates. GPUI collapses a box of
/// text with a negative margin when its parent sizes to its content, so only a box whose parent
/// has its own width, such as a row's name, takes it.
pub(crate) fn overhang(
    text: &SharedString,
    size: f32,
    weight: FontWeight,
    features: FontFeatures,
    cx: &App,
) -> f32 {
    let width = width(text, size, weight, features, cx);
    width - width.ceil()
}

/// `text`'s width as GPUI shapes it in the system font.
pub(crate) fn width(
    text: &SharedString,
    size: f32,
    weight: FontWeight,
    features: FontFeatures,
    cx: &App,
) -> f32 {
    let mut face = font(skillbinder_theme::system_font());
    face.weight = weight;
    face.features = features;
    let shaped = WindowTextSystem::new(cx.text_system().clone()).shape_line(
        text.clone(),
        px(size),
        &[TextRun {
            len: text.len(),
            font: face,
            color: hsla(0., 0., 0., 1.),
            background_color: None,
            underline: None,
            strikethrough: None,
        }],
        None,
    );
    f32::from(shaped.width)
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
