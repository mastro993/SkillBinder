//! The width a line takes. GPUI draws a weight with the system font's nearest named instance,
//! while Chrome draws CSS weight 500 from SF Pro's variable wght axis at 500, with narrower
//! advances than the Medium instance's (wght 510). A line GPUI draws with the Medium instance is
//! therefore laid out at Chrome's width and drawn with Medium, a hair wider than its box. Any
//! other face keeps the width GPUI shapes: weight 600, drawn with Semibold at wght 590, sets a
//! little narrower than Chrome's 600 ("e2e-playwright" at 14 px: 102.69 against 102.96), within
//! the parity check's half point.

use gpui_kit::{
    App, Font, FontFeatures, FontWeight, Pixels, ShapedLine, SharedString, TextRun, TextSystem,
    WindowTextSystem, font, hsla, px,
};
use skillbinder_theme::{MEDIUM, system_font};

/// The wght of the system font's named Regular and Medium instances, from the `fvar` table of
/// `/System/Library/Fonts/SFNS.ttf`.
const REGULAR_WGHT: f32 = 400.;
const MEDIUM_WGHT: f32 = 510.;
/// The wght Chrome draws CSS weight 500 at.
const CSS_MEDIUM_WGHT: f32 = 500.;
/// How far CSS weight 500 lies from the Regular instance towards the Medium one on the wght axis.
pub(super) const MEDIUM_SHARE: f32 =
    (CSS_MEDIUM_WGHT - REGULAR_WGHT) / (MEDIUM_WGHT - REGULAR_WGHT);

/// A line's widths in points: `laid`, which sizes its box, and `drawn`, the width of the glyphs
/// GPUI paints. Only a line drawn with the Medium instance has them differ.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Widths {
    pub laid: f32,
    pub drawn: f32,
}

impl Widths {
    /// The widths of a line GPUI shapes `drawn` wide, and `regular` wide at Regular when it draws
    /// the line with the Medium instance: an advance moves linearly along the wght axis between
    /// the two instances, so Chrome's lies [`MEDIUM_SHARE`] of the way from one to the other.
    pub(super) fn new(drawn: f32, regular: Option<f32>) -> Self {
        let laid = regular.map_or(drawn, |regular| regular + MEDIUM_SHARE * (drawn - regular));
        Self { laid, drawn }
    }

    /// How many times as wide as its box the text is, so a cut keeps every glyph GPUI draws.
    pub(super) fn overhang(self) -> f32 {
        if self.laid > 0. {
            (self.drawn / self.laid).max(1.)
        } else {
            1.
        }
    }
}

/// Whether GPUI draws `font` with the system font's Medium instance: whether `font` resolves to
/// the face [`MEDIUM`] resolves to, which differs from Regular's. A weight is a request that
/// font-kit matches to a face, so this holds for every weight that picks Medium, not one value.
pub(super) fn drawn_medium(font: &Font, fonts: &TextSystem) -> bool {
    let face = |weight| {
        fonts.resolve_font(&Font {
            family: system_font().into(),
            weight,
            ..font.clone()
        })
    };
    let medium = face(MEDIUM);
    fonts.resolve_font(font) == medium && medium != face(FontWeight::NORMAL)
}

/// `text` shaped in `run` at `size`, and the widths it is laid out at.
pub(super) fn measure(
    text: &SharedString,
    run: &TextRun,
    size: Pixels,
    lines: &WindowTextSystem,
    fonts: &TextSystem,
) -> (ShapedLine, Widths) {
    let shape =
        |run: &TextRun| lines.shape_line(text.clone(), size, std::slice::from_ref(run), None);
    let line = shape(run);
    let regular = drawn_medium(&run.font, fonts).then(|| {
        let regular = TextRun {
            font: Font {
                weight: FontWeight::NORMAL,
                ..run.font.clone()
            },
            ..run.clone()
        };
        f32::from(shape(&regular).width)
    });
    let widths = Widths::new(f32::from(line.width), regular);
    (line, widths)
}

/// The width a line of `text` takes in the system font at `size` px, `weight`, and `features`, as
/// [`super::ExactText`] lays it out, for a box sized ahead of layout.
pub(crate) fn width(
    text: &SharedString,
    size: f32,
    weight: FontWeight,
    features: FontFeatures,
    cx: &App,
) -> f32 {
    let mut font = font(system_font());
    font.weight = weight;
    font.features = features;
    let run = TextRun {
        len: text.len(),
        font,
        color: hsla(0., 0., 0., 1.),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let lines = WindowTextSystem::new(cx.text_system().clone());
    measure(text, &run, px(size), &lines, cx.text_system())
        .1
        .laid
}
