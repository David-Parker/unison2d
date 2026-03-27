//! Glyph texture atlas — rasterizes glyphs on demand into a packed texture.

use std::collections::HashMap;

use ab_glyph::{Font, GlyphId, PxScale, ScaleFont};

use unison_render::{Renderer, TextureDescriptor, TextureFormat, TextureId};

use super::font::FontData;

/// A cached glyph entry in the atlas.
#[derive(Clone, Debug)]
pub struct GlyphEntry {
    /// UV coordinates in the atlas: [min_u, min_v, max_u, max_v].
    pub uv: [f32; 4],
    /// Offset from the pen position to the top-left of the glyph bitmap (pixels).
    pub offset_x: f32,
    pub offset_y: f32,
    /// Rasterized bitmap size in pixels.
    pub width: f32,
    pub height: f32,
}

/// Key for looking up a cached glyph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct GlyphKey {
    glyph_id: GlyphId,
    /// Font size encoded as integer (font_size * 10) for hashing.
    size_key: u32,
}

fn size_key(font_size: f32) -> u32 {
    (font_size * 10.0) as u32
}

/// Shelf-packed glyph texture atlas.
///
/// Glyphs are rasterized on demand and packed into rows (shelves).
/// When the atlas is full, it doubles in size and re-rasterizes all glyphs.
pub struct GlyphAtlas {
    /// Current atlas dimensions.
    width: u32,
    height: u32,
    /// GPU texture handle.
    texture: TextureId,
    /// RGBA pixel data (CPU copy for re-upload and resize).
    pixels: Vec<u8>,
    /// Cached glyph entries.
    entries: HashMap<GlyphKey, GlyphEntry>,
    /// Current shelf (row) packing state.
    shelf_x: u32,
    shelf_y: u32,
    shelf_height: u32,
    /// 1-pixel padding between glyphs.
    padding: u32,
    /// Device scale factor (e.g., 2.0 on Retina). Glyphs are rasterized at
    /// `font_size * scale_factor` but GlyphEntry stores logical (point) sizes.
    scale_factor: f32,
}

impl GlyphAtlas {
    /// Create a new atlas with the given initial dimensions.
    ///
    /// `scale_factor` is the device pixel ratio (e.g., 2.0 on Retina).
    /// Glyphs are rasterized at `font_size * scale_factor` for crisp rendering,
    /// but `GlyphEntry` dimensions are stored in logical points.
    pub fn new(
        width: u32,
        height: u32,
        scale_factor: f32,
        renderer: &mut dyn Renderer<Error = String>,
    ) -> Result<Self, String> {
        let pixels = vec![0u8; (width * height * 4) as usize];
        let texture = Self::create_texture(renderer, width, height, &pixels)?;
        Ok(Self {
            width,
            height,
            texture,
            pixels,
            entries: HashMap::new(),
            shelf_x: 0,
            shelf_y: 0,
            shelf_height: 0,
            padding: 1,
            scale_factor,
        })
    }

    /// Update the scale factor. Clears the glyph cache so glyphs are
    /// re-rasterized at the new resolution on next use.
    pub fn set_scale_factor(
        &mut self,
        scale_factor: f32,
        renderer: &mut dyn Renderer<Error = String>,
    ) -> Result<(), String> {
        if (self.scale_factor - scale_factor).abs() < 0.001 {
            return Ok(());
        }
        self.scale_factor = scale_factor;
        self.entries.clear();
        self.pixels.fill(0);
        self.shelf_x = 0;
        self.shelf_y = 0;
        self.shelf_height = 0;
        self.upload(renderer)
    }

    /// The atlas texture ID.
    pub fn texture(&self) -> TextureId {
        self.texture
    }

    /// Current atlas dimensions.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Number of cached glyphs.
    pub fn glyph_count(&self) -> usize {
        self.entries.len()
    }

