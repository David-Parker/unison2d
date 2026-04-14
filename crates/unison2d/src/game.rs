//! Game trait — internal lifecycle interface driven by platform crates.
//!
//! Game code is authored in Lua. `unison_scripting::ScriptedGame` is the only
//! shipping implementation; platform crates (`unison-web`, `unison-ios`,
//! `unison-android`) drive it from their frame loops.

use crate::engine::Engine;

/// Lifecycle contract that platform `run()` functions call into.
///
/// Not a game-authoring entry point: `ScriptedGame` hosts a Lua VM and bridges
/// these callbacks to the `unison.*` Lua surface.
pub trait Game: 'static {
    fn init(&mut self, engine: &mut Engine);
    fn update(&mut self, engine: &mut Engine);
    fn render(&mut self, engine: &mut Engine);
}
