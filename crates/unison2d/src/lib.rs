/// Unison 2D Game Engine
///
/// A 2D game engine with clean subsystem architecture.
///
/// **Quick start:** Create a [`World`], implement the [`Game`] trait, call your platform's `run()`.
///
/// ## Architecture
/// - [`World`] — owns [`ObjectSystem`] and [`CameraSystem`]
/// - [`Engine`] — thin shell for input and renderer access
/// - [`Game`] — lifecycle trait: init, update, render
///
/// Game code is written in **Lua** using `unison-scripting` (`ScriptedGame`).
///
/// ## Subsystem crates (re-exported)
/// - [`core`] — Vec2, Color, Rect, EventSink
/// - [`physics`] — XPBD soft body & rigid body simulation
/// - [`render`] — Platform-agnostic rendering abstractions
/// - [`profiler`] — Lightweight function-level profiling
/// - [`input`] — Raw input state (keyboard, mouse, touch)
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
mod game;

pub use engine::Engine;
pub use event_bus::{EventBus, HandlerId};
pub use ctx::Ctx;
pub use unison_render::AntiAliasing;
pub use object::{ObjectId, SoftBodyDesc, RigidBodyDesc, SpriteDesc};
pub use object_system::{ObjectSystem, CollisionEvent};
pub use camera_system::CameraSystem;
pub use world::{World, Environment, RenderLayerId, RenderLayerConfig};
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
