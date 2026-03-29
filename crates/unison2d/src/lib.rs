/// Unison 2D Game Engine
///
/// A 2D game engine with clean subsystem architecture.
///
/// **Quick start:** Create a [`World`], implement the [`Game`] trait, call your platform's `run()`.
///
/// ```ignore
/// use unison2d::*;
/// use unison2d::core::{Color, Vec2};
/// use unison2d::input::KeyCode;
///
/// struct MyGame { world: World, player: ObjectId }
///
/// impl Game for MyGame {
///     type Action = MyAction;
///     fn init(&mut self, engine: &mut Engine<MyAction>) {
///         engine.bind_key(KeyCode::Space, MyAction::Jump);
///         self.player = self.world.spawn_soft_body(/* ... */);
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
/// - [`World`] — owns [`ObjectSystem`] and [`CameraSystem`]
/// - [`Engine`] — thin shell for input/actions and renderer access
/// - [`Game`] — lifecycle trait: init, update, render
///
/// ## Subsystem crates (re-exported)
/// - [`core`] — Vec2, Color, Rect, EventSink
/// - [`physics`] — XPBD soft body & rigid body simulation
/// - [`render`] — Platform-agnostic rendering abstractions
/// - [`profiler`] — Lightweight function-level profiling
/// - [`input`] — Two-layer input system (raw state + action mapping)
/// - [`assets`] — Build-time asset embedding and runtime asset store
/// - [`lighting`] — 2D lighting with lightmap compositing and shadows
/// - [`ui`] — Declarative React-like UI system (HUDs, menus, buttons)

// Engine layer
mod engine;
mod event_bus;
mod ctx;
mod object;
mod object_system;
mod camera_system;
mod world;
mod level;
mod prefab;
mod game;

pub use engine::Engine;
pub use event_bus::{EventBus, HandlerId};
pub use ctx::Ctx;
pub use unison_render::AntiAliasing;
pub use object::{ObjectId, SoftBodyDesc, RigidBodyDesc, SpriteDesc};
pub use object_system::{ObjectSystem, CollisionEvent};
pub use camera_system::CameraSystem;
pub use world::{World, Environment, RenderLayerId, RenderLayerConfig};
pub use level::Level;
pub use prefab::Prefab;
pub use game::Game;

// Subsystem re-exports
pub use unison_core as core;
pub use unison_physics as physics;
pub use unison_render as render;
pub use unison_profiler as profiler;
pub use unison_input as input;
pub use unison_assets as assets;
pub use unison_lighting as lighting;
pub use unison_ui as ui;
