//! Action mapping — maps raw inputs to game-defined actions

use std::collections::HashMap;
use std::hash::Hash;
use unison_core::Rect;
use crate::types::{KeyCode, MouseButton, TouchPhase};
use crate::state::InputState;

/// A binding from a raw input to an action
#[derive(Debug, Clone)]
enum Binding<A> {
    Key(KeyCode, A),
    MouseButton(MouseButton, A),
    TouchRegion(Rect, A),
}

/// Maps raw inputs to game-defined actions.
///
/// The game defines an action enum (any `Copy + Eq + Hash` type), then binds
/// raw inputs to those actions. Game code reads actions through `Engine` methods
/// like `action_active()` and `action_just_started()`.
///
/// ```ignore
/// #[derive(Copy, Clone, Eq, PartialEq, Hash)]
/// enum Action { Jump, MoveLeft, MoveRight }
///
/// // In Game::init():
/// engine.bind_key(KeyCode::Space, Action::Jump);
/// engine.bind_key(KeyCode::ArrowLeft, Action::MoveLeft);
///
/// // In Game::update():
/// if engine.action_just_started(Action::Jump) { /* jump! */ }
/// let move_x = engine.action_axis(Action::MoveLeft, Action::MoveRight);
/// ```
pub struct ActionMap<A: Copy + Eq + Hash> {
    bindings: Vec<Binding<A>>,
    // Cached action states, updated each frame
    active: HashMap<A, bool>,
    just_started: HashMap<A, bool>,
    just_ended: HashMap<A, bool>,
}

impl<A: Copy + Eq + Hash> ActionMap<A> {
    /// Create a new empty action map
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
            active: HashMap::new(),
            just_started: HashMap::new(),
            just_ended: HashMap::new(),
        }
    }

    // ── Binding ──

    /// Bind a keyboard key to an action
    pub fn bind_key(&mut self, key: KeyCode, action: A) {
        self.bindings.push(Binding::Key(key, action));
    }

    /// Bind a mouse button to an action
    pub fn bind_mouse_button(&mut self, button: MouseButton, action: A) {
        self.bindings.push(Binding::MouseButton(button, action));
    }

    /// Bind a touch region to an action (for mobile platforms).
    /// Any touch inside the rect activates the action.
    pub fn bind_touch_region(&mut self, region: Rect, action: A) {
        self.bindings.push(Binding::TouchRegion(region, action));
    }

    /// Remove all bindings
    pub fn clear_bindings(&mut self) {
        self.bindings.clear();
    }

    // ── Update (called by Engine each frame) ──

    /// Evaluate all bindings against current input state.
    /// Must be called once per frame after `InputState::begin_frame()`.
    pub fn update(&mut self, input: &InputState) {
        self.active.clear();
        self.just_started.clear();
        self.just_ended.clear();

        for binding in &self.bindings {
            match binding {
                Binding::Key(key, action) => {
                    if input.is_key_pressed(*key) {
                        self.active.insert(*action, true);
                    }
                    if input.is_key_just_pressed(*key) {
                        self.just_started.insert(*action, true);
                    }
                    if input.is_key_just_released(*key) {
                        self.just_ended.insert(*action, true);
                    }
                }
                Binding::MouseButton(button, action) => {
                    if input.is_mouse_pressed(*button) {
                        self.active.insert(*action, true);
                    }
                    if input.is_mouse_just_pressed(*button) {
                        self.just_started.insert(*action, true);
                    }
                    if input.is_mouse_just_released(*button) {
                        self.just_ended.insert(*action, true);
                    }
                }
                Binding::TouchRegion(region, action) => {
                    // Check if any active touch is inside the region
                    for touch in input.active_touches() {
                        if region.contains(touch.position) {
                            self.active.insert(*action, true);
                            if touch.phase == TouchPhase::Began {
                                self.just_started.insert(*action, true);
                            }
                        }
                    }
                    // Check ended touches for just_ended
                    for touch in input.touches_just_ended() {
                        if region.contains(touch.position) {
                            self.just_ended.insert(*action, true);
                        }
                    }
                }
            }
        }
    }

    // ── Queries (what game code reads) ──

    /// Is the action currently active? (any bound input is held)
    pub fn is_action_active(&self, action: A) -> bool {
        self.active.get(&action).copied().unwrap_or(false)
    }

    /// Did the action just start this frame?
    pub fn is_action_just_started(&self, action: A) -> bool {
        self.just_started.get(&action).copied().unwrap_or(false)
    }

    /// Did the action just end this frame?
    pub fn is_action_just_ended(&self, action: A) -> bool {
        self.just_ended.get(&action).copied().unwrap_or(false)
    }

    /// Get an axis value from two opposing actions.
    /// Returns -1.0 if negative is active, +1.0 if positive is active, 0.0 if neither/both.
    pub fn axis_value(&self, negative: A, positive: A) -> f32 {
        let neg = if self.is_action_active(negative) { -1.0 } else { 0.0 };
        let pos = if self.is_action_active(positive) { 1.0 } else { 0.0 };
        neg + pos
    }
}

