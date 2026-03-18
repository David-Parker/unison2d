/// Unison 2D Game Engine
///
/// A batteries-included 2D game engine with a simple, AI-agent-friendly API.
///
/// **Quick start:** Implement the [`Game`] trait, then call your platform's `run()` function.
///
/// ```ignore
/// use unison2d::*;
///
/// struct MyGame { player: ObjectId }
///
/// impl Game for MyGame {
///     type Action = MyAction;
///     fn init(&mut self, engine: &mut Engine<MyAction>) { /* spawn objects, bind input */ }
///     fn update(&mut self, engine: &mut Engine<MyAction>) { /* game logic */ }
/// }
/// ```
///
/// ## Subsystem crates (re-exported)
/// - [`math`] — Vec2, Color, Rect
/// - [`physics`] — XPBD soft body & rigid body simulation
/// - [`render`] — Platform-agnostic rendering abstractions
/// - [`lighting`] — 2D dynamic lighting with soft shadows
/// - [`profiler`] — Lightweight function-level profiling
/// - [`input`] — Two-layer input system (raw state + action mapping)

// Engine layer
mod engine;
mod object;
mod game;

pub use engine::Engine;
pub use object::{ObjectId, SoftBodyDesc, RigidBodyDesc};
pub use game::Game;

// Subsystem re-exports
pub use unison_math as math;
pub use unison_physics as physics;
pub use unison_render as render;
pub use unison_lighting as lighting;
pub use unison_profiler as profiler;
pub use unison_input as input;
