//! Engine — thin platform bridge for input, actions, and rendering.
//!
//! The engine does NOT own a world. Games create and manage their own
//! `World` instances (typically through `Level` structs). Engine provides:
//! - Input → action mapping
//! - Access to the renderer
//! - Fixed timestep delta

use std::hash::Hash;

use unison_input::{ActionMap, InputState, KeyCode, MouseButton};
use unison_assets::AssetStore;
use crate::ctx::Ctx;
use crate::event_bus::EventBus;
use crate::World;
use unison_render::{AntiAliasing, Renderer, TextureId, RenderTargetId};

/// The engine struct. Manages input, actions, and renderer access.
///
/// Games receive `&mut Engine<A>` in their `init()`, `update()`, and `render()`
/// callbacks. Use it for input bindings and action queries. For physics, objects,
/// and cameras, use your `World` directly.
///
/// ## Action generics
///
/// `A` is the game's action enum, used for type-safe input mapping.
/// Scripted games (`ScriptedGame`) use `NoAction` (from `unison-scripting`) because
/// input is handled in Lua rather than through Rust action enums. Rust-native games
/// can define their own action enum and bind keys/buttons to it.
///
/// As of Phase 5, the only active consumer is `ScriptedGame<NoAction>`. The generic
/// is retained because:
/// - The action API (`bind_key`, `action_active`, `action_axis`, etc.) provides real
///   value for future Rust-native game code
/// - The platform crates (`unison-web`, `unison-ios`, `unison-android`) are correctly
///   generic over `G::Action`, enabling non-scripted games without changes
/// - Removing it would be a breaking API change with no current technical benefit
///
/// // TODO: re-evaluate Action generics once there are more game consumers (scripted and native)
pub struct Engine<A: Copy + Eq + Hash> {
    #[doc(hidden)]
    pub input: InputState,
    #[doc(hidden)]
    pub actions: ActionMap<A>,

    // Renderer is set by the platform crate at startup.
    #[doc(hidden)]
    pub renderer: Option<Box<dyn Renderer<Error = String>>>,

    // Fixed timestep delta (set by platform's game loop).
    #[doc(hidden)]
    pub fixed_dt: f32,

    // Asset store for embedded assets.
    assets: AssetStore,

    // Event bus for inter-component messaging.
    events: EventBus<World>,
}

impl<A: Copy + Eq + Hash> Engine<A> {
    /// Create a new engine with default settings.
    pub fn new() -> Self {
        Self {
            input: InputState::new(),
            actions: ActionMap::new(),
            renderer: None,
            fixed_dt: 1.0 / 60.0,
            assets: AssetStore::new(),
            events: EventBus::new(),
        }
    }

    // ── Input / Actions ──

    /// Bind a keyboard key to an action.
    pub fn bind_key(&mut self, key: KeyCode, action: A) {
        self.actions.bind_key(key, action);
    }

    /// Bind a mouse button to an action.
    pub fn bind_mouse_button(&mut self, button: MouseButton, action: A) {
        self.actions.bind_mouse_button(button, action);
    }

    /// Is the action currently active? (any bound input is held)
    pub fn action_active(&self, action: A) -> bool {
        self.actions.is_action_active(action)
    }

    /// Did the action just start this frame?
    pub fn action_just_started(&self, action: A) -> bool {
        self.actions.is_action_just_started(action)
    }

    /// Did the action just end this frame?
    pub fn action_just_ended(&self, action: A) -> bool {
        self.actions.is_action_just_ended(action)
    }

    /// Get an axis value from two opposing actions (-1, 0, or +1).
    pub fn action_axis(&self, negative: A, positive: A) -> f32 {
        self.actions.axis_value(negative, positive)
    }

    // ── Raw access ──

    /// Direct access to raw input state.
    pub fn input_state(&self) -> &InputState {
        &self.input
    }

    /// Direct access to the action map for custom bindings.
    pub fn actions_mut(&mut self) -> &mut ActionMap<A> {
        &mut self.actions
    }

    /// Get a mutable reference to the renderer.
    pub fn renderer_mut(&mut self) -> Option<&mut dyn Renderer<Error = String>> {
        match self.renderer.as_mut() {
            Some(r) => Some(&mut **r),
            None => None,
        }
    }

    /// Get the current fixed timestep delta (typically 1/60).
    pub fn dt(&self) -> f32 {
        self.fixed_dt
    }

    // ── Assets ──

    /// Access the asset store (read-only).
    pub fn assets(&self) -> &AssetStore {
        &self.assets
    }

    /// Access the asset store (mutable — for loading assets).
    pub fn assets_mut(&mut self) -> &mut AssetStore {
        &mut self.assets
    }

    /// Load a texture from the asset store by path.
    ///
    /// Decodes the image (PNG, JPEG, GIF, BMP, WebP) and uploads it to the GPU
    /// in one step. Returns the `TextureId` for use in sprites and soft bodies.
    ///
    /// ```ignore
    /// let texture = engine.load_texture("textures/donut-pink.png")?;
    /// ```
    pub fn load_texture(&mut self, asset_path: &str) -> Result<TextureId, String> {
        let bytes = self.assets.get(asset_path)
            .ok_or_else(|| format!("Asset not found: '{}'", asset_path))?;
        let desc = unison_render::decode_image(bytes)?;
        let renderer = self.renderer.as_mut()
            .ok_or("No renderer available")?;
        renderer.create_texture(&desc)
    }

