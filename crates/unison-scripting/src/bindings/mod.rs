//! Lua bindings for the Unison 2D engine.
//!
//! Each submodule exposes one domain of the engine to Lua scripts.
//! All bindings are registered during [`ScriptedGame::init`] via [`register_all`].

pub mod world;
pub mod objects;
pub mod input;
pub mod camera;
pub mod engine;

use mlua::prelude::*;

/// Register all Phase 2 bindings into the Lua VM.
pub fn register_all(lua: &Lua) -> LuaResult<()> {
    world::register(lua)?;
    input::register(lua)?;
    engine::register(lua)?;
    Ok(())
}
