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
//!     fn update(&mut self, ctx: &mut Ctx) {
//!         // Input, renderer, events, shared state — all in ctx
//!         self.world.step(ctx.dt);
//!     }
//!
//!     fn render(&mut self, ctx: &mut Ctx) {
//!         self.world.auto_render(ctx.renderer);
//!     }
//! }
//! ```

use crate::ctx::Ctx;
use crate::World;

// ── Level trait ──

/// A self-contained game scene, optionally generic over shared state `S`.
///
/// Each level owns a [`World`] and defines update/render behavior.
/// The `Game` decides which levels are active and calls into them.
///
/// Levels receive [`Ctx<S>`] for both update and render. The context includes
/// input, renderer, events, assets, and shared state — everything a level needs.
///
/// Lifecycle hooks (`on_enter`, `on_exit`, `on_pause`, `on_resume`) have
/// default no-op implementations.
pub trait Level<S = ()> {
    /// Immutable access to this level's world.
    fn world(&self) -> &World;

    /// Mutable access to this level's world.
    fn world_mut(&mut self) -> &mut World;

    /// Advance the level by one timestep.
    ///
    /// Receives unified context with input, renderer, events, dt, and shared state.
    /// Typically calls `self.world_mut().step(ctx.dt)` plus game logic.
    fn update(&mut self, ctx: &mut Ctx<S>);

    /// Render the level.
    ///
    /// Receives the same [`Ctx`] with renderer access and compositing helpers.
    fn render(&mut self, ctx: &mut Ctx<S>);

    // ── Lifecycle hooks (default no-op) ──

    /// Called when this level becomes the active level.
    fn on_enter(&mut self) {}

    /// Called when this level is being removed/replaced.
    fn on_exit(&mut self) {}

    /// Called when this level is paused (another level pushed on top).
    fn on_pause(&mut self) {}

    /// Called when this level is resumed (level above it was popped).
    fn on_resume(&mut self) {}
}
