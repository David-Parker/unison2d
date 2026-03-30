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
//!
//! function game.init()
//!     engine.set_background(0.1, 0.1, 0.12)
//! end
//!
//! function game.update(dt) end
//!
//! function game.render()
//!     engine.draw_rect(0, 0, 2, 2, 1, 0.2, 0.2)
//! end
//!
//! return game
//! ```
//!
//! The `engine` global is pre-registered before the script runs.
//! Missing lifecycle functions are silently ignored (no panic).
//! Script errors are logged and do not crash the process.

mod bridge;

use mlua::prelude::*;
use unison2d::{Engine, Game};
use unison2d::render::Camera;

/// Unit action type — scripted games don't use Rust action mapping.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum NoAction {}

/// Top-level scripted game. Holds the Lua VM and the loaded script table.
pub struct ScriptedGame {
    /// The Lua VM. `None` before [`Game::init`] is called.
    lua: Option<Lua>,
    /// The script source to execute (typically loaded from assets).
    script_src: String,
}

impl ScriptedGame {
    /// Create a new `ScriptedGame` with the given Lua source code.
    pub fn new(script_src: impl Into<String>) -> Self {
        Self {
            lua: None,
            script_src: script_src.into(),
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
        let lua = Lua::new();

        // Register engine globals before running the script.
        if let Err(e) = bridge::register_engine_globals(&lua) {
            eprintln!("[unison-scripting] Failed to register engine globals: {e}");
            return;
        }

        // Update screen size cache.
        if let Some(r) = engine.renderer_mut() {
            let (w, h) = r.screen_size();
            bridge::set_screen_size(w, h);
        }

        // Execute the script. It must return a table.
        let game_table: LuaTable = match lua.load(&self.script_src).eval() {
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

        // Call the script's init().
        if let Err(e) = self.call_lifecycle("init", ()) {
            eprintln!("[unison-scripting] init() error: {e}");
        }
    }

    fn update(&mut self, engine: &mut Engine<NoAction>) {
        let dt = engine.dt();

        // Refresh screen size each frame.
        if let Some(r) = engine.renderer_mut() {
            let (w, h) = r.screen_size();
            bridge::set_screen_size(w, h);
        }

        if let Err(e) = self.call_lifecycle("update", dt) {
            eprintln!("[unison-scripting] update() error: {e}");
        }
    }

    fn render(&mut self, engine: &mut Engine<NoAction>) {
        if let Some(r) = engine.renderer_mut() {
            let clear = bridge::get_clear_color();
            let cam = Camera::new(2.0, 2.0);
            r.begin_frame(&cam);
            r.clear(clear);

            // Lua render() call buffers RenderCommands via bridge globals.
            if let Err(e) = self.call_lifecycle("render", ()) {
                eprintln!("[unison-scripting] render() error: {e}");
            }

            // Submit buffered commands to the renderer.
            bridge::flush_commands(r);
            r.end_frame();
        }
    }
}
