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
    /// Optional per-vertex colors (RGBA floats, 4 per vertex).
    /// When present, each vertex's color is multiplied with `color`.
    /// Length must be `positions.len() / 2 * 4`.
    pub vertex_colors: Option<Vec<f32>>,
}

/// A sprite drawn with an additional shadow mask texture.
///
/// Used by the lighting system to render lights with shadow casting.
/// The shader samples both the light gradient (texture) and the shadow
/// mask, with optional PCF filtering for soft shadow edges.
#[derive(Debug, Clone)]
pub struct DrawLitSprite {
    /// Light shape texture (e.g., radial gradient for point lights).
    pub texture: TextureId,
    /// Shadow mask texture (white = lit, black = shadowed).
    pub shadow_mask: TextureId,
    /// Position (x, y) in world space.
    pub position: [f32; 2],
    /// Size (width, height) in world units.
    pub size: [f32; 2],
    /// Rotation in radians.
    pub rotation: f32,
    /// UV coordinates (min_u, min_v, max_u, max_v).
    pub uv: [f32; 4],
    /// Light color (color * intensity).
    pub color: Color,
    /// Viewport dimensions for shadow UV calculation.
    pub screen_size: (f32, f32),
    /// PCF filter mode (0 = none, 5 = PCF5, 13 = PCF13).
    pub shadow_filter: u32,
    /// Shadow strength (0.0 = no shadow, 1.0 = full shadow). Default 1.0.
    pub shadow_strength: f32,
}

/// Render command enum
#[derive(Debug, Clone)]
pub enum RenderCommand {
    /// Draw a sprite
    Sprite(DrawSprite),
    /// Draw a mesh
    Mesh(DrawMesh),
    /// Draw a sprite with shadow mask sampling
    LitSprite(DrawLitSprite),
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

/// Opaque handle to an offscreen render target.
///
/// Created by [`Renderer::create_render_target`]. The special value
/// [`RenderTargetId::SCREEN`] refers to the default framebuffer (the screen).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RenderTargetId(pub u32);

impl RenderTargetId {
    /// The default framebuffer (screen).
    pub const SCREEN: Self = Self(0);
}

/// Blend mode for draw operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    /// Standard alpha blending: src * srcA + dst * (1 - srcA)
    Alpha,
    /// Additive blending: src * srcA + dst
    Additive,
    /// Multiply blending: src * dst
    Multiply,
}

/// Anti-aliasing mode for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntiAliasing {
    /// No anti-aliasing.
    None,
    /// 2x multisample anti-aliasing.
    MSAAx2,
    /// 4x multisample anti-aliasing.
    MSAAx4,
    /// 8x multisample anti-aliasing.
    MSAAx8,
}

impl AntiAliasing {
    /// Number of samples for this AA mode.
    pub fn samples(self) -> u32 {
        match self {
            AntiAliasing::None => 1,
            AntiAliasing::MSAAx2 => 2,
            AntiAliasing::MSAAx4 => 4,
            AntiAliasing::MSAAx8 => 8,
        }
    }
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

    /// Get the screen size in logical points (matches touch/UI coordinate space).
    fn screen_size(&self) -> (f32, f32);

    /// Get the screen size in physical pixels (for GPU resources like FBOs).
    ///
    /// On non-retina displays this equals `screen_size()`. On retina/HiDPI
    /// displays it is `screen_size() * scale_factor`.
    fn drawable_size(&self) -> (f32, f32) {
        self.screen_size()
    }

    /// Update the screen size in logical points (e.g., on window resize or device rotation).
    fn set_screen_size(&mut self, width: f32, height: f32);

    // ── Blend mode ──

    /// Set the blend mode for subsequent draw calls.
    ///
    /// Default is [`BlendMode::Alpha`]. Implementations should track state
    /// to avoid redundant GPU calls.
    fn set_blend_mode(&mut self, _mode: BlendMode) {}

    // ── Render targets ──

    /// Create an offscreen render target of the given size.
    ///
    /// Returns the target ID and a texture ID for the color attachment.
    /// The texture can be used in draw commands (e.g., for compositing).
    fn create_render_target(&mut self, _width: u32, _height: u32) -> Result<(RenderTargetId, TextureId), Self::Error> {
        unimplemented!("Renderer does not support render targets")
    }

    /// Bind a render target for subsequent draw calls.
    ///
    /// Pass [`RenderTargetId::SCREEN`] to bind the default framebuffer.
    fn bind_render_target(&mut self, _target: RenderTargetId) {
        unimplemented!("Renderer does not support render targets")
    }

    /// Destroy a render target (but not its associated texture).
    fn destroy_render_target(&mut self, _target: RenderTargetId) {}

    // ── Platform hints ──

    /// Whether FBO textures have origin at top-left (Metal) or bottom-left (OpenGL).
    ///
    /// OpenGL FBO textures have Y=0 at the bottom, so compositing requires a
    /// V-flip. Metal textures have Y=0 at the top, so no flip is needed.
    /// The engine uses this to select the correct UV orientation when
    /// compositing offscreen render targets (scene FBO, lightmap).
    fn fbo_origin_top_left(&self) -> bool { false }

    // ── Anti-aliasing ──

    /// Set the anti-aliasing mode for offscreen render targets.
    ///
    /// Existing render targets are not affected — only newly created ones
    /// will use the new setting. To apply a change, destroy and recreate
    /// active render targets (the lighting and world systems do this
    /// automatically on the next frame when the size changes).
    fn set_anti_aliasing(&mut self, _mode: AntiAliasing) {}

    /// Get the current anti-aliasing mode.
    fn anti_aliasing(&self) -> AntiAliasing { AntiAliasing::None }
}
