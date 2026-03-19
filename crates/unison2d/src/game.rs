//! Game trait — the application lifecycle interface.
//!
//! Games implement this trait and pass themselves to a platform's `run()` function.
//! The platform handles the frame loop; the game handles logic and rendering.

use std::hash::Hash;
use crate::engine::Engine;

/// The main game trait. Implement this to create a game.
///
/// ```ignore
/// struct MyGame { world: World, player: ObjectId }
///
/// impl Game for MyGame {
///     type Action = MyAction;
///
///     fn init(&mut self, engine: &mut Engine<MyAction>) {
///         engine.bind_key(KeyCode::Space, MyAction::Jump);
///         self.player = self.world.objects.spawn_soft_body(/* ... */);
///     }
///
///     fn update(&mut self, engine: &mut Engine<MyAction>) {
///         if engine.action_just_started(MyAction::Jump) {
///             self.world.objects.apply_impulse(self.player, Vec2::new(0.0, 10.0));
///         }
///         self.world.step(engine.dt());
///     }
///
///     fn render(&mut self, engine: &mut Engine<MyAction>) {
///         if let Some(renderer) = engine.renderer_mut() {
///             self.world.auto_render(renderer);
///         }
///     }
/// }
/// ```
pub trait Game {
    /// The game's action enum, used for input mapping.
    type Action: Copy + Eq + Hash + 'static;

    /// Called once after the engine is initialized.
    /// Bind input actions on engine, set up your world(s), spawn objects.
    fn init(&mut self, engine: &mut Engine<Self::Action>);

    /// Called once per fixed timestep tick.
    /// Read input via `engine.action_*()`, apply forces to your world, step physics.
    fn update(&mut self, engine: &mut Engine<Self::Action>);

    /// Called once per frame for rendering.
    /// The game is responsible for rendering its world(s) using `engine.renderer_mut()`.
    fn render(&mut self, engine: &mut Engine<Self::Action>);
}
