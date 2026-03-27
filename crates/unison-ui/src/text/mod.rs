//! Text rendering — font loading, glyph atlas, measurement, and render command generation.

pub mod atlas;
pub mod font;

use unison_math::{Color, Vec2};
use unison_render::{DrawSprite, RenderCommand, Renderer, TextureId};

use atlas::GlyphAtlas;
use font::FontData;

/// Initial atlas size (512×512 RGBA8 = 1MB).
const INITIAL_ATLAS_SIZE: u32 = 512;

/// Handles text measurement and rendering via a glyph atlas.
pub struct TextRenderer {
    font: FontData,
    atlas: GlyphAtlas,
}

impl TextRenderer {
    /// Create a new TextRenderer from raw font bytes (TTF/OTF).
    ///
    /// `scale_factor` is the device pixel ratio (e.g., 2.0 on Retina).
    /// Glyphs are rasterized at `font_size * scale_factor` for crisp HiDPI text.
    pub fn new(
        font_bytes: Vec<u8>,
        scale_factor: f32,
        renderer: &mut dyn Renderer<Error = String>,
    ) -> Result<Self, String> {
        let font = FontData::from_bytes(font_bytes)?;
        let atlas = GlyphAtlas::new(INITIAL_ATLAS_SIZE, INITIAL_ATLAS_SIZE, scale_factor, renderer)?;
        Ok(Self { font, atlas })
    }

    /// Update the device scale factor. Clears the glyph cache so glyphs
    /// are re-rasterized at the new resolution on next use.
    pub fn set_scale_factor(
        &mut self,
        scale_factor: f32,
        renderer: &mut dyn Renderer<Error = String>,
    ) -> Result<(), String> {
        self.atlas.set_scale_factor(scale_factor, renderer)
    }

    /// The atlas texture ID (for use in render commands).
    pub fn atlas_texture(&self) -> TextureId {
        self.atlas.texture()
    }

    /// Measure the pixel dimensions of a text string at the given font size.
    ///
    /// Returns (width, height) in pixels. Ensures all glyphs are cached.
    pub fn measure(
        &mut self,
        text: &str,
        font_size: f32,
        renderer: &mut dyn Renderer<Error = String>,
    ) -> Vec2 {
        if text.is_empty() {
            return Vec2::ZERO;
        }

        let mut width: f32 = 0.0;
        let mut prev_glyph = None;

        for c in text.chars() {
            let glyph_id = self.font.glyph_id(c);

            // Kerning
            if let Some(prev) = prev_glyph {
                width += self.font.kern(prev, glyph_id, font_size);
            }

            width += self.font.advance_width(glyph_id, font_size);

            // Ensure glyph is cached (may trigger atlas resize)
            let _ = self.atlas.get_or_insert(&self.font, glyph_id, font_size, renderer);

            prev_glyph = Some(glyph_id);
        }

        let height = self.font.line_height(font_size);
        Vec2::new(width, height)
    }

    /// Generate render commands for a text string.
    ///
    /// `position` is the top-left corner of the text in pixel coordinates.
    /// Returns a list of Sprite render commands (one per visible glyph).
    pub fn render_text(
        &mut self,
        text: &str,
        position: Vec2,
        font_size: f32,
        color: Color,
        renderer: &mut dyn Renderer<Error = String>,
    ) -> Vec<RenderCommand> {
        if text.is_empty() {
            return Vec::new();
        }

        let mut commands = Vec::new();
        let mut cursor_x = position.x;
        let mut prev_glyph = None;

        for c in text.chars() {
            let glyph_id = self.font.glyph_id(c);

            // Kerning
            if let Some(prev) = prev_glyph {
                cursor_x += self.font.kern(prev, glyph_id, font_size);
            }

            // Get or rasterize the glyph
            let entry = match self.atlas.get_or_insert(&self.font, glyph_id, font_size, renderer) {
                Ok(e) => e.clone(),
                Err(_) => {
                    cursor_x += self.font.advance_width(glyph_id, font_size);
                    prev_glyph = Some(glyph_id);
                    continue;
                }
            };

            // Only emit a sprite for non-zero-size glyphs
            if entry.width > 0.0 && entry.height > 0.0 {
                // Glyph position: cursor + glyph offset
                // offset_y is relative to the top of the line (from ab_glyph bounds)
                let gx = cursor_x + entry.offset_x;
                let gy = position.y + entry.offset_y;

                // DrawSprite position is the center of the quad
                let cx = gx + entry.width / 2.0;
                let cy = gy + entry.height / 2.0;

                commands.push(RenderCommand::Sprite(DrawSprite {
                    texture: self.atlas.texture(),
                    position: [cx, cy],
                    size: [entry.width, entry.height],
                    rotation: 0.0,
                    uv: entry.uv,
                    color,
                }));
            }

            cursor_x += self.font.advance_width(glyph_id, font_size);
            prev_glyph = Some(glyph_id);
        }

        commands
    }
}
