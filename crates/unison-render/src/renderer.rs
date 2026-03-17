//! Renderer trait and render commands

use crate::color::Color;
use crate::texture::{TextureId, TextureDescriptor};
use crate::camera::Camera;

/// A sprite draw command
#[derive(Debug, Clone)]
pub struct DrawSprite {
    /// Texture to use
    pub texture: TextureId,
    /// Position (x, y)
    pub position: [f32; 2],
    /// Size (width, height)
    pub size: [f32; 2],
    /// Rotation in radians
    pub rotation: f32,
    /// UV coordinates (min_u, min_v, max_u, max_v)
    pub uv: [f32; 4],
    /// Tint color
    pub color: Color,
}

impl Default for DrawSprite {
    fn default() -> Self {
        Self {
            texture: TextureId::NONE,
            position: [0.0, 0.0],
            size: [1.0, 1.0],
            rotation: 0.0,
            uv: [0.0, 0.0, 1.0, 1.0],
            color: Color::WHITE,
        }
    }
}

/// A mesh draw command (for soft bodies, terrain, etc.)
#[derive(Debug, Clone)]
pub struct DrawMesh {
    /// Vertex positions (x, y pairs)
    pub positions: Vec<f32>,
    /// UV coordinates (u, v pairs), same length as positions
    pub uvs: Vec<f32>,
    /// Triangle indices
    pub indices: Vec<u32>,
    /// Texture (or NONE for solid color)
    pub texture: TextureId,
    /// Color (used as tint if textured, solid color if not)
    pub color: Color,
}

/// Render command enum
#[derive(Debug, Clone)]
pub enum RenderCommand {
    /// Draw a sprite
    Sprite(DrawSprite),
    /// Draw a mesh
    Mesh(DrawMesh),
    /// Draw a line
    Line {
        start: [f32; 2],
        end: [f32; 2],
        color: Color,
        width: f32,
    },
    /// Draw a filled rectangle
    Rect {
        position: [f32; 2],
        size: [f32; 2],
        color: Color,
    },
    /// Draw terrain as filled polygon
    Terrain {
        /// (x, y) points along terrain surface
        points: Vec<(f32, f32)>,
        /// Fill color
        fill_color: Color,
        /// Line color for surface
        line_color: Color,
    },
}

/// Renderer trait that platform crates implement
pub trait Renderer {
    /// Error type for renderer operations
    type Error;

    /// Initialize the renderer
    fn init(&mut self) -> Result<(), Self::Error>;

    /// Begin a new frame
    fn begin_frame(&mut self, camera: &Camera);

    /// Clear the screen with a color
    fn clear(&mut self, color: Color);

    /// Submit a render command
    fn draw(&mut self, command: RenderCommand);

    /// End the frame and present
    fn end_frame(&mut self);

    /// Create a texture from a descriptor
    fn create_texture(&mut self, desc: &TextureDescriptor) -> Result<TextureId, Self::Error>;

    /// Destroy a texture
    fn destroy_texture(&mut self, id: TextureId);

    /// Get the screen/canvas size
    fn screen_size(&self) -> (f32, f32);
}
