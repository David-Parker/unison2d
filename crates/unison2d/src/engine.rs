//! Engine — thin platform bridge for input, actions, and rendering.
//!
//! The engine does NOT own a world. Games create and manage their own
//! `World` instances (typically through `Level` structs). Engine provides:
//! - Input → action mapping
//! - Access to the renderer
//! - Fixed timestep delta

use std::hash::Hash;

use unison_input::{ActionMap, InputState, KeyCode, MouseButton};
use unison_math::{Color, Rect};
use unison_render::{Renderer, RenderCommand, DrawSprite, TextureId, RenderTargetId, Camera};

/// The engine struct. Manages input, actions, and renderer access.
///
/// Games receive `&mut Engine<A>` in their `init()`, `update()`, and `render()`
/// callbacks. Use it for input bindings and action queries. For physics, objects,
/// and cameras, use your `World` directly.
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
}

impl<A: Copy + Eq + Hash> Engine<A> {
    /// Create a new engine with default settings.
    pub fn new() -> Self {
        Self {
            input: InputState::new(),
            actions: ActionMap::new(),
            renderer: None,
            fixed_dt: 1.0 / 60.0,
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

    // ── Render targets ──

    /// Create an offscreen render target.
    ///
    /// Returns `(target_id, texture_id)`. The texture can be used in
    /// `composite_layer()` to draw the target's contents on screen.
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

    // ── Compositing ──

    /// Begin compositing: bind the screen, set up a screen-space camera, and clear.
    ///
    /// After this call, use `composite_layer()` to draw render target textures
    /// onto the screen. Finish with `end_composite()`.
    ///
    /// Screen-space coordinates use a 1×1 unit camera where (0,0) is bottom-left
    /// and (1,1) is top-right.
    pub fn begin_composite(&mut self, clear_color: Color) {
        if let Some(r) = self.renderer.as_mut() {
            r.bind_render_target(RenderTargetId::SCREEN);
            let mut cam = Camera::new(1.0, 1.0);
            cam.set_position(0.5, 0.5);
            r.begin_frame(&cam);
            r.clear(clear_color);
        }
    }

    /// Draw a render-target texture onto the screen.
    ///
    /// `screen_rect` uses normalized coordinates: (0,0) is bottom-left, (1,1) is
    /// top-right. For a full-screen quad, use `Rect::from_position(Vec2::ZERO, Vec2::ONE)`.
    pub fn composite_layer(&mut self, texture: TextureId, screen_rect: Rect) {
        if let Some(r) = self.renderer.as_mut() {
            let center = screen_rect.center();
            let size = screen_rect.size();
            r.draw(RenderCommand::Sprite(DrawSprite {
                texture,
                position: [center.x, center.y],
                size: [size.x, size.y],
                rotation: 0.0,
                uv: [0.0, 0.0, 1.0, 1.0],
                color: Color::WHITE,
            }));
        }
    }

    /// Finish compositing and present the frame.
    pub fn end_composite(&mut self) {
        if let Some(r) = self.renderer.as_mut() {
            r.end_frame();
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