impl<A: Copy + Eq + Hash> Default for ActionMap<A> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
    enum TestAction {
        Jump,
        MoveLeft,
        MoveRight,
        Shoot,
    }

    #[test]
    fn key_binding_active() {
        let mut input = InputState::new();
        let mut actions = ActionMap::new();
        actions.bind_key(KeyCode::Space, TestAction::Jump);

        input.key_pressed(KeyCode::Space);
        actions.update(&input);

        assert!(actions.is_action_active(TestAction::Jump));
        assert!(actions.is_action_just_started(TestAction::Jump));
        assert!(!actions.is_action_just_ended(TestAction::Jump));
    }

    #[test]
    fn key_binding_just_started_clears_next_frame() {
        let mut input = InputState::new();
        let mut actions = ActionMap::new();
        actions.bind_key(KeyCode::Space, TestAction::Jump);

        input.key_pressed(KeyCode::Space);
        actions.update(&input);
        assert!(actions.is_action_just_started(TestAction::Jump));

        // Next frame
        input.begin_frame();
        actions.update(&input);
        assert!(actions.is_action_active(TestAction::Jump));
        assert!(!actions.is_action_just_started(TestAction::Jump));
    }

    #[test]
    fn key_binding_just_ended() {
        let mut input = InputState::new();
        let mut actions = ActionMap::new();
        actions.bind_key(KeyCode::Space, TestAction::Jump);

        input.key_pressed(KeyCode::Space);
        actions.update(&input);
        input.begin_frame();

        input.key_released(KeyCode::Space);
        actions.update(&input);

        assert!(!actions.is_action_active(TestAction::Jump));
        assert!(actions.is_action_just_ended(TestAction::Jump));
    }

    #[test]
    fn axis_value_both_directions() {
        let mut input = InputState::new();
        let mut actions = ActionMap::new();
        actions.bind_key(KeyCode::ArrowLeft, TestAction::MoveLeft);
        actions.bind_key(KeyCode::ArrowRight, TestAction::MoveRight);

        // Neither pressed
        actions.update(&input);
        assert_eq!(actions.axis_value(TestAction::MoveLeft, TestAction::MoveRight), 0.0);

        // Left pressed
        input.key_pressed(KeyCode::ArrowLeft);
        actions.update(&input);
        assert_eq!(actions.axis_value(TestAction::MoveLeft, TestAction::MoveRight), -1.0);

        // Both pressed
        input.key_pressed(KeyCode::ArrowRight);
        actions.update(&input);
        assert_eq!(actions.axis_value(TestAction::MoveLeft, TestAction::MoveRight), 0.0);

        // Only right pressed
        input.key_released(KeyCode::ArrowLeft);
        actions.update(&input);
        assert_eq!(actions.axis_value(TestAction::MoveLeft, TestAction::MoveRight), 1.0);
    }

    #[test]
    fn multiple_keys_same_action() {
        let mut input = InputState::new();
        let mut actions = ActionMap::new();
        actions.bind_key(KeyCode::Space, TestAction::Jump);
        actions.bind_key(KeyCode::W, TestAction::Jump);

        // Either key triggers the action
        input.key_pressed(KeyCode::W);
        actions.update(&input);
        assert!(actions.is_action_active(TestAction::Jump));
        assert!(actions.is_action_just_started(TestAction::Jump));
    }

    #[test]
    fn mouse_binding() {
        let mut input = InputState::new();
        let mut actions = ActionMap::new();
        actions.bind_mouse_button(MouseButton::Left, TestAction::Shoot);

        input.mouse_button_pressed(MouseButton::Left);
        actions.update(&input);

        assert!(actions.is_action_active(TestAction::Shoot));
        assert!(actions.is_action_just_started(TestAction::Shoot));
    }

    #[test]
    fn touch_region_binding() {
        let mut input = InputState::new();
        let mut actions = ActionMap::new();

        // Define a jump button region at bottom-right of screen
        let jump_region = Rect::from_center(
            unison_core::Vec2::new(700.0, 500.0),
            unison_core::Vec2::new(100.0, 100.0),
        );
        actions.bind_touch_region(jump_region, TestAction::Jump);

        // Touch inside region
        input.touch_started(1, 700.0, 500.0);
        actions.update(&input);
        assert!(actions.is_action_active(TestAction::Jump));
        assert!(actions.is_action_just_started(TestAction::Jump));

        // Touch outside region
        input.begin_frame();
        let mut input2 = InputState::new();
        input2.touch_started(2, 100.0, 100.0);
        actions.update(&input2);
        assert!(!actions.is_action_active(TestAction::Jump));
    }

    #[test]
    fn clear_bindings() {
        let mut input = InputState::new();
        let mut actions = ActionMap::new();
        actions.bind_key(KeyCode::Space, TestAction::Jump);
        actions.clear_bindings();

        input.key_pressed(KeyCode::Space);
        actions.update(&input);
        assert!(!actions.is_action_active(TestAction::Jump));
    }
}
