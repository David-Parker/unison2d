//! Font loading and glyph metrics via ab_glyph.

use ab_glyph::{Font, FontArc, GlyphId, PxScale, ScaleFont};

/// Loaded font data with metric queries.
pub struct FontData {
    font: FontArc,
}

impl FontData {
    /// Load a font from raw TTF/OTF bytes (takes ownership via Vec).
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, String> {
        let font = FontArc::try_from_vec(bytes)
            .map_err(|e| format!("Failed to load font: {}", e))?;
        Ok(Self { font })
    }

    /// Get the underlying ab_glyph font.
    pub fn font(&self) -> &FontArc {
        &self.font
    }

    /// Get a scaled font for the given pixel size.
    pub fn scaled(&self, font_size: f32) -> ab_glyph::PxScaleFont<&FontArc> {
        self.font.as_scaled(PxScale::from(font_size))
    }

    /// Look up the glyph ID for a character.
    pub fn glyph_id(&self, c: char) -> GlyphId {
        self.font.glyph_id(c)
    }

    /// Get the horizontal advance width for a glyph at the given size.
    pub fn advance_width(&self, glyph_id: GlyphId, font_size: f32) -> f32 {
        let scaled = self.scaled(font_size);
        scaled.h_advance(glyph_id)
    }

    /// Get the line height (ascent - descent) at the given size.
    pub fn line_height(&self, font_size: f32) -> f32 {
        let scaled = self.scaled(font_size);
        scaled.height()
    }

    /// Get the ascent (distance from baseline to top) at the given size.
    pub fn ascent(&self, font_size: f32) -> f32 {
        let scaled = self.scaled(font_size);
        scaled.ascent()
    }

    /// Get the descent (distance from baseline to bottom, negative) at the given size.
    pub fn descent(&self, font_size: f32) -> f32 {
        let scaled = self.scaled(font_size);
        scaled.descent()
    }

    /// Get kerning adjustment between two glyphs.
    pub fn kern(&self, a: GlyphId, b: GlyphId, font_size: f32) -> f32 {
        let scaled = self.scaled(font_size);
        scaled.kern(a, b)
    }
}
