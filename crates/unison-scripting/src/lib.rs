//! Unison 2D Lua scripting — `ScriptedGame` implementing the [`Game`] trait.
//!
//! # Overview
//!
//! `ScriptedGame` embeds a Lua 5.4 VM and drives the game lifecycle from a Lua script.
//! The script must return a table with `init()`, `update(dt)`, and `render()` functions.
//!
//! # Script Lifecycle
//!
//! ```lua
//! local game = {}
//! local world, donut
//!
//! function game.init()
//!     world = World.new()
//!     world:set_gravity(-9.8)
//!     world:set_ground(-4.5)
//!     donut = world:spawn_soft_body({
//!         mesh = "ring", mesh_params = {1.0, 0.25, 24, 8},
//!         material = "rubber",
//!         position = {0, 3.5}, color = 0xFFFFFF,
//!         texture = engine.load_texture("textures/donut-pink.png"),
//!     })
//!     world.cameras:follow("main", donut, { smoothing = 0.08 })
//! end
//!
//! function game.update(dt)
//!     if input.is_key_pressed("ArrowRight") then
//!         world:apply_force(donut, 80, 0)
//!     end
//!     world:step(dt)
//! end
//!
//! function game.render()
//!     world:render()
//! end
//!
//! return game
//! ```

pub mod bridge;
pub mod bindings;
pub mod error_overlay;
pub mod hot_reload;

// Platform entry macro — stitches together web/iOS/Android entry points
// so a Lua game's lib.rs can be a single macro invocation.
#[macro_use]
mod entry_macro;

// libc stubs needed by embedded Lua on wasm32-unknown-unknown. Compiled only
// under wasm32; symbols are pulled in at link time by the Lua C static lib.
#[cfg(target_arch = "wasm32")]
pub mod wasm_libc;

// Re-exports used by the scripted_game_entry! macro so the expansion at the
// call site only has to name $crate::reexports::*. Games should not import
// from here directly — use the macro.
#[doc(hidden)]
pub mod reexports {
    #[cfg(feature = "web")]
    pub use wasm_bindgen;
    #[cfg(feature = "web")]
    pub use console_error_panic_hook;
    #[cfg(feature = "web")]
    pub use unison_web;
    #[cfg(feature = "ios")]
    pub use unison_ios;
    #[cfg(feature = "android")]
    pub use unison_android;
}

use mlua::prelude::*;
use mlua::{StdLib, LuaOptions};
use unison2d::{AntiAliasing, Engine, Game};
use unison2d::assets::EmbeddedAsset;
use unison_profiler::profile_scope;
use error_overlay::ErrorOverlay;

/// How the Lua script source is provided.
enum ScriptSource {
    /// Inline source code (e.g. from tests).
    Source(String),
    /// Asset path resolved via [`Engine::assets`] during [`Game::init`].
    AssetPath(String),
}

/// Top-level scripted game. Holds the Lua VM and the loaded script table.
pub struct ScriptedGame {
    /// The Lua VM. `None` before [`Game::init`] is called.
    lua: Option<Lua>,
    /// Where to get the script source.
    source: ScriptSource,
    /// Optional embedded asset table to load during init.
    embedded_assets: Option<&'static [EmbeddedAsset]>,
    /// On-screen error overlay (captures Lua errors for display in debug builds).
    overlay: ErrorOverlay,
}

impl ScriptedGame {
    /// Create a new `ScriptedGame` with inline Lua source code.
    pub fn new(script_src: impl Into<String>) -> Self {
        Self {
            lua: None,
            source: ScriptSource::Source(script_src.into()),
            embedded_assets: None,
            overlay: ErrorOverlay::new(),
        }
    }

    /// Create a new `ScriptedGame` that loads its script from embedded assets
    /// during [`Game::init`]. Pass the build-generated `assets::ASSETS` table
    /// so the engine can decompress them.
    pub fn from_asset(path: impl Into<String>, assets: &'static [EmbeddedAsset]) -> Self {
        Self {
            lua: None,
            source: ScriptSource::AssetPath(path.into()),
            embedded_assets: Some(assets),
            overlay: ErrorOverlay::new(),
        }
    }

    /// Returns `true` if a Lua error has been captured.
    pub fn has_error(&self) -> bool {
        self.overlay.has_error()
    }

    /// Returns the captured error message, if any.
    pub fn error_message(&self) -> Option<&str> {
        self.overlay.message()
    }

