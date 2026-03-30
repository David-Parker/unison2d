//! Level — a self-contained game scene with its own world and behavior.
//!
//! Implement [`Level`] for each distinct scene in your game (main menu,
//! gameplay, cutscene, etc.). Each level owns a [`World`] and defines its
//! own update and render logic.
//!
//! The engine handles `world.step()` and `world.auto_render()` automatically
//! via the provided [`run_update`] and [`run_render`] methods — levels only
//! need to implement game logic.
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
//!         // Just game logic — world.step() is called automatically
//!         if ctx.input.key_pressed(KeyCode::Space) {
//!             self.world.objects.apply_impulse(self.player, Vec2::new(0.0, 10.0));
//!         }
//!     }
//!
//!     // render() is optional — world.auto_render() runs automatically.
//!     // Override only to queue overlays or custom draw commands.
//! }
//! ```
//!
//! For levels with fully custom render pipelines (e.g. multi-camera PiP),
//! override [`run_render`] instead.

use crate::ctx::Ctx;
use crate::World;

// ── Level trait ──

/// A self-contained game scene, optionally generic over shared state `S`.
///
/// Each level owns a [`World`] and defines update/render behavior.
/// The `Game` decides which levels are active and calls into them via
/// [`run_update`] and [`run_render`], which handle `world.step()` and
/// `world.auto_render()` automatically.
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

    /// Game logic for one timestep.
    ///
    /// Handle input, spawn/despawn objects, update state — but do **not**
    /// call `world.step()` here; that happens automatically in [`run_update`].
    fn update(&mut self, ctx: &mut Ctx<S>);

    /// Custom rendering before auto-render.
    ///
    /// Override to queue overlays, UI commands, or sky draws before the world
    /// is rendered. Default is a no-op — most levels don't need this.
    ///
    /// Do **not** call `world.auto_render()` here; that happens automatically
    /// in [`run_render`].
    fn render(&mut self, _ctx: &mut Ctx<S>) {}

    // ── Provided orchestration methods ──
    // Game code calls these instead of update/render directly.

    /// Run one update tick: level logic + `world.step()`.
    ///
    /// Called by the game loop. Levels should not override this unless they
    /// need to skip or customize the physics step.
    fn run_update(&mut self, ctx: &mut Ctx<S>) {
        self.update(ctx);
        self.world_mut().step(ctx.dt);
    }

    /// Run one render frame: custom rendering + `world.auto_render()`.
    ///
    /// Called by the game loop. Override for levels with fully custom render
    /// pipelines (e.g. multi-camera, render-to-texture).
    fn run_render(&mut self, ctx: &mut Ctx<S>) {
        self.render(ctx);
        self.world_mut().auto_render(ctx.renderer);
    }

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
