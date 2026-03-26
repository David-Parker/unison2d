//! Level — a self-contained game scene with its own world and behavior.
//!
//! Implement [`Level`] for each distinct scene in your game (main menu,
//! gameplay, cutscene, etc.). Each level owns a [`World`] and defines its
//! own update and render logic.
//!
//! The `Game` struct manages which levels are active and delegates to them:
//!
//! ```ignore
//! struct GameplayLevel {
//!     world: World,
//!     player: ObjectId,
//! }
//!
//! impl Level for GameplayLevel {
//!     fn world(&self) -> &World { &self.world }
//!     fn world_mut(&mut self) -> &mut World { &mut self.world }
//!
//!     fn update(&mut self, ctx: &mut LevelContext) {
//!         // Game logic here...
//!         self.world.step(ctx.dt);
//!     }
//!
//!     fn render(&mut self, ctx: &mut RenderContext) {
//!         self.world.auto_render(ctx.renderer);
//!     }
//! }
//! ```

use unison_input::InputState;
use unison_render::{Camera, Color, DrawSprite, RenderCommand, Renderer, RenderTargetId, TextureId};

use crate::World;

// ── Context types ──

/// Bundled context passed to [`Level::update`].
///
/// Contains input state, delta time, and optional shared state
/// that the `Game` passes down to all levels. The shared state `S`
/// defaults to `()` for levels that don't need it.
pub struct LevelContext<'a, S = ()> {
    /// Raw input state for this frame.
    pub input: &'a InputState,
    /// Fixed timestep delta (seconds).
    pub dt: f32,
    /// Shared state provided by the Game. Levels can read/write
    /// shared state (e.g., score, inventory, events) without owning it.
    pub shared: &'a mut S,
}

/// Bundled context passed to [`Level::render`].
///
/// Provides renderer access plus helpers for render target management
/// so levels can do multi-camera compositing.
pub struct RenderContext<'a> {
    /// The renderer for this frame.
    pub renderer: &'a mut dyn Renderer<Error = String>,
}

impl<'a> RenderContext<'a> {
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
    /// (1,1) is top-right. Intended for render-target textures (PiP cameras,
    /// minimaps) — UVs are adjusted for the platform's FBO orientation.
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
    ///
    /// Same as [`draw_overlay`](Self::draw_overlay) but draws a solid-color
    /// border around the texture. `border_width` is in the same 0..1
    /// normalized screen units.
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

// ── Level trait ──

/// A self-contained game scene, optionally generic over shared state `S`.
///
/// Each level owns a [`World`] and defines update/render behavior.
/// The `Game` decides which levels are active and calls into them.
///
/// Levels receive [`LevelContext<S>`] for update and [`RenderContext`] for render.
/// The shared state `S` defaults to `()` for levels that don't need it.
///
/// Lifecycle hooks (`on_enter`, `on_exit`, `on_pause`, `on_resume`) have
/// default no-op implementations.
pub trait Level<S = ()> {
    /// Immutable access to this level's world.
    fn world(&self) -> &World;

    /// Mutable access to this level's world.
    fn world_mut(&mut self) -> &mut World;

    /// Advance the level by one timestep.
    ///
    /// Receives bundled context with input, dt, and shared state.
    /// Typically calls `self.world_mut().step(ctx.dt)` plus game logic.
    fn update(&mut self, ctx: &mut LevelContext<S>);

    /// Render the level.
    ///
    /// Receives a [`RenderContext`] with renderer access and compositing helpers.
    fn render(&mut self, ctx: &mut RenderContext);

    // ── Lifecycle hooks (default no-op) ──

    /// Called when this level becomes the active level.
    fn on_enter(&mut self) {}

    /// Called when this level is being removed/replaced.
    fn on_exit(&mut self) {}

    /// Called when this level is paused (another level pushed on top).
    fn on_pause(&mut self) {}

    /// Called when this level is resumed (level above it was popped).
    fn on_resume(&mut self) {}
}