    /// Look up a cached glyph, or rasterize and cache it.
    pub fn get_or_insert(
        &mut self,
        font: &FontData,
        glyph_id: GlyphId,
        font_size: f32,
        renderer: &mut dyn Renderer<Error = String>,
    ) -> Result<&GlyphEntry, String> {
        let key = GlyphKey {
            glyph_id,
            size_key: size_key(font_size),
        };

        if self.entries.contains_key(&key) {
            return Ok(&self.entries[&key]);
        }

        // Rasterize at physical size for crisp rendering on HiDPI
        let raster_size = font_size * self.scale_factor;
        let scaled = font.font().as_scaled(PxScale::from(raster_size));
        let glyph = glyph_id.with_scale_and_position(
            PxScale::from(raster_size),
            ab_glyph::point(0.0, scaled.ascent()),
        );

        let outlined = match font.font().outline_glyph(glyph) {
            Some(o) => o,
            None => {
                // Whitespace or missing glyph — insert a zero-size entry
                let entry = GlyphEntry {
                    uv: [0.0, 0.0, 0.0, 0.0],
                    offset_x: 0.0,
                    offset_y: 0.0,
                    width: 0.0,
                    height: 0.0,
                };
                self.entries.insert(key, entry);
                return Ok(&self.entries[&key]);
            }
        };

        let bounds = outlined.px_bounds();
        let gw = (bounds.max.x - bounds.min.x).ceil() as u32;
        let gh = (bounds.max.y - bounds.min.y).ceil() as u32;

        let inv_scale = 1.0 / self.scale_factor;

        if gw == 0 || gh == 0 {
            let entry = GlyphEntry {
                uv: [0.0, 0.0, 0.0, 0.0],
                offset_x: bounds.min.x * inv_scale,
                offset_y: bounds.min.y * inv_scale,
                width: 0.0,
                height: 0.0,
            };
            self.entries.insert(key, entry);
            return Ok(&self.entries[&key]);
        }

        // Ensure space in the atlas
        self.ensure_space(gw, gh, font, renderer)?;

        // Pack into current shelf
        let px = self.shelf_x;
        let py = self.shelf_y;

        // Rasterize into the pixel buffer
        outlined.draw(|x, y, coverage| {
            let ax = px + x as u32;
            let ay = py + y as u32;
            if ax < self.width && ay < self.height {
                let idx = ((ay * self.width + ax) * 4) as usize;
                let alpha = (coverage * 255.0).round() as u8;
                self.pixels[idx] = 255;     // R
                self.pixels[idx + 1] = 255; // G
                self.pixels[idx + 2] = 255; // B
                self.pixels[idx + 3] = alpha; // A = coverage
            }
        });

        // Compute UVs
        let u0 = px as f32 / self.width as f32;
        let v0 = py as f32 / self.height as f32;
        let u1 = (px + gw) as f32 / self.width as f32;
        let v1 = (py + gh) as f32 / self.height as f32;

        // Store logical (point) sizes — divide physical raster dims by scale
        let entry = GlyphEntry {
            uv: [u0, v0, u1, v1],
            offset_x: bounds.min.x * inv_scale,
            offset_y: bounds.min.y * inv_scale,
            width: gw as f32 * inv_scale,
            height: gh as f32 * inv_scale,
        };

        // Advance shelf cursor
        self.shelf_x += gw + self.padding;
        if gh + self.padding > self.shelf_height {
            self.shelf_height = gh + self.padding;
        }

        self.entries.insert(key, entry);

        // Re-upload entire texture
        self.upload(renderer)?;

        Ok(&self.entries[&key])
    }

    /// Ensure there is space for a glyph of the given size.
    /// If not, starts a new shelf or doubles the atlas.
    fn ensure_space(
        &mut self,
        gw: u32,
        gh: u32,
        font: &FontData,
        renderer: &mut dyn Renderer<Error = String>,
    ) -> Result<(), String> {
        // Try fitting on current shelf
        if self.shelf_x + gw <= self.width && self.shelf_y + gh.max(self.shelf_height) <= self.height {
            return Ok(());
        }

        // Try starting a new shelf
        let new_y = self.shelf_y + self.shelf_height;
        if gw <= self.width && new_y + gh <= self.height {
            self.shelf_x = 0;
            self.shelf_y = new_y;
            self.shelf_height = 0;
            return Ok(());
        }

        // Atlas is full — double the size and re-rasterize
        self.resize(font, renderer)
    }

    /// Double the atlas size and re-rasterize all cached glyphs.
    fn resize(
        &mut self,
        font: &FontData,
        renderer: &mut dyn Renderer<Error = String>,
    ) -> Result<(), String> {
        let new_width = self.width * 2;
        let new_height = self.height * 2;

        // Destroy old texture
        renderer.destroy_texture(self.texture);

        // Reset state
        self.width = new_width;
        self.height = new_height;
        self.pixels = vec![0u8; (new_width * new_height * 4) as usize];
        self.shelf_x = 0;
        self.shelf_y = 0;
        self.shelf_height = 0;

        // Collect keys to re-rasterize
        let keys: Vec<GlyphKey> = self.entries.keys().copied().collect();
        self.entries.clear();

        // Create new texture
        self.texture = Self::create_texture(renderer, new_width, new_height, &self.pixels)?;

        // Re-rasterize all glyphs
        for key in keys {
            let font_size = key.size_key as f32 / 10.0;
            // This recursive call will re-rasterize into the new, larger atlas.
            // We use a separate method to avoid borrow issues.
            self.rasterize_glyph(font, key.glyph_id, font_size)?;
        }

        // Re-upload
        self.upload(renderer)?;

        Ok(())
    }