    /// Call a named function on the script table (the value returned by the top-level chunk).
    /// Returns `Ok(())` if the function doesn't exist or returns no error.
    fn call_lifecycle(&self, name: &str, args: impl IntoLuaMulti + Clone) -> LuaResult<()> {
        let lua = match &self.lua {
            Some(l) => l,
            None => return Ok(()),
        };

        let game_table: LuaTable = match lua.globals().get("__game") {
            Ok(t) => t,
            Err(_) => return Ok(()),
        };

        let func: Option<LuaFunction> = game_table.get(name).ok();
        if let Some(f) = func {
            f.call::<()>(args)?;
        }
        Ok(())
    }
}

impl Game for ScriptedGame {
    fn init(&mut self, engine: &mut Engine) {
        // Load embedded assets if provided.
        if let Some(assets) = self.embedded_assets {
            engine.assets_mut().load_embedded(assets);
        }

        // Resolve script source.
        let script_src = match &self.source {
            ScriptSource::Source(s) => s.clone(),
            ScriptSource::AssetPath(path) => {
                match engine.assets().get(path) {
                    Some(bytes) => match std::str::from_utf8(bytes) {
                        Ok(s) => s.to_string(),
                        Err(e) => {
                            let msg = format!("[unison-scripting] Script '{path}' is not valid UTF-8: {e}");
                            eprintln!("{msg}");
                            self.overlay.set(msg);
                            return;
                        }
                    },
                    None => {
                        let msg = format!("[unison-scripting] Script asset not found: '{path}'");
                        eprintln!("{msg}");
                        self.overlay.set(msg);
                        return;
                    }
                }
            }
        };

        // SAFETY: The debug library is needed for TSTL source-map traceback
        // (debug.getinfo / debug.traceback). The game VM is sandboxed — no
        // filesystem or C-module access — so exposing debug introspection is
        // harmless.
        let lua = unsafe {
            Lua::unsafe_new_with(StdLib::ALL_SAFE | StdLib::DEBUG, LuaOptions::default())
        };

        // Register all bindings (World, input, engine globals).
        if let Err(e) = bindings::register_all(&lua) {
            let msg = format!("[unison-scripting] Failed to register bindings: {e}");
            eprintln!("{msg}");
            self.overlay.set(msg);
            return;
        }

        // Pre-load all .lua script assets into package.preload so require() works.
        if let Err(e) = Self::setup_require(&lua, engine) {
            let msg = format!("[unison-scripting] Failed to setup require: {e}");
            eprintln!("{msg}");
            // Non-fatal: log but continue (require may not be used).
        }

        // Update screen size before script runs.
        if let Some(r) = engine.renderer_mut() {
            let (w, h) = r.screen_size();
            bindings::engine_state::set_screen_size(w, h);
        }

        // Execute the script. It must return a table.
        // Profiled so the one-time parse + top-level execution cost is visible.
        let load_result = {
            profile_scope!("scripting.load");
            lua.load(&script_src).eval::<LuaTable>()
        };
        let game_table: LuaTable = match load_result {
            Ok(t) => t,
            Err(e) => {
                let msg = format!("[unison-scripting] Script error: {e}");
                eprintln!("{msg}");
                self.overlay.set(msg);
                // Store the Lua VM so render() can draw the overlay.
                self.lua = Some(lua);
                return;
            }
        };

        // Store the returned game table as a global for lifecycle dispatch.
        if let Err(e) = lua.globals().set("__game", game_table) {
            let msg = format!("[unison-scripting] Failed to store game table: {e}");
            eprintln!("{msg}");
            self.overlay.set(msg);
        }

        self.lua = Some(lua);

        // Set engine pointer so Lua closures can call load_texture synchronously.
        // The guard clears the pointer automatically when it drops at end of scope.
        let _engine_guard = bindings::engine_state::set_engine_ptr(engine);

        // Call the script's init().
        {
            profile_scope!("scripting.init");
            if let Err(e) = self.call_lifecycle("init", ()) {
                let msg = format!("[unison-scripting] init() error: {e}");
                eprintln!("{msg}");
                self.overlay.set(msg);
            }
        }

        // Drop the guard explicitly before accessing engine again (AA setup below).
        drop(_engine_guard);

        // Apply anti-aliasing request if set.
        if let Some(aa) = bindings::engine_state::take_aa_request() {
            let mode = match aa.as_str() {
                "none" => AntiAliasing::None,
                "msaa2x" | "MSAAx2" => AntiAliasing::MSAAx2,
                "msaa4x" | "MSAAx4" => AntiAliasing::MSAAx4,
                "msaa8x" | "MSAAx8" => AntiAliasing::MSAAx8,
                _ => {
                    eprintln!("[unison-scripting] Unknown AA mode: '{aa}'");
                    AntiAliasing::None
                }
            };
            engine.set_anti_aliasing(mode);
        }
    }

