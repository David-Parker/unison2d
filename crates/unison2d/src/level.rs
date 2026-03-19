//! Level — a self-contained game scene with its own world and behavior.
//!
//! Implement [`Level`] for each distinct scene in your game (main menu,
//! gameplay, cutscene, etc.). Each level owns a [`World`] and defines its
//! own update and render logic.
//!
//! The `Game` struct manages which levels are active and delegates to them:
//!
//! ```ignore
//! struct GameplayLevel {
//!     world: World,
//!     player: ObjectId,
//! }
//!
//! impl Level for GameplayLevel {
//!     fn world(&self) -> &World { &self.world }
//!     fn world_mut(&mut self) -> &mut World { &mut self.world }
//!
//!     fn update(&mut self, input: &InputState, dt: f32) {
//!         // Game logic here...
//!         self.world.step(dt);
//!     }
//!
//!     fn render(&mut self, renderer: &mut dyn Renderer<Error = String>) {
//!         self.world.auto_render(renderer);
//!     }
//! }
//! ```

use unison_input::InputState;
use unison_render::Renderer;

use crate::World;

/// A self-contained game scene.
///
/// Each level owns a [`World`] (physics, objects, cameras, lighting) and
/// defines update/render behavior. The `Game` decides which levels are
/// active and calls into them.
///
/// Levels receive `&InputState` (not an action map) so they are not generic
/// over the game's action type. This allows `Vec<Box<dyn Level>>`.
pub trait Level {
    /// Immutable access to this level's world.
    fn world(&self) -> &World;

    /// Mutable access to this level's world.
    fn world_mut(&mut self) -> &mut World;

    /// Advance the level by one timestep.
    ///
    /// Typically calls `self.world_mut().step(dt)` plus any game logic
    /// (spawning, AI, input handling via raw `InputState`).
    fn update(&mut self, input: &InputState, dt: f32);

    /// Render the level.
    ///
    /// Simple levels can call `self.world().auto_render(renderer)`.
    /// Multi-camera levels can use `self.world().render_to_targets(...)`.
    fn render(&mut self, renderer: &mut dyn Renderer<Error = String>);
}
