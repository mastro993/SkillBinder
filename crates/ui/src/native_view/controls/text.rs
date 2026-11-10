//! The prototypes' type: every line box is 1.4 times its font size, since GPUI's default line
//! height (about 1.618) would make rows taller than the prototypes', and counts use tabular
//! figures. A run of text takes the width Chrome gives it in the prototypes.

use std::sync::Arc;

use gpui_kit::{
    App, Div, ElementId, FontFeatures, FontWeight, ParentElement, SharedString, Styled, StyledText,
    TextRun, WindowTextSystem, div, font, hsla, px, relative,
};

/// The prototypes' line height, as a multiple of the font size.
const LINE: f32 = 1.4;

/// The prototypes' weight 500. font-kit reads the system font's Medium face as weight 530, so a
/// request for 500 falls back to Regular under CSS font matching; any weight above 500 and at
/// most 530 picks Medium.
pub(crate) const MEDIUM: FontWeight = FontWeight(520.);

/// The wght of the system font's named Regular and Medium instances, from the `fvar` table of
/// `/System/Library/Fonts/SFNS.ttf`. GPUI draws [`MEDIUM`] with the Medium instance.
const REGULAR_WGHT: f32 = 400.;
const MEDIUM_WGHT: f32 = 510.;
/// The wght Chrome draws CSS weight 500 at, from the system font's variable wght axis.
const CSS_MEDIUM_WGHT: f32 = 500.;
/// How far CSS weight 500 lies from the Regular instance towards the Medium one on the wght axis.
const MEDIUM_SHARE: f32 = (CSS_MEDIUM_WGHT - REGULAR_WGHT) / (MEDIUM_WGHT - REGULAR_WGHT);

/// A hair over a run's laid width, so the text GPUI measured to fit its box always fits.
const HAIR: f32 = 1. / 1024.;
/// The height of the strut that carries an exact width.
const STRUT: f32 = 4.;

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
/// as wide as Chrome lays it out, so whatever follows lands where the prototypes put it.
pub(crate) fn run(
    text: impl Into<SharedString>,
    size: f32,
    weight: FontWeight,
    tabular: bool,
    cx: &App,
) -> Div {
    let features = if tabular {
        self::tabular()
    } else {
        FontFeatures::default()
    };
    set(
        text.into(),
        Face::new(size, weight, features),
        Fit::Whole,
        cx,
    )
}

/// As [`run`], on a line box as high as the font size, as a small button's label is set.
pub(crate) fn tight(text: impl Into<SharedString>, size: f32, weight: FontWeight, cx: &App) -> Div {
    let face = Face {
        line: size,
        ..Face::new(size, weight, FontFeatures::default())
    };
    set(text.into(), face, Fit::Whole, cx)
}

/// As [`run`], for a name that ends in an ellipsis once its line is too narrow for it, such as a
/// skill's name: its box shrinks below the text's width when the line has no room for it.
pub(crate) fn truncated(
    text: impl Into<SharedString>,
    size: f32,
    weight: FontWeight,
    cx: &App,
) -> Div {
    let face = Face::new(size, weight, FontFeatures::default());
    set(text.into(), face, Fit::Truncate, cx)
}

/// `text` set in `face`, in a box as wide as Chrome lays it out, named for the gpui MCP.
fn set(text: SharedString, face: Face, fit: Fit, cx: &App) -> Div {
    let widths = Widths::of(&text, &face, cx);
    let id = format!("text-{text}");
    observed(exact(widths, face, StyledText::new(text), fit), id)
}

/// How a run is set: its font size, line box, weight, and font features.
#[derive(Clone)]
struct Face {
    size: f32,
    line: f32,
    weight: FontWeight,
    features: FontFeatures,
}

impl Face {
    /// `size` px text on the prototypes' line box of 1.4 times it.
    fn new(size: f32, weight: FontWeight, features: FontFeatures) -> Self {
        Self {
            size,
            line: size * LINE,
            weight,
            features,
        }
    }
}

/// A run's widths in points: `laid`, the width Chrome gives it, which sizes its box, and `drawn`,
/// the width of the glyphs GPUI paints.
#[derive(Clone, Copy)]
struct Widths {
    laid: f32,
    drawn: f32,
}