    fn update(&mut self, engine: &mut Engine) {
        profile_scope!("scripting.update");
        let dt = engine.dt();

        // Refresh screen size.
        if let Some(r) = engine.renderer_mut() {
            let (w, h) = r.screen_size();
            bindings::engine_state::set_screen_size(w, h);
        }

        // Refresh input snapshot.
        bindings::input::refresh(engine.input_state());

        // Tick audio before user code runs (backend tweens, bookkeeping).
        engine.pre_update();

        // Make engine available to Lua closures (so load_texture works in
        // scene on_enter when scenes are switched mid-game). Kept set until
        // after event flushing so that event handlers (e.g. level_complete
        // → switch_scene → on_enter → load_texture) can still reach the engine.
        // The guard clears the pointer when it drops at end of this block.
        let _engine_guard = bindings::engine_state::set_engine_ptr(engine);

        // Dispatch update: scene system takes priority if active.
        // The inner scope isolates time spent inside the Lua VM (user script
        // code + binding callbacks) from the surrounding Rust overhead.
        {
            profile_scope!("lua.update");
            if bindings::scene::is_active() {
                if let Some(lua) = &self.lua {
                    if let Err(e) = bindings::scene::call_scene_update(lua, dt) {
                        let msg = format!("[unison-scripting] scene update() error: {e}");
                        eprintln!("{msg}");
                        self.overlay.set(msg);
                    }
                }
            } else {
                if let Err(e) = self.call_lifecycle("update", dt) {
                    let msg = format!("[unison-scripting] update() error: {e}");
                    eprintln!("{msg}");
                    self.overlay.set(msg);
                }
            }
        }

        // Flush collision events from world into Lua callbacks.
        {
            profile_scope!("lua.flush_events");
            if let Some(lua) = &self.lua {
                if let Some(world_rc) = bindings::engine_state::peek_render_world() {
                    let world_key = bindings::collisions::key_of(&world_rc);
                    let mut world = world_rc.borrow_mut();
                    bindings::collisions::flush(lua, world_key, &mut world);
                }
                // Flush string-keyed events.
                bindings::events::flush_string_events(lua);
            }
        }

        // Drop guard explicitly so engine is free before AA application below.
        drop(_engine_guard);

        // Apply any anti-aliasing request made during scene on_enter() callbacks.
        // Scenes switch during update(), so AA requests from on_enter() arrive
        // here rather than in init().
        if let Some(aa) = bindings::engine_state::take_aa_request() {
            let mode = match aa.as_str() {
                "none" => AntiAliasing::None,
                "msaa2x" | "MSAAx2" => AntiAliasing::MSAAx2,
                "msaa4x" | "MSAAx4" => AntiAliasing::MSAAx4,
                "msaa8x" | "MSAAx8" => AntiAliasing::MSAAx8,
                _ => {
                    eprintln!("[unison-scripting] Unknown AA mode: '{aa}'");
                    AntiAliasing::None
                }
            };
            engine.set_anti_aliasing(mode);
        }
    }

