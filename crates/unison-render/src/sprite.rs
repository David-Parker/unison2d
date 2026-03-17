//! Sprite and sprite sheet types

use crate::texture::TextureId;
use crate::color::Color;

/// A sprite is a textured quad
#[derive(Debug, Clone)]
pub struct Sprite {
    /// Texture to render
    pub texture: TextureId,
    /// UV coordinates (min_u, min_v, max_u, max_v)
    pub uv: [f32; 4],
    /// Tint color
    pub color: Color,
    /// Pivot point (0,0 = bottom-left, 0.5,0.5 = center, 1,1 = top-right)
    pub pivot: [f32; 2],
}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            texture: TextureId::NONE,
            uv: [0.0, 0.0, 1.0, 1.0],
            color: Color::WHITE,
            pivot: [0.5, 0.5],
        }
    }
}

impl Sprite {
    /// Create a sprite from a texture (uses full texture)
    pub fn from_texture(texture: TextureId) -> Self {
        Self {
            texture,
            ..Default::default()
        }
    }

    /// Set UV coordinates
    pub fn with_uv(mut self, min_u: f32, min_v: f32, max_u: f32, max_v: f32) -> Self {
        self.uv = [min_u, min_v, max_u, max_v];
        self
    }

    /// Set tint color
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set pivot point
    pub fn with_pivot(mut self, x: f32, y: f32) -> Self {
        self.pivot = [x, y];
        self
    }
}

/// A sprite sheet is a texture divided into frames
#[derive(Debug, Clone)]
pub struct SpriteSheet {
    /// The texture containing all frames
    pub texture: TextureId,
    /// Width of each frame in pixels
    pub frame_width: u32,
    /// Height of each frame in pixels
    pub frame_height: u32,
    /// Number of columns in the sheet
    pub columns: u32,
    /// Number of rows in the sheet
    pub rows: u32,
    /// Total number of frames (may be less than columns * rows)
    pub frame_count: u32,
}

impl SpriteSheet {
    /// Create a new sprite sheet
    pub fn new(
        texture: TextureId,
        texture_width: u32,
        texture_height: u32,
        frame_width: u32,
        frame_height: u32,
    ) -> Self {
        let columns = texture_width / frame_width;
        let rows = texture_height / frame_height;
        Self {
            texture,
            frame_width,
            frame_height,
            columns,
            rows,
            frame_count: columns * rows,
        }
    }

    /// Get UV coordinates for a frame by index
    pub fn frame_uv(&self, index: u32) -> [f32; 4] {
        let index = index % self.frame_count;
        let col = index % self.columns;
        let row = index / self.columns;

        let u_size = 1.0 / self.columns as f32;
        let v_size = 1.0 / self.rows as f32;

        let min_u = col as f32 * u_size;
        let max_u = min_u + u_size;
        let min_v = row as f32 * v_size;
        let max_v = min_v + v_size;

        [min_u, min_v, max_u, max_v]
    }

    /// Create a sprite for a specific frame
    pub fn sprite(&self, frame: u32) -> Sprite {
        let uv = self.frame_uv(frame);
        Sprite {
            texture: self.texture,
            uv,
            color: Color::WHITE,
            pivot: [0.5, 0.5],
        }
    }
}
