//! A stand-in for the system font's faces: GPUI's no-op text system, with the Regular, Medium,
//! and Semibold faces resolved as font-kit resolves SF Pro's, which it reads at weights 400, 530,
//! and 600, and with Medium's advances half again as wide as Regular's, so a test can tell which
//! width a line was laid out at. Any other family is a face of its own at Regular's widths.

use std::borrow::Cow;

use gpui_kit::{
    Bounds, DevicePixels, Font, FontId, FontMetrics, FontRun, GlyphId, LineLayout, NoopTextSystem,
    Pixels, PlatformTextSystem, RenderGlyphParams, Result, Size, TextRenderingMode,
};
use skillbinder_theme::system_font;

const REGULAR: FontId = FontId(1);
const MEDIUM_FACE: FontId = FontId(2);
const SEMIBOLD: FontId = FontId(3);
const OTHER: FontId = FontId(4);
/// How much wider than Regular the Medium face sets.
pub(super) const MEDIUM_SCALE: f32 = 1.5;
/// How much wider than Regular the Semibold face sets.
const SEMIBOLD_SCALE: f32 = 1.75;
/// A glyph's advance in the no-op text system, in ems.
const ADVANCE: f32 = 0.6;

pub(super) struct Faces(NoopTextSystem);

impl Faces {
    pub(super) fn new() -> Self {
        Self(NoopTextSystem::new())
    }
}

/// The width the no-op text system shapes `text` at `size` px in a Regular face.
pub(super) fn regular_width(text: &str, size: f32) -> f32 {
    // Test strings stay short, so the cast is exact.
    #[expect(clippy::cast_precision_loss)]
    let glyphs = text.chars().count() as f32;
    glyphs * ADVANCE * size
}

impl PlatformTextSystem for Faces {
    fn add_fonts(&self, fonts: Vec<Cow<'static, [u8]>>) -> Result<()> {
        self.0.add_fonts(fonts)
    }

    fn all_font_names(&self) -> Vec<String> {
        self.0.all_font_names()
    }

    fn font_id(&self, font: &Font) -> Result<FontId> {
        if font.family.as_ref() != system_font() {
            return Ok(OTHER);
        }
        Ok(match font.weight.0 {
            weight if weight <= 500. => REGULAR,
            weight if weight <= 530. => MEDIUM_FACE,
            _ => SEMIBOLD,
        })
    }

    fn font_metrics(&self, font_id: FontId) -> FontMetrics {
        self.0.font_metrics(font_id)
    }

    fn typographic_bounds(&self, font_id: FontId, glyph_id: GlyphId) -> Result<Bounds<f32>> {
        self.0.typographic_bounds(font_id, glyph_id)
    }

    fn advance(&self, font_id: FontId, glyph_id: GlyphId) -> Result<Size<f32>> {
        self.0.advance(font_id, glyph_id)
    }

    fn glyph_for_char(&self, font_id: FontId, ch: char) -> Option<GlyphId> {
        self.0.glyph_for_char(font_id, ch)
    }

    fn glyph_raster_bounds(&self, params: &RenderGlyphParams) -> Result<Bounds<DevicePixels>> {
        self.0.glyph_raster_bounds(params)
    }

    fn rasterize_glyph(
        &self,
        params: &RenderGlyphParams,
        raster_bounds: Bounds<DevicePixels>,
    ) -> Result<(Size<DevicePixels>, Vec<u8>)> {
        self.0.rasterize_glyph(params, raster_bounds)
    }

    fn layout_line(&self, text: &str, font_size: Pixels, runs: &[FontRun]) -> LineLayout {
        let mut layout = self.0.layout_line(text, font_size, runs);
        let scale = match runs.first().map(|run| run.font_id) {
            Some(MEDIUM_FACE) => MEDIUM_SCALE,
            Some(SEMIBOLD) => SEMIBOLD_SCALE,
            _ => 1.,
        };
        layout.width *= scale;
        for glyph in layout.runs.iter_mut().flat_map(|run| run.glyphs.iter_mut()) {
            glyph.position.x *= scale;
        }
        layout
    }

    fn recommended_rendering_mode(&self, font_id: FontId, font_size: Pixels) -> TextRenderingMode {
        self.0.recommended_rendering_mode(font_id, font_size)
    }
}