    fn render(&mut self, engine: &mut Engine) {
        profile_scope!("scripting.render");

        // Make engine available to Lua closures during render.
        // The guard clears the pointer when dropped.
        {
            let _engine_guard = bindings::engine_state::set_engine_ptr(engine);

            // Dispatch render: scene system takes priority if active.
            // Inner scope isolates time spent inside the Lua VM (script
            // render() + binding calls) from the surrounding Rust pipeline.
            profile_scope!("lua.render");
            if bindings::scene::is_active() {
                if let Some(lua) = &self.lua {
                    if let Err(e) = bindings::scene::call_scene_render(lua) {
                        let msg = format!("[unison-scripting] scene render() error: {e}");
                        eprintln!("{msg}");
                        self.overlay.set(msg);
                    }
                }
            } else {
                if let Err(e) = self.call_lifecycle("render", ()) {
                    let msg = format!("[unison-scripting] render() error: {e}");
                    eprintln!("{msg}");
                    self.overlay.set(msg);
                }
            }
            // _engine_guard drops here, clearing the pointer before we need
            // to take renderer_mut() below.
        }

        // If a UI frame was requested, render it into the world's overlays
        // before the main render pass. This needs the engine (renderer +
        // assets + input), so it has to happen before we take the renderer
        // borrow below.
        if let Some(world_rc) = bindings::engine_state::peek_render_world() {
            let mut world = world_rc.borrow_mut();
            bindings::ui::render_pending_ui(engine, &mut world);
            drop(world);
        }

        // Check if Lua called world:render(). We take the render world first
        // (before acquiring the renderer borrow) so we can push the active
        // camera position to the audio system's listener.
        let render_world = bindings::engine_state::take_render_world();
        if let Some(world_rc) = render_world.as_ref() {
            let listener_pos = {
                let world = world_rc.borrow();
                world.cameras
                    .active_world_position()
                    .unwrap_or(unison2d::core::Vec2::ZERO)
            };
            engine.audio.set_listener_position(listener_pos);
        }

        if let Some(r) = engine.renderer_mut() {
            // Check if Lua called world:render().
            if let Some(world_rc) = render_world {
                let mut world = world_rc.borrow_mut();
                world.snapshot_for_render();

                // Check for render_to_targets request.
                if let Some(targets) = bindings::render_targets::take_render_to_targets() {
                    use unison2d::render::RenderTargetId;
                    let camera_targets: Vec<(&str, RenderTargetId)> = targets.iter()
                        .map(|(name, raw)| (name.as_str(), RenderTargetId::from_raw(*raw)))
                        .collect();
                    world.render_to_targets(r, &camera_targets);
                } else {
                    world.render(r);
                }

                // Handle overlay requests (PiP, etc.)
                for overlay in bindings::render_targets::take_overlay_requests() {
                    use unison2d::render::{DrawSprite, RenderCommand, TextureId, RenderTargetId};
                    r.bind_render_target(RenderTargetId::SCREEN);
                    let cam = unison2d::render::Camera::new(1.0, 1.0);
                    r.begin_frame(&cam);
                    r.draw(RenderCommand::Sprite(DrawSprite {
                        texture: TextureId::from_raw(overlay.texture_id),
                        position: [overlay.x, overlay.y],
                        size: [overlay.w, overlay.h],
                        ..DrawSprite::default()
                    }));
                    r.end_frame();
                }
            } else {
                // Fallback: Phase 1 style — manual clear + draw_rect commands.
                let clear = bindings::engine_state::get_clear_color();
                let cam = unison2d::render::Camera::new(2.0, 2.0);
                r.begin_frame(&cam);
                r.clear(clear);
                bridge::flush_commands(r);
                r.end_frame();
            }
        }

        // Draw the error overlay on top of everything else (debug builds only).
        // This is a separate compositing pass so it always appears regardless of
        // which render path was taken above.
        #[cfg(debug_assertions)]
        self.overlay.render(engine);
    }
}

impl Drop for ScriptedGame {
    fn drop(&mut self) {
        // Drop the Lua VM first so that any Lua GC finalizers run before we
        // reset the thread-locals they may reference.
        self.lua = None;

        // Reset all thread-local state owned by the scripting system so that
        // a subsequent ScriptedGame constructed on the same thread starts clean.
        bindings::collisions::reset();
        bindings::events::reset();
        bindings::scene::reset();
        bindings::engine_state::reset();
        bindings::render_targets::reset();
        bindings::ui::reset();
    }
}

