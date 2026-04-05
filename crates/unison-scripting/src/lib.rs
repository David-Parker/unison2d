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
//!     world:camera_follow("main", donut, 0.08)
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
//!     world:auto_render()
//! end
//!
//! return game
//! ```

pub mod bridge;
pub mod bindings;

use mlua::prelude::*;
use unison2d::{AntiAliasing, Engine, Game};
use unison2d::assets::EmbeddedAsset;

/// Unit action type — scripted games don't use Rust action mapping.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum NoAction {}

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
}

impl ScriptedGame {
    /// Create a new `ScriptedGame` with inline Lua source code.
    pub fn new(script_src: impl Into<String>) -> Self {
        Self {
            lua: None,
            source: ScriptSource::Source(script_src.into()),
            embedded_assets: None,
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
        }
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
    type Action = NoAction;

    fn init(&mut self, engine: &mut Engine<NoAction>) {
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
                            eprintln!("[unison-scripting] Script '{path}' is not valid UTF-8: {e}");
                            return;
                        }
                    },
                    None => {
                        eprintln!("[unison-scripting] Script asset not found: '{path}'");
                        return;
                    }
                }
            }
        };

        let lua = Lua::new();

        // Register all bindings (World, input, engine globals).
        if let Err(e) = bindings::register_all(&lua) {
            eprintln!("[unison-scripting] Failed to register bindings: {e}");
            return;
        }

        // Pre-load all .lua script assets into package.preload so require() works.
        if let Err(e) = Self::setup_require(&lua, engine) {
            eprintln!("[unison-scripting] Failed to setup require: {e}");
        }

        // Update screen size before script runs.
        if let Some(r) = engine.renderer_mut() {
            let (w, h) = r.screen_size();
            bindings::engine::set_screen_size(w, h);
        }

        // Execute the script. It must return a table.
        let game_table: LuaTable = match lua.load(&script_src).eval() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[unison-scripting] Script error: {e}");
                self.lua = Some(lua);
                return;
            }
        };

        // Store the returned game table as a global for lifecycle dispatch.
        if let Err(e) = lua.globals().set("__game", game_table) {
            eprintln!("[unison-scripting] Failed to store game table: {e}");
        }

        self.lua = Some(lua);

        // Set engine pointer so Lua closures can call load_texture synchronously.
        bindings::engine::set_engine_ptr(engine);

        // Call the script's init().
        if let Err(e) = self.call_lifecycle("init", ()) {
            eprintln!("[unison-scripting] init() error: {e}");
        }

        // Clear engine pointer — it's only valid during init.
        bindings::engine::clear_engine_ptr();

        // Apply anti-aliasing request if set.
        if let Some(aa) = bindings::engine::take_aa_request() {
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

    fn update(&mut self, engine: &mut Engine<NoAction>) {
        let dt = engine.dt();

        // Refresh screen size.
        if let Some(r) = engine.renderer_mut() {
            let (w, h) = r.screen_size();
            bindings::engine::set_screen_size(w, h);
        }

        // Refresh input snapshot.
        bindings::input::refresh(engine.input_state());

        // Make engine available to Lua closures (so load_texture works in
        // scene on_enter when scenes are switched mid-game).
        bindings::engine::set_engine_ptr(engine);

        // Dispatch update: scene system takes priority if active.
        if bindings::scene::is_active() {
            if let Some(lua) = &self.lua {
                if let Err(e) = bindings::scene::call_scene_update(lua, dt) {
                    eprintln!("[unison-scripting] scene update() error: {e}");
                }
            }
        } else {
            if let Err(e) = self.call_lifecycle("update", dt) {
                eprintln!("[unison-scripting] update() error: {e}");
            }
        }

        bindings::engine::clear_engine_ptr();

        // Flush collision events from world into Lua callbacks.
        if let Some(lua) = &self.lua {
            if let Some(world_rc) = bindings::engine::peek_auto_render_world() {
                let mut world = world_rc.borrow_mut();
                bindings::events::flush_collision_events(lua, &mut world);
            }
            // Flush string-keyed events.
            bindings::events::flush_string_events(lua);
        }
    }

    fn render(&mut self, engine: &mut Engine<NoAction>) {
        // Make engine available to Lua closures during render.
        bindings::engine::set_engine_ptr(engine);

        // Dispatch render: scene system takes priority if active.
        if bindings::scene::is_active() {
            if let Some(lua) = &self.lua {
                if let Err(e) = bindings::scene::call_scene_render(lua) {
                    eprintln!("[unison-scripting] scene render() error: {e}");
                }
            }
        } else {
            if let Err(e) = self.call_lifecycle("render", ()) {
                eprintln!("[unison-scripting] render() error: {e}");
            }
        }

        bindings::engine::clear_engine_ptr();

        if let Some(r) = engine.renderer_mut() {
            // Check if Lua called world:auto_render().
            if let Some(world_rc) = bindings::engine::take_auto_render_world() {
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
                    world.auto_render(r);
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
                let clear = bindings::engine::get_clear_color();
                let cam = unison2d::render::Camera::new(2.0, 2.0);
                r.begin_frame(&cam);
                r.clear(clear);
                bridge::flush_commands(r);
                r.end_frame();
            }
        }
    }
}

impl ScriptedGame {
    /// Set up Lua `require()` to load scripts from embedded assets.
    ///
    /// Iterates all `.lua` asset paths and registers them in `package.preload`
    /// so that `require("scenes/shared")` loads `scripts/scenes/shared.lua`.
    fn setup_require(lua: &Lua, engine: &Engine<NoAction>) -> LuaResult<()> {
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
            preload.set(module_name, func)?;
        }

        Ok(())
    }
}
