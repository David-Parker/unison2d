//! Raw input state — platform crates feed events into this

use std::collections::{HashMap, HashSet};
use unison_math::Vec2;
use crate::types::{KeyCode, MouseButton, Touch, TouchPhase};

/// Raw input state for all input devices.
///
/// Platform crates (e.g., unison-web) call the mutation methods (`key_pressed`, `mouse_moved`, etc.)
/// to feed native events into the state. Game code reads state through `Engine`, but raw access
/// is available via `engine.input_state()`.
pub struct InputState {
    // Keyboard
    keys_held: HashSet<KeyCode>,
    keys_just_pressed: HashSet<KeyCode>,
    keys_just_released: HashSet<KeyCode>,

    // Mouse
    mouse_pos: Vec2,
    mouse_buttons_held: HashSet<MouseButton>,
    mouse_buttons_just_pressed: HashSet<MouseButton>,
    mouse_buttons_just_released: HashSet<MouseButton>,

    // Touch
    touches: HashMap<u64, Touch>,
    touches_just_began: Vec<u64>,
    touches_just_ended: Vec<u64>,

    // Axis (e.g., virtual joystick)
    axis: Vec2,
}

impl InputState {
    /// Create a new empty input state
    pub fn new() -> Self {
        Self {
            keys_held: HashSet::new(),
            keys_just_pressed: HashSet::new(),
            keys_just_released: HashSet::new(),
            mouse_pos: Vec2::ZERO,
            mouse_buttons_held: HashSet::new(),
            mouse_buttons_just_pressed: HashSet::new(),
            mouse_buttons_just_released: HashSet::new(),
            touches: HashMap::new(),
            touches_just_began: Vec::new(),
            touches_just_ended: Vec::new(),
            axis: Vec2::ZERO,
        }
    }

    // ── Frame management ──

    /// Call at the start of each frame to transition per-frame state.
    /// Clears just_pressed / just_released flags and updates touch phases.
    pub fn begin_frame(&mut self) {
        self.keys_just_pressed.clear();
        self.keys_just_released.clear();
        self.mouse_buttons_just_pressed.clear();
        self.mouse_buttons_just_released.clear();

        // Remove ended/cancelled touches, transition Began→Stationary
        self.touches.retain(|_, t| {
            t.phase != TouchPhase::Ended && t.phase != TouchPhase::Cancelled
        });
        for touch in self.touches.values_mut() {
            match touch.phase {
                TouchPhase::Began | TouchPhase::Moved => {
                    touch.phase = TouchPhase::Stationary;
                }
                _ => {}
            }
        }
        self.touches_just_began.clear();
        self.touches_just_ended.clear();
    }

    // ── Keyboard queries ──