    /// Rasterize a single glyph into the pixel buffer without uploading.
    fn rasterize_glyph(
        &mut self,
        font: &FontData,
        glyph_id: GlyphId,
        font_size: f32,
    ) -> Result<(), String> {
        let key = GlyphKey {
            glyph_id,
            size_key: size_key(font_size),
        };

        let raster_size = font_size * self.scale_factor;
        let scaled = font.font().as_scaled(PxScale::from(raster_size));
        let glyph = glyph_id.with_scale_and_position(
            PxScale::from(raster_size),
            ab_glyph::point(0.0, scaled.ascent()),
        );
        let inv_scale = 1.0 / self.scale_factor;

        let outlined = match font.font().outline_glyph(glyph) {
            Some(o) => o,
            None => {
                self.entries.insert(key, GlyphEntry {
                    uv: [0.0, 0.0, 0.0, 0.0],
                    offset_x: 0.0,
                    offset_y: 0.0,
                    width: 0.0,
                    height: 0.0,
                });
                return Ok(());
            }
        };

        let bounds = outlined.px_bounds();
        let gw = (bounds.max.x - bounds.min.x).ceil() as u32;
        let gh = (bounds.max.y - bounds.min.y).ceil() as u32;

        if gw == 0 || gh == 0 {
            self.entries.insert(key, GlyphEntry {
                uv: [0.0, 0.0, 0.0, 0.0],
                offset_x: bounds.min.x * inv_scale,
                offset_y: bounds.min.y * inv_scale,
                width: 0.0,
                height: 0.0,
            });
            return Ok(());
        }

        // Check space on current shelf
        if self.shelf_x + gw > self.width {
            self.shelf_y += self.shelf_height;
            self.shelf_x = 0;
            self.shelf_height = 0;
        }

        if self.shelf_y + gh > self.height {
            return Err("Atlas resize overflow — glyph doesn't fit after resize".into());
        }

        let px = self.shelf_x;
        let py = self.shelf_y;

        outlined.draw(|x, y, coverage| {
            let ax = px + x as u32;
            let ay = py + y as u32;
            if ax < self.width && ay < self.height {
                let idx = ((ay * self.width + ax) * 4) as usize;
                let alpha = (coverage * 255.0).round() as u8;
                self.pixels[idx] = 255;
                self.pixels[idx + 1] = 255;
                self.pixels[idx + 2] = 255;
                self.pixels[idx + 3] = alpha;
            }
        });

        let u0 = px as f32 / self.width as f32;
        let v0 = py as f32 / self.height as f32;
        let u1 = (px + gw) as f32 / self.width as f32;
        let v1 = (py + gh) as f32 / self.height as f32;

        self.shelf_x += gw + self.padding;
        if gh + self.padding > self.shelf_height {
            self.shelf_height = gh + self.padding;
        }

        self.entries.insert(key, GlyphEntry {
            uv: [u0, v0, u1, v1],
            offset_x: bounds.min.x * inv_scale,
            offset_y: bounds.min.y * inv_scale,
            width: gw as f32 * inv_scale,
            height: gh as f32 * inv_scale,
        });

        Ok(())
    }

    /// Upload the pixel buffer to the GPU texture.
    fn upload(&mut self, renderer: &mut dyn Renderer<Error = String>) -> Result<(), String> {
        // Destroy and recreate (simplest approach — no partial upload in the Renderer trait)
        renderer.destroy_texture(self.texture);
        self.texture = Self::create_texture(renderer, self.width, self.height, &self.pixels)?;
        Ok(())
    }

    /// Create an RGBA8 atlas texture.
    fn create_texture(
        renderer: &mut dyn Renderer<Error = String>,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> Result<TextureId, String> {
        let desc = TextureDescriptor::new(width, height, TextureFormat::Rgba8, pixels.to_vec());
        renderer.create_texture(&desc)
    }
}