impl ScriptedGame {
    /// Hot-reload the script from new source (debug builds only).
    ///
    /// Two levels are attempted in order:
    ///
    /// - **Level 2 (default) — VM-preserving:** Re-execute the new source inside
    ///   the existing Lua VM and replace the `__game` table. World objects, physics
    ///   state, and all other Lua globals from `init()` are preserved. New `update`
    ///   and `render` definitions take effect on the very next frame.
    ///
    /// - **Level 1 (fallback) — Full restart:** If Level 2 fails (e.g. the script
    ///   structure changed fundamentally), destroy the VM, create a fresh one,
    ///   re-register all bindings, re-execute the script, and call `init()` again.
    ///   World state is lost.
    ///
    /// In release builds this is a no-op.
    #[cfg(debug_assertions)]
    pub fn reload(&mut self, new_source: &str) {
        // Always update the stored source so future reloads and crash recovery
        // use the new text.
        self.source = ScriptSource::Source(new_source.to_string());

        // --- Level 2: re-evaluate in the existing VM ---
        if let Some(lua) = &self.lua {
            // Clear require() cache so changed dependency modules are re-executed.
            let _ = lua.load("for k in pairs(package.loaded) do package.loaded[k] = nil end").exec();

            match lua.load(new_source).eval::<LuaTable>() {
                Ok(new_table) => {
                    // Replace the __game global with the freshly returned table.
                    if lua.globals().set("__game", new_table).is_ok() {
                        self.overlay.clear();
                        return; // Level 2 succeeded — done.
                    }
                    // If set failed, fall through to Level 1.
                }
                Err(_) => {
                    // Level 2 failed — fall through to Level 1.
                }
            }
        }

        // --- Level 1: full VM restart ---
        // Clear require() cache in existing VM (if any) before tearing it down,
        // so stale modules don't re-execute against a dying Lua state.
        if let Some(lua) = &self.lua {
            let _ = lua.load("for k in pairs(package.loaded) do package.loaded[k] = nil end").exec();
        }

        // Tear down the existing VM and create a fresh one.
        self.lua = None;

        // SAFETY: The debug library is needed for TSTL source-map traceback
        // (debug.getinfo / debug.traceback). The game VM is sandboxed — no
        // filesystem or C-module access — so exposing debug introspection is
        // harmless.
        let lua = unsafe {
            Lua::unsafe_new_with(StdLib::ALL_SAFE | StdLib::DEBUG, LuaOptions::default())
        };

        // Re-register all bindings.
        if let Err(e) = bindings::register_all(&lua) {
            let msg = format!("[unison-scripting] reload: failed to register bindings: {e}");
            eprintln!("{msg}");
            self.overlay.set(msg);
            return;
        }

        // Execute the script — it must return a table.
        let game_table: LuaTable = match lua.load(new_source).eval() {
            Ok(t) => t,
            Err(e) => {
                let msg = format!("[unison-scripting] reload: script error: {e}");
                eprintln!("{msg}");
                self.overlay.set(msg);
                // Keep the VM so render() can draw the overlay.
                self.lua = Some(lua);
                return;
            }
        };

        if let Err(e) = lua.globals().set("__game", game_table) {
            let msg = format!("[unison-scripting] reload: failed to store game table: {e}");
            eprintln!("{msg}");
            self.overlay.set(msg);
            self.lua = Some(lua);
            return;
        }

        self.lua = Some(lua);

        // Call init() on the fresh VM.
        if let Err(e) = self.call_lifecycle("init", ()) {
            let msg = format!("[unison-scripting] reload: init() error: {e}");
            eprintln!("{msg}");
            self.overlay.set(msg);
        } else {
            self.overlay.clear();
        }
    }

    /// No-op in release builds.
    #[cfg(not(debug_assertions))]
    pub fn reload(&mut self, _new_source: &str) {}

    /// Set up Lua `require()` to load scripts from embedded assets.
    ///
    /// Iterates all `.lua` asset paths and registers them in `package.preload`
    /// so that both `require("scenes/shared")` and `require("scenes.shared")`
    /// resolve to `scripts/scenes/shared.lua`. The dot-notation form is needed
    /// because TypeScriptToLua emits `require("scenes.menu")` rather than
    /// `require("scenes/menu")`.
    fn setup_require(lua: &Lua, engine: &Engine) -> LuaResult<()> {
        let preload: LuaTable = lua.globals()
            .get::<LuaTable>("package")?
            .get::<LuaTable>("preload")?;

        for path in engine.assets().paths() {
            if !path.starts_with("scripts/") || !path.ends_with(".lua") {
                continue;
            }

            // Convert "scripts/scenes/shared.lua" → "scenes/shared"
            let module_name = path
                .strip_prefix("scripts/").unwrap()
                .strip_suffix(".lua").unwrap()
                .to_string();

            let bytes = match engine.assets().get(path) {
                Some(b) => b,
                None => continue,
            };

            let source = match std::str::from_utf8(bytes) {
                Ok(s) => s.to_string(),
                Err(_) => continue,
            };

            let chunk_name = format!("@{path}");
            let func = lua.load(&source).set_name(&chunk_name).into_function()?;

            // Register with dot-notation key (e.g. "scenes.shared") for TSTL compat
            let dot_name = module_name.replace('/', ".");
            if dot_name != module_name {
                preload.set(dot_name, func.clone())?;
            }

            preload.set(module_name, func)?;
        }

        Ok(())
    }
}
