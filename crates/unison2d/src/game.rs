//! Game trait — the application lifecycle interface.
//!
//! Games implement this trait and pass themselves to a platform's `run()` function.
//! The platform handles the frame loop; the game handles logic and (optionally) rendering.

use std::hash::Hash;
use crate::engine::Engine;

/// The main game trait. Implement this to create a game.
///
/// ```ignore
/// struct MyGame { player: ObjectId }
///
/// impl Game for MyGame {
///     type Action = MyAction;
///
///     fn init(&mut self, engine: &mut Engine<MyAction>) {
///         self.player = engine.spawn_soft_body(/* ... */);
///         engine.bind_key(KeyCode::Space, MyAction::Jump);
///     }
///
///     fn update(&mut self, engine: &mut Engine<MyAction>) {
///         if engine.action_just_started(MyAction::Jump) {
///             engine.apply_impulse(self.player, Vec2::new(0.0, 10.0));
///         }
///     }
/// }
/// ```
pub trait Game {
    /// The game's action enum, used for input mapping.
    type Action: Copy + Eq + Hash + 'static;

    /// Called once after the engine is initialized.
    /// Set up your game world, spawn objects, bind input actions.
    fn init(&mut self, engine: &mut Engine<Self::Action>);

    /// Called once per fixed timestep tick.
    /// Read input via `engine.action_*()`, apply forces, update game state.
    /// Physics is stepped automatically after this returns.
    fn update(&mut self, engine: &mut Engine<Self::Action>);

    /// Called once per frame for custom rendering.
    /// The engine auto-renders all spawned objects before calling this.
    /// Override to draw additional things (UI, debug lines, particles).
    /// Default implementation does nothing.
    fn render(&mut self, _engine: &mut Engine<Self::Action>) {}
}
