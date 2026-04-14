//! Ctx — unified context passed to game code during update and render.
//!
//! A single struct that has everything game code needs: input, renderer,
//! and shared state.
//!
//! ```ignore
//! fn my_update(ctx: &mut Ctx<SharedState>) {
//!     // Input
//!     let input = ctx.input;
//!     let dt = ctx.dt;
//!
//!     // Renderer
//!     let screen = ctx.renderer.screen_size();
//!
//!     // Shared state
//!     ctx.shared.score += 1;
//! }
//! ```

use unison_input::InputState;
use unison_render::{Camera, Color, DrawSprite, RenderCommand, Renderer, RenderTargetId, TextureId};

/// Unified context passed for both update and render.
///
/// Contains everything game code needs: input, renderer, delta time, and
/// shared state. Built by `Engine::ctx()` each frame.
pub struct Ctx<'a, S = ()> {
    /// Raw input state for this frame.
    pub input: &'a InputState,
    /// Fixed timestep delta (seconds).
    pub dt: f32,
    /// Shared state provided by the Game. Levels can read/write
    /// shared state (e.g., score, inventory) without owning it.
    pub shared: &'a mut S,
    /// The renderer for this frame.
    pub renderer: &'a mut dyn Renderer<Error = String>,
}

impl<'a, S> Ctx<'a, S> {
    /// Create an offscreen render target.
    ///
    /// Returns `(target_id, texture_id)`. Use the texture with sprite
    /// drawing to composite the result on screen.
    pub fn create_render_target(&mut self, width: u32, height: u32) -> Result<(RenderTargetId, TextureId), String> {
        self.renderer.create_render_target(width, height)
    }

    /// Bind a render target for subsequent draw calls.
    pub fn bind_render_target(&mut self, target: RenderTargetId) {
        self.renderer.bind_render_target(target);
    }

    /// Destroy an offscreen render target.
    pub fn destroy_render_target(&mut self, target: RenderTargetId) {
        self.renderer.destroy_render_target(target);
    }

    /// Get the screen/canvas size in pixels.
    pub fn screen_size(&self) -> (f32, f32) {
        self.renderer.screen_size()
    }

    /// Draw a texture as a screen-space overlay.
    ///
    /// Coordinates are in normalized screen space: (0,0) is bottom-left,
    /// (1,1) is top-right.
    pub fn draw_overlay(&mut self, texture: TextureId, position: [f32; 2], size: [f32; 2]) {
        let cx = position[0] + size[0] / 2.0;
        let cy = position[1] + size[1] / 2.0;

        let mut cam = Camera::new(1.0, 1.0);
        cam.set_position(0.5, 0.5);

        let uv = if self.renderer.fbo_origin_top_left() {
            [0.0, 0.0, 1.0, 1.0]
        } else {
            [0.0, 1.0, 1.0, 0.0]
        };

        self.renderer.bind_render_target(RenderTargetId::SCREEN);
        self.renderer.begin_frame(&cam);
        self.renderer.draw(RenderCommand::Sprite(DrawSprite {
            texture,
            position: [cx, cy],
            size,
            rotation: 0.0,
            uv,
            color: Color::WHITE,
        }));
        self.renderer.end_frame();
    }

    /// Draw a texture as a screen-space overlay with a colored border.
    pub fn draw_overlay_bordered(
        &mut self,
        texture: TextureId,
        position: [f32; 2],
        size: [f32; 2],
        border_width: f32,
        border_color: Color,
    ) {
        let cx = position[0] + size[0] / 2.0;
        let cy = position[1] + size[1] / 2.0;

        let mut cam = Camera::new(1.0, 1.0);
        cam.set_position(0.5, 0.5);

        let uv = if self.renderer.fbo_origin_top_left() {
            [0.0, 0.0, 1.0, 1.0]
        } else {
            [0.0, 1.0, 1.0, 0.0]
        };

        self.renderer.bind_render_target(RenderTargetId::SCREEN);
        self.renderer.begin_frame(&cam);

        self.renderer.draw(RenderCommand::Rect {
            position: [
                position[0] - border_width,
                position[1] - border_width,
            ],
            size: [
                size[0] + border_width * 2.0,
                size[1] + border_width * 2.0,
            ],
            color: border_color,
        });

        self.renderer.draw(RenderCommand::Sprite(DrawSprite {
            texture,
            position: [cx, cy],
            size,
            rotation: 0.0,
            uv,
            color: Color::WHITE,
        }));

        self.renderer.end_frame();
    }

}
