/// Unison 2D Game Engine
///
/// A 2D game engine with clean subsystem architecture.
///
/// **Quick start:** Create a [`World`], implement the [`Game`] trait, call your platform's `run()`.
///
/// ```ignore
/// use unison2d::*;
/// use unison2d::math::{Color, Vec2};
/// use unison2d::input::KeyCode;
///
/// struct MyGame { world: World, player: ObjectId }
///
/// impl Game for MyGame {
///     type Action = MyAction;
///     fn init(&mut self, engine: &mut Engine<MyAction>) {
///         engine.bind_key(KeyCode::Space, MyAction::Jump);
///         self.player = self.world.objects.spawn_soft_body(/* ... */);
///     }
///     fn update(&mut self, engine: &mut Engine<MyAction>) {
///         self.world.step(engine.dt());
///     }
///     fn render(&mut self, engine: &mut Engine<MyAction>) {
///         if let Some(r) = engine.renderer_mut() { self.world.auto_render(r); }
///     }
/// }
/// ```
///
/// ## Architecture
/// - [`World`] — owns [`ObjectSystem`], [`CameraSystem`], and [`LightingSystem`](lighting::LightingSystem)
/// - [`Engine`] — thin shell for input/actions and renderer access
/// - [`Game`] — lifecycle trait: init, update, render
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
mod object_system;
mod camera_system;
mod world;
mod level;
mod game;

pub use engine::Engine;
pub use object::{ObjectId, SoftBodyDesc, RigidBodyDesc};
pub use object_system::ObjectSystem;
pub use camera_system::CameraSystem;
pub use world::World;
pub use level::Level;
pub use game::Game;

// Subsystem re-exports
pub use unison_math as math;
pub use unison_physics as physics;
pub use unison_render as render;
pub use unison_lighting as lighting;
pub use unison_profiler as profiler;
pub use unison_input as input;