    /// Build a [`Ctx`] from this engine's input, renderer, events, dt, and the given shared state.
    ///
    /// The unified context replaces the old split `LevelContext` / `RenderContext`.
    /// Levels receive this single context for both update and render.
    ///
    /// ```ignore
    /// let mut ctx = engine.ctx(&mut self.shared);
    /// level.update(&mut ctx);
    /// level.render(&mut ctx);
    /// ```
    ///
    /// Panics if the renderer is not set (should only happen if called before platform init).
    pub fn ctx<'a, S>(&'a mut self, shared: &'a mut S) -> Ctx<'a, S> {
        let renderer = self.renderer.as_mut()
            .expect("Engine::ctx() called before renderer was set")
            .as_mut();
        Ctx {
            input: &self.input,
            dt: self.fixed_dt,
            shared,
            renderer,
            events: &mut self.events,
        }
    }

    /// Access the event bus directly.
    pub fn events(&mut self) -> &mut EventBus<World> {
        &mut self.events
    }

    /// Create a UI system pre-wired to the event bus.
    ///
    /// Events from button clicks are automatically routed through the
    /// `EventBus` via an `EventSink`.
    ///
    /// ```ignore
    /// let ui = engine.create_ui::<MenuAction>(font_bytes)?;
    /// ```
    pub fn create_ui<E: Clone + 'static>(&mut self, font_bytes: Vec<u8>) -> Result<unison_ui::facade::Ui<E>, String> {
        let sink = self.events.create_sink();
        let renderer = self.renderer.as_mut()
            .ok_or("No renderer available")?
            .as_mut();
        unison_ui::facade::Ui::new(font_bytes, renderer, sink)
    }

    // ── Render targets ──

    /// Create an offscreen render target.
    ///
    /// Returns `(target_id, texture_id)`. The texture can be used to draw
    /// the target's contents on screen.
    pub fn create_render_target(&mut self, width: u32, height: u32) -> Result<(RenderTargetId, TextureId), String> {
        match self.renderer.as_mut() {
            Some(r) => r.create_render_target(width, height),
            None => Err("No renderer".into()),
        }
    }

    /// Destroy an offscreen render target (the associated texture is kept).
    pub fn destroy_render_target(&mut self, target: RenderTargetId) {
        if let Some(r) = self.renderer.as_mut() {
            r.destroy_render_target(target);
        }
    }

    // ── Anti-aliasing ──

    /// Set the anti-aliasing mode for rendering.
    ///
    /// Controls MSAA sample count for offscreen render targets. Higher
    /// values produce smoother edges at the cost of GPU memory and fill rate.
    ///
    /// ```ignore
    /// engine.set_anti_aliasing(AntiAliasing::MSAAx4);
    /// ```
    pub fn set_anti_aliasing(&mut self, mode: AntiAliasing) {
        if let Some(r) = self.renderer.as_mut() {
            r.set_anti_aliasing(mode);
        }
    }

    /// Get the current anti-aliasing mode.
    pub fn anti_aliasing(&self) -> AntiAliasing {
        match self.renderer.as_ref() {
            Some(r) => r.anti_aliasing(),
            None => AntiAliasing::None,
        }
    }

    // ── Internal: called by platform game loop ──

    /// Update action state from current input.
    /// Called by the platform's game loop before `Game::update()`.
    #[doc(hidden)]
    pub fn pre_update(&mut self) {
        self.actions.update(&self.input);
    }
}

impl<A: Copy + Eq + Hash> Default for Engine<A> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Copy, Clone, Eq, PartialEq, Hash)]
    enum TestAction {
        Jump,
        Left,
        Right,
    }

    #[test]
    fn action_binding_and_query() {
        let mut engine = Engine::<TestAction>::new();
        engine.bind_key(KeyCode::Space, TestAction::Jump);
        engine.bind_key(KeyCode::ArrowLeft, TestAction::Left);
        engine.bind_key(KeyCode::ArrowRight, TestAction::Right);

        // Simulate a key press
        engine.input.key_pressed(KeyCode::Space);
        engine.pre_update();

        assert!(engine.action_active(TestAction::Jump));
        assert!(engine.action_just_started(TestAction::Jump));
        assert!(!engine.action_active(TestAction::Left));
    }

    #[test]
    fn action_axis_works() {
        let mut engine = Engine::<TestAction>::new();
        engine.bind_key(KeyCode::ArrowLeft, TestAction::Left);
        engine.bind_key(KeyCode::ArrowRight, TestAction::Right);

        engine.input.key_pressed(KeyCode::ArrowRight);
        engine.pre_update();

        assert_eq!(engine.action_axis(TestAction::Left, TestAction::Right), 1.0);
    }
}
