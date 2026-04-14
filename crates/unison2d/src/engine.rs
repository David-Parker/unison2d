//! Engine — thin platform bridge for input and rendering.
//!
//! The engine does NOT own a world. Games create and manage their own
//! `World` instances (typically through `Level` structs). Engine provides:
//! - Access to raw input state
//! - Access to the renderer
//! - Fixed timestep delta
//! - Asset loading

use unison_input::InputState;
use unison_assets::AssetStore;
use unison_render::{AntiAliasing, Renderer, TextureId, RenderTargetId};

/// The engine struct. Manages input and renderer access.
///
/// Games receive `&mut Engine` in their `init()`, `update()`, and `render()`
/// callbacks. Use it for raw input and renderer access. For physics, objects,
/// and cameras, use your `World` directly.
pub struct Engine {
    #[doc(hidden)]
    pub input: InputState,

    // Renderer is set by the platform crate at startup.
    #[doc(hidden)]
    pub renderer: Option<Box<dyn Renderer<Error = String>>>,

    // Fixed timestep delta (set by platform's game loop).
    #[doc(hidden)]
    pub fixed_dt: f32,

    // Asset store for embedded assets.
    assets: AssetStore,
}

impl Engine {
    /// Create a new engine with default settings.
    pub fn new() -> Self {
        Self {
            input: InputState::new(),
            renderer: None,
            fixed_dt: 1.0 / 60.0,
            assets: AssetStore::new(),
        }
    }

    // ── Raw access ──

    /// Direct access to raw input state.
    pub fn input_state(&self) -> &InputState {
        &self.input
    }

    /// Get a mutable reference to the renderer.
    pub fn renderer_mut(&mut self) -> Option<&mut dyn Renderer<Error = String>> {
        match self.renderer.as_mut() {
            Some(r) => Some(&mut **r),
            None => None,
        }
    }

    /// Get the current fixed timestep delta (typically 1/60).
    pub fn dt(&self) -> f32 {
        self.fixed_dt
    }

    // ── Assets ──

    /// Access the asset store (read-only).
    pub fn assets(&self) -> &AssetStore {
        &self.assets
    }

    /// Access the asset store (mutable — for loading assets).
    pub fn assets_mut(&mut self) -> &mut AssetStore {
        &mut self.assets
    }

    /// Load a texture from the asset store by path.
    ///
    /// Decodes the image (PNG, JPEG, GIF, BMP, WebP) and uploads it to the GPU
    /// in one step. Returns the `TextureId` for use in sprites and soft bodies.
    ///
    /// ```ignore
    /// let texture = engine.load_texture("textures/donut-pink.png")?;
    /// ```
    pub fn load_texture(&mut self, asset_path: &str) -> Result<TextureId, String> {
        let bytes = self.assets.get(asset_path)
            .ok_or_else(|| format!("Asset not found: '{}'", asset_path))?;
        let desc = unison_render::decode_image(bytes)?;
        let renderer = self.renderer.as_mut()
            .ok_or("No renderer available")?;
        renderer.create_texture(&desc)
    }

    // ── Render targets ──

    /// Create an offscreen render target.
    ///
    /// Returns `(target_id, texture_id)`. The texture can be used to draw
    /// the target's contents on screen.
    pub fn create_render_target(&mut self, width: u32, height: u32) -> Result<(RenderTargetId, TextureId), String> {
        match self.renderer.as_mut() {
            Some(r) => r.create_render_target(width, height),
            None => Err("No renderer".into()),
        }
    }

    /// Destroy an offscreen render target (the associated texture is kept).
    pub fn destroy_render_target(&mut self, target: RenderTargetId) {
        if let Some(r) = self.renderer.as_mut() {
            r.destroy_render_target(target);
        }
    }

    // ── Anti-aliasing ──

    /// Set the anti-aliasing mode for rendering.
    ///
    /// Controls MSAA sample count for offscreen render targets. Higher
    /// values produce smoother edges at the cost of GPU memory and fill rate.
    ///
    /// ```ignore
    /// engine.set_anti_aliasing(AntiAliasing::MSAAx4);
    /// ```
    pub fn set_anti_aliasing(&mut self, mode: AntiAliasing) {
        if let Some(r) = self.renderer.as_mut() {
            r.set_anti_aliasing(mode);
        }
    }

    /// Get the current anti-aliasing mode.
    pub fn anti_aliasing(&self) -> AntiAliasing {
        match self.renderer.as_ref() {
            Some(r) => r.anti_aliasing(),
            None => AntiAliasing::None,
        }
    }

    // ── Internal: called by platform game loop ──

    /// Update input state for the next tick.
    /// Called by the platform's game loop before `Game::update()`.
    #[doc(hidden)]
    pub fn pre_update(&mut self) {
        // No-op: action mapping removed. Raw InputState is updated by the
        // platform via swap_into before each tick.
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}
