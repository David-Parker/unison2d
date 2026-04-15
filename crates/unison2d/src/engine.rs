//! Engine — thin platform bridge for input and rendering.
//!
//! The engine does NOT own a world. Games create and manage their own
//! `World` instances (typically through `Level` structs). Engine provides:
//! - Access to raw input state
//! - Access to the renderer
//! - Fixed timestep delta
//! - Asset loading

use std::collections::HashMap;

use unison_input::InputState;
use unison_assets::AssetStore;
use unison_audio::AudioSystem;
use unison_render::{AntiAliasing, FontId, Renderer, TextureId, RenderTargetId};

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

    // Font registry — maps FontId to an asset path. The asset store owns the
    // bytes; this map exists so scripts can refer to fonts by an opaque handle.
    font_paths: HashMap<FontId, String>,
    next_font_id: u32,

    /// Audio subsystem. Initialized in `Engine::new()` with a `KiraBackend`
    /// when the `backend-kira` feature is enabled; falls back to a silent
    /// stub backend if initialization fails or the feature is disabled.
    pub audio: AudioSystem,
}

impl Engine {
    /// Create a new engine with default settings.
    pub fn new() -> Self {
        Self {
            input: InputState::new(),
            renderer: None,
            fixed_dt: 1.0 / 60.0,
            assets: AssetStore::new(),
            font_paths: HashMap::new(),
            next_font_id: 1,
            audio: make_default_audio_system(),
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

    // ── Fonts ──

    /// Register a font asset and return an opaque [`FontId`].
    ///
    /// The engine verifies the asset exists in the asset store, allocates an
    /// id, and remembers the mapping. No bytes are copied — UI code fetches
    /// the bytes from the asset store when it actually needs to rasterize.
    pub fn load_font(&mut self, asset_path: &str) -> Result<FontId, String> {
        if self.assets.get(asset_path).is_none() {
            return Err(format!("Asset not found: '{}'", asset_path));
        }
        let id = FontId::from_raw(self.next_font_id);
        self.next_font_id += 1;
        self.font_paths.insert(id, asset_path.to_string());
        Ok(id)
    }

    /// Look up the asset path for a previously-registered font.
    pub fn font_path(&self, id: FontId) -> Option<&str> {
        self.font_paths.get(&id).map(|s| s.as_str())
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
        // Tick audio each frame (backend tweens, bookkeeping, etc.).
        self.audio.tick(self.dt());
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the default audio system.
///
/// Non-web targets: try `KiraBackend` (when enabled), falling back to a
/// silent `StubBackend` on error or when the feature is disabled so games
/// still run on headless / audio-less systems.
///
/// Web (`wasm32`): ALWAYS start on `StubBackend` and leave the system
/// unarmed. Constructing `KiraBackend` here would create the browser
/// `AudioContext` before any user gesture, which the autoplay policy then
/// suspends permanently. The web platform crate constructs the real
/// `KiraBackend` inside its first-gesture handler and swaps it in via
/// [`AudioSystem::swap_backend`].
fn make_default_audio_system() -> AudioSystem {
    #[cfg(target_arch = "wasm32")]
    {
        let mut sys = AudioSystem::with_backend(Box::new(unison_audio::StubBackend::new()));
        sys.unarm_for_web();
        return sys;
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "backend-kira"))]
    {
        use unison_audio::KiraBackend;
        match KiraBackend::new() {
            Ok(backend) => return AudioSystem::with_backend(Box::new(backend)),
            Err(e) => eprintln!(
                "[unison] audio: KiraBackend init failed ({e}); using silent stub"
            ),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    AudioSystem::with_backend(Box::new(unison_audio::StubBackend::new()))
}

// ── FFI helpers for platform glue (Swift / Kotlin / web event handlers) ──
//
// Each helper takes `*mut Engine`. The platform crate owns the Engine and is
// responsible for ensuring the pointer is valid and not aliased at the call
// site. Null pointers are tolerated (no-op) as a defensive measure.

/// Suspend audio output (e.g. app backgrounded).
///
/// # Safety
/// `engine` must be either null or a valid, exclusively-accessible pointer
/// to an `Engine` owned by the caller.
#[no_mangle]
pub unsafe extern "C" fn engine_audio_suspend(engine: *mut Engine) {
    if !engine.is_null() {
        (&mut *engine).audio.suspend();
    }
}

/// Resume audio output after suspension.
///
/// # Safety
/// See [`engine_audio_suspend`].
#[no_mangle]
pub unsafe extern "C" fn engine_audio_resume_system(engine: *mut Engine) {
    if !engine.is_null() {
        (&mut *engine).audio.resume_system();
    }
}

/// Arm the audio system after a user gesture (web autoplay policy).
///
/// # Safety
/// See [`engine_audio_suspend`].
#[no_mangle]
pub unsafe extern "C" fn engine_audio_arm(engine: *mut Engine) {
    if !engine.is_null() {
        (&mut *engine).audio.arm();
    }
}