    /// Is the key currently held down?
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys_held.contains(&key)
    }

    /// Was the key pressed this frame (not held last frame)?
    pub fn is_key_just_pressed(&self, key: KeyCode) -> bool {
        self.keys_just_pressed.contains(&key)
    }

    /// Was the key released this frame?
    pub fn is_key_just_released(&self, key: KeyCode) -> bool {
        self.keys_just_released.contains(&key)
    }

    // ── Mouse queries ──

    /// Current mouse position in screen coordinates
    pub fn mouse_position(&self) -> Vec2 {
        self.mouse_pos
    }

    /// Is the mouse button currently held?
    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons_held.contains(&button)
    }

    /// Was the mouse button pressed this frame?
    pub fn is_mouse_just_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons_just_pressed.contains(&button)
    }

    /// Was the mouse button released this frame?
    pub fn is_mouse_just_released(&self, button: MouseButton) -> bool {
        self.mouse_buttons_just_released.contains(&button)
    }

    // ── Touch queries ──

    /// All currently active touches
    pub fn active_touches(&self) -> Vec<&Touch> {
        self.touches.values().collect()
    }

    /// Touches that began this frame
    pub fn touches_just_began(&self) -> Vec<&Touch> {
        self.touches_just_began.iter()
            .filter_map(|id| self.touches.get(id))
            .collect()
    }

    /// Touches that ended this frame
    pub fn touches_just_ended(&self) -> Vec<&Touch> {
        self.touches_just_ended.iter()
            .filter_map(|id| self.touches.get(id))
            .collect()
    }

    /// Get a specific touch by ID
    pub fn get_touch(&self, id: u64) -> Option<&Touch> {
        self.touches.get(&id)
    }

    // ── Axis queries ──

    /// Current axis value (e.g., from a virtual joystick). Each component is in -1.0..=1.0.
    pub fn axis(&self) -> Vec2 {
        self.axis
    }

    // ── Transfer API ──

    /// Copy held-key and held-mouse-button state from another InputState.
    /// Used after swapping input buffers so the shared buffer starts with
    /// the correct held state for processing key-release events next frame.
    pub fn copy_held_from(&mut self, other: &InputState) {
        self.keys_held = other.keys_held.clone();
        self.mouse_buttons_held = other.mouse_buttons_held.clone();
        self.mouse_pos = other.mouse_pos;
        self.touches = other.touches.clone();
        self.axis = other.axis;
    }

    // ── Platform mutation API ──
    // Platform crates call these to feed native events in.

    /// Record a key press (call from platform layer)
    pub fn key_pressed(&mut self, key: KeyCode) {
        if self.keys_held.insert(key) {
            self.keys_just_pressed.insert(key);
        }
    }

    /// Record a key release (call from platform layer)
    pub fn key_released(&mut self, key: KeyCode) {
        if self.keys_held.remove(&key) {
            self.keys_just_released.insert(key);
        }
    }

    /// Record mouse movement (call from platform layer)
    pub fn mouse_moved(&mut self, x: f32, y: f32) {
        self.mouse_pos = Vec2::new(x, y);
    }

    /// Record a mouse button press (call from platform layer)
    pub fn mouse_button_pressed(&mut self, button: MouseButton) {
        if self.mouse_buttons_held.insert(button) {
            self.mouse_buttons_just_pressed.insert(button);
        }
    }

    /// Record a mouse button release (call from platform layer)
    pub fn mouse_button_released(&mut self, button: MouseButton) {
        if self.mouse_buttons_held.remove(&button) {
            self.mouse_buttons_just_released.insert(button);
        }
    }

    /// Record a touch start (call from platform layer)
    pub fn touch_started(&mut self, id: u64, x: f32, y: f32) {
        self.touches.insert(id, Touch {
            id,
            position: Vec2::new(x, y),
            phase: TouchPhase::Began,
        });
        self.touches_just_began.push(id);
    }

    /// Record a touch move (call from platform layer)
    pub fn touch_moved(&mut self, id: u64, x: f32, y: f32) {
        if let Some(touch) = self.touches.get_mut(&id) {
            touch.position = Vec2::new(x, y);
            touch.phase = TouchPhase::Moved;
        }
    }

    /// Record a touch end (call from platform layer)
    pub fn touch_ended(&mut self, id: u64) {
        if let Some(touch) = self.touches.get_mut(&id) {
            touch.phase = TouchPhase::Ended;
            self.touches_just_ended.push(id);
        }
    }

    /// Record a touch cancel (call from platform layer)
    pub fn touch_cancelled(&mut self, id: u64) {
        if let Some(touch) = self.touches.get_mut(&id) {
            touch.phase = TouchPhase::Cancelled;
            self.touches_just_ended.push(id);
        }
    }

    /// Set the axis value (call from platform layer, e.g., virtual joystick).
    /// Each component is clamped to -1.0..=1.0.
    pub fn set_axis(&mut self, x: f32, y: f32) {
        self.axis = Vec2::new(x.clamp(-1.0, 1.0), y.clamp(-1.0, 1.0));
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_just_pressed_one_frame() {
        let mut input = InputState::new();
        input.key_pressed(KeyCode::Space);

        assert!(input.is_key_pressed(KeyCode::Space));
        assert!(input.is_key_just_pressed(KeyCode::Space));

        // Next frame: still held, no longer "just pressed"
        input.begin_frame();
        assert!(input.is_key_pressed(KeyCode::Space));
        assert!(!input.is_key_just_pressed(KeyCode::Space));
    }

    #[test]
    fn key_just_released_one_frame() {
        let mut input = InputState::new();
        input.key_pressed(KeyCode::Space);
        input.begin_frame();
        input.key_released(KeyCode::Space);

        assert!(!input.is_key_pressed(KeyCode::Space));
        assert!(input.is_key_just_released(KeyCode::Space));

        input.begin_frame();
        assert!(!input.is_key_just_released(KeyCode::Space));
    }

    #[test]
    fn duplicate_key_press_ignored() {
        let mut input = InputState::new();
        input.key_pressed(KeyCode::A);
        input.key_pressed(KeyCode::A); // duplicate

        assert!(input.is_key_just_pressed(KeyCode::A));
        assert!(input.is_key_pressed(KeyCode::A));
    }

    #[test]
    fn mouse_button_transitions() {
        let mut input = InputState::new();
        input.mouse_button_pressed(MouseButton::Left);

        assert!(input.is_mouse_pressed(MouseButton::Left));
        assert!(input.is_mouse_just_pressed(MouseButton::Left));

        input.begin_frame();
        assert!(input.is_mouse_pressed(MouseButton::Left));
        assert!(!input.is_mouse_just_pressed(MouseButton::Left));

        input.mouse_button_released(MouseButton::Left);
        assert!(!input.is_mouse_pressed(MouseButton::Left));
        assert!(input.is_mouse_just_released(MouseButton::Left));
    }

    #[test]
    fn mouse_position_tracks() {
        let mut input = InputState::new();
        assert_eq!(input.mouse_position(), Vec2::ZERO);

        input.mouse_moved(100.0, 200.0);
        assert_eq!(input.mouse_position(), Vec2::new(100.0, 200.0));
    }

    #[test]
    fn touch_lifecycle() {
        let mut input = InputState::new();

        // Touch starts
        input.touch_started(1, 50.0, 100.0);
        assert_eq!(input.active_touches().len(), 1);
        assert_eq!(input.touches_just_began().len(), 1);
        assert_eq!(input.touches_just_began()[0].id, 1);

        // Next frame: touch is stationary
        input.begin_frame();
        assert_eq!(input.active_touches().len(), 1);
        assert_eq!(input.active_touches()[0].phase, TouchPhase::Stationary);
        assert_eq!(input.touches_just_began().len(), 0);

        // Touch moves
        input.touch_moved(1, 60.0, 110.0);
        assert_eq!(input.active_touches()[0].phase, TouchPhase::Moved);
        assert_eq!(input.active_touches()[0].position, Vec2::new(60.0, 110.0));

        // Touch ends
        input.touch_ended(1);
        assert_eq!(input.touches_just_ended().len(), 1);

        // Next frame: touch removed
        input.begin_frame();
        assert_eq!(input.active_touches().len(), 0);
    }

    #[test]
    fn multi_touch() {
        let mut input = InputState::new();
        input.touch_started(1, 10.0, 20.0);
        input.touch_started(2, 30.0, 40.0);

        assert_eq!(input.active_touches().len(), 2);
        assert_eq!(input.touches_just_began().len(), 2);
    }

    #[test]
    fn axis_defaults_to_zero() {
        let input = InputState::new();
        assert_eq!(input.axis(), Vec2::ZERO);
    }

    #[test]
    fn axis_clamps_to_unit_range() {
        let mut input = InputState::new();
        input.set_axis(2.0, -3.0);
        assert_eq!(input.axis(), Vec2::new(1.0, -1.0));

        input.set_axis(-0.5, 0.7);
        assert_eq!(input.axis(), Vec2::new(-0.5, 0.7));
    }

    #[test]
    fn axis_survives_begin_frame() {
        let mut input = InputState::new();
        input.set_axis(0.8, -0.3);
        input.begin_frame();
        assert_eq!(input.axis(), Vec2::new(0.8, -0.3));
    }
}
