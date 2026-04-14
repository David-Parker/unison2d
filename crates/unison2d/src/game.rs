//! Game trait — the application lifecycle interface.
//!
//! Games implement this trait and pass themselves to a platform's `run()` function.
//! The platform handles the frame loop; the game handles logic and rendering.

use crate::engine::Engine;

/// The main game trait. Implement this to create a game.
///
/// Game code is written in Lua using `unison-scripting` (`ScriptedGame`), which
/// implements this trait. For advanced use cases you can implement `Game` directly
/// in Rust.
pub trait Game: 'static {
    /// Called once after the engine is initialized.
    /// Set up your world(s), spawn objects, load assets.
    fn init(&mut self, engine: &mut Engine);

    /// Called once per fixed timestep tick.
    /// Read input via `engine.input_state()`, apply forces to your world, step physics.
    fn update(&mut self, engine: &mut Engine);

    /// Called once per frame for rendering.
    /// The game is responsible for rendering its world(s) using `engine.renderer_mut()`.
    fn render(&mut self, engine: &mut Engine);
}
