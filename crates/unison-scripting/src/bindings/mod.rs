//! Lua bindings for the Unison 2D engine.
//!
//! All subsystems are registered under the `unison` global via [`register_all`].
//! The old flat globals (`engine`, `input`, `events`, `World`, `Color`, `Rng`,
//! `debug`) are gone — everything lives under `unison.*`.

// Internal state (no Lua registration)
pub mod engine_state;

// New top-level modules
pub mod assets;
pub mod renderer;
pub mod unison_root;

// Per-subsystem modules (each exposes `populate(lua, unison)`)
pub mod world;
pub mod objects;
pub mod input;
pub mod camera;
pub mod lighting;
pub mod events;
pub mod scene;
pub mod render_layers;
pub mod render_targets;
pub mod ui;
pub mod math;
pub mod debug;

use mlua::prelude::*;

/// Register all bindings into the Lua VM under `unison.*`.
pub fn register_all(lua: &Lua) -> LuaResult<()> {
    unison_root::register(lua)
}