impl Widths {
    /// GPUI shapes weight 500 with the Medium instance, while Chrome draws it from the wght axis
    /// at 500, with narrower advances; such a run is laid out as Chrome lays it, see [`between`].
    fn of(text: &SharedString, face: &Face, cx: &App) -> Self {
        let drawn = shaped(text, face, face.weight, cx);
        if face.weight != MEDIUM {
            return Self { laid: drawn, drawn };
        }
        let regular = shaped(text, face, FontWeight::NORMAL, cx);
        Self {
            laid: between(regular, drawn),
            drawn,
        }
    }

    /// How many times as wide as its box the text is, so it holds every glyph GPUI draws.
    fn overhang(self) -> f32 {
        if self.laid > 0. {
            (self.drawn / self.laid).max(1.)
        } else {
            1.
        }
    }
}

/// A weight 500 width from the Regular and Medium ones: an advance moves linearly along the wght
/// axis between the two instances, so it lies [`MEDIUM_SHARE`] of the way from one to the other.
fn between(regular: f32, medium: f32) -> f32 {
    regular + MEDIUM_SHARE * (medium - regular)
}

/// What a run does once its box is narrower than its text.
#[derive(Clone, Copy)]
enum Fit {
    /// Nothing: its box keeps the width it was laid out for.
    Whole,
    /// Its box shrinks like a CSS flex item, and the text ends in an ellipsis.
    Truncate,
}

/// `text` in a box `widths.laid` wide and one line of `face` high whose width GPUI keeps to the
/// fraction. Chrome places every edge at its exact position and rounds it to the device pixel;
/// GPUI rounds each length first, and sizes text to the next whole point, so a box after a run of
/// text could land a pixel off. A length GPUI derives from an aspect ratio stays exact, so a strut
/// 4 points high, cancelled by an equal negative margin, carries the width, and a second strut the
/// line's height. The text, positioned absolutely, adds no size of its own. It is as much wider
/// than the box as its glyphs are wider than the laid width, so GPUI neither wraps nor ellipsizes
/// the run, even under an ancestor that truncates, until the box itself shrinks.
fn exact(widths: Widths, face: Face, text: StyledText, fit: Fit) -> Div {
    let line = div()
        .absolute()
        .top_0()
        .left_0()
        .w(relative(widths.overhang()));
    let (line, shrink) = match fit {
        Fit::Whole => (line.whitespace_nowrap(), 0.),
        Fit::Truncate => (line.truncate(), 1.),
    };
    div()
        .relative()
        .flex()
        .flex_col()
        .flex_none()
        .flex_shrink(shrink)
        .min_w_0()
        .text_size(px(face.size))
        .line_height(px(face.line))
        .font_weight(face.weight)
        .font_features(face.features)
        .child(
            div()
                .h(px(STRUT))
                .mb(px(-STRUT))
                .aspect_ratio((widths.laid + HAIR) / STRUT),
        )
        .child(div().h(px(face.line)))
        .child(line.child(text))
}

/// `text`'s width as Chrome lays it out in the system font.
pub(crate) fn width(
    text: &SharedString,
    size: f32,
    weight: FontWeight,
    features: FontFeatures,
    cx: &App,
) -> f32 {
    Widths::of(text, &Face::new(size, weight, features), cx).laid
}

/// `text`'s width as GPUI shapes it in `face`'s system font at `weight`.
fn shaped(text: &SharedString, face: &Face, weight: FontWeight, cx: &App) -> f32 {
    let mut font = font(skillbinder_theme::system_font());
    font.weight = weight;
    font.features = face.features.clone();
    let shaped = WindowTextSystem::new(cx.text_system().clone()).shape_line(
        text.clone(),
        px(face.size),
        &[TextRun {
            len: text.len(),
            font,
            color: hsla(0., 0., 0., 1.),
            background_color: None,
            underline: None,
            strikethrough: None,
        }],
        None,
    );
    f32::from(shaped.width)
}

/// Names `element` `id` under `native-test`, so the gpui MCP reports its layout box to a parity
/// check; the name changes nothing else. Elsewhere it returns `element` as it is.
#[cfg(feature = "native-test")]
pub(crate) fn observed<E: gpui_kit::InteractiveElement>(
    mut element: E,
    id: impl Into<ElementId>,
) -> E {
    element.interactivity().element_id = Some(id.into());
    element
}

#[cfg(not(feature = "native-test"))]
pub(crate) fn observed<E>(element: E, _: impl Into<ElementId>) -> E {
    element
}

/// Tabular figures, so counts keep their width as they change.
pub(crate) fn tabular() -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".into(), 1)]))
}

#[cfg(test)]
mod tests;
