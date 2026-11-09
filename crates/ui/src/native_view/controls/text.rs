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

/// One line of `size` px text in `weight`, with tabular figures when `tabular`, in a box exactly
/// as wide as GPUI shapes it, so whatever follows lands where Chrome puts it in the prototypes.
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
    exact(width, size, weight, features).child(
        div()
            .absolute()
            .top_0()
            .left_0()
            .whitespace_nowrap()
            .child(text),
    )
}

/// As [`run`], for a name that ends in an ellipsis once its line is too narrow for it, such as a
/// skill's name: its box may shrink below the text's width.
pub(crate) fn truncated(
    text: impl Into<SharedString>,
    size: f32,
    weight: FontWeight,
    cx: &App,
) -> Div {
    let text = text.into();
    let width = width(&text, size, weight, FontFeatures::default(), cx);
    exact(width, size, weight, FontFeatures::default())
        .min_w_0()
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .w_full()
                .truncate()
                .child(text),
        )
}

/// A box `width` wide and one `size` px line high whose width GPUI keeps to the fraction. Chrome
/// places every edge at its exact position and rounds it to the device pixel; GPUI rounds each
/// length first, and sizes text to the next whole point, so a box after a run of text could land a
/// pixel off. A length GPUI derives from an aspect ratio stays exact, so a strut 4 points high,
/// cancelled by an equal negative margin, carries the width, and a second strut the line's height.
/// The text, positioned absolutely, adds no size of its own.
fn exact(width: f32, size: f32, weight: FontWeight, features: FontFeatures) -> Div {
    // A hair over the shaped width, so GPUI never truncates the text it measured to fit.
    let width = width + 1. / 1024.;
    div()
        .relative()
        .flex()
        .flex_col()
        .flex_none()
        .text_size(px(size))
        .line_height(px(size * LINE))
        .font_weight(weight)
        .font_features(features)
        .child(
            div()
                .h(px(STRUT))
                .mb(px(-STRUT))
                .aspect_ratio(width / STRUT),
        )
        .child(div().h(px(size * LINE)))
}

/// The height of the strut that carries an exact width.
const STRUT: f32 = 4.;

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
