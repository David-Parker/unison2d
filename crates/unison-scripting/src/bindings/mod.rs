//! Lua bindings for the Unison 2D engine.
//!
//! Each submodule exposes one domain of the engine to Lua scripts.
//! All bindings are registered during [`ScriptedGame::init`] via [`register_all`].

pub mod world;
pub mod objects;
pub mod input;
pub mod camera;
pub mod engine;
pub mod lighting;
pub mod events;
pub mod scene;
pub mod render_layers;
pub mod render_targets;
pub mod ui;
pub mod math;

use mlua::prelude::*;

/// Register all bindings into the Lua VM.
pub fn register_all(lua: &Lua) -> LuaResult<()> {
    // Core globals (World, input, engine)
    world::register(lua)?;
    input::register(lua)?;
    engine::register(lua)?;

    // Phase 3 globals
    events::register(lua)?;
    math::register(lua)?;

    // Extensions to existing globals (engine.set_scene, engine.create_ui, etc.)
    scene::register(lua)?;
    render_targets::register(lua)?;
    ui::register(lua)?;

    Ok(())
}
