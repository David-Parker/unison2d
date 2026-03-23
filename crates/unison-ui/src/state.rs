//! Widget state — persistent per-widget data across frames.
//!
//! Each widget identified by a [`NodeKey`] has a [`WidgetState`] tracking
//! hover, press, animation timers, etc. State is created when a node
//! is Added and removed after its exit animation completes.

use std::collections::HashMap;

use crate::diff::{DiffOp, NodeKey};

/// Exit animation duration in seconds.
pub const EXIT_DURATION: f32 = 0.12;

/// Per-widget state that persists across frames.
#[derive(Clone, Debug)]
pub struct WidgetState {
    /// Whether the mouse is over this widget.
    pub hovered: bool,
    /// Whether the mouse button is pressed on this widget.
    pub pressed: bool,
    /// Whether this widget has keyboard focus.
    pub focused: bool,
    /// Time since this widget was added (seconds). Starts at 0, increases.
    pub animation_time: f32,
    /// Remaining exit animation time. None = not exiting. Some(t) = t seconds left.
    pub exit_time: Option<f32>,
    /// Hover interpolation (0.0 = not hovered, approaches 1.0 when hovered).
    pub hover_time: f32,
}

impl WidgetState {
    pub fn new() -> Self {
        Self {
            hovered: false,
            pressed: false,
            focused: false,
            animation_time: 0.0,
            exit_time: None,
            hover_time: 0.0,
        }
    }

    /// Whether this widget is in its exit animation.
    pub fn is_exiting(&self) -> bool {
        self.exit_time.is_some()
    }
}

/// Manages state for all active widgets.
pub struct UiState {
    states: HashMap<NodeKey, WidgetState>,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
        }
    }

    /// Get the state for a widget, if it exists.
    pub fn get(&self, key: &NodeKey) -> Option<&WidgetState> {
        self.states.get(key)
    }

    /// Get mutable state for a widget.
    pub fn get_mut(&mut self, key: &NodeKey) -> Option<&mut WidgetState> {
        self.states.get_mut(key)
    }

    /// Apply diff operations to create/remove/update state.
    pub fn apply_diff(&mut self, ops: &[DiffOp]) {
        for op in ops {
            match op {
                DiffOp::Added(key) => {
                    // Create fresh state (or revive if it was exiting)
                    self.states.insert(key.clone(), WidgetState::new());
                }
                DiffOp::Removed(key) => {
                    // Start exit animation instead of immediate removal
                    if let Some(state) = self.states.get_mut(key) {
                        if state.exit_time.is_none() {
                            state.exit_time = Some(EXIT_DURATION);
                        }
                    }
                }
                DiffOp::Updated(_) | DiffOp::Unchanged(_) => {
                    // State persists — nothing to do
                }
            }
        }
    }

    /// Advance animation timers and purge completed exit animations.
    pub fn update(&mut self, dt: f32) {
        let hover_speed = 8.0; // reaches 1.0 in ~0.125s

        let mut to_remove = Vec::new();

        for (key, state) in &mut self.states {
            // Advance enter animation
            state.animation_time += dt;

            // Advance hover interpolation
            if state.hovered {
                state.hover_time = (state.hover_time + hover_speed * dt).min(1.0);
            } else {
                state.hover_time = (state.hover_time - hover_speed * dt).max(0.0);
            }

            // Advance exit animation
            if let Some(ref mut t) = state.exit_time {
                *t -= dt;
                if *t <= 0.0 {
                    to_remove.push(key.clone());
                }
            }
        }

        for key in to_remove {
            self.states.remove(&key);
        }
    }

    /// Number of active widget states (including exiting).
    pub fn len(&self) -> usize {
        self.states.len()
    }

    /// Whether there are no active states.
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    /// Get all currently tracked node keys.
    pub fn all_keys(&self) -> Vec<NodeKey> {
        self.states.keys().cloned().collect()
    }

    /// Reset hover and press state for all widgets (called at start of frame).
    pub fn clear_input_state(&mut self) {
        for state in self.states.values_mut() {
            state.hovered = false;
            // pressed persists until mouse release
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(path: &[u16]) -> NodeKey {
        NodeKey { path: path.to_vec() }
    }

    #[test]
    fn state_created_on_added() {
        let mut state = UiState::new();
        state.apply_diff(&[DiffOp::Added(key(&[0]))]);
        assert!(state.get(&key(&[0])).is_some());
        assert_eq!(state.get(&key(&[0])).unwrap().animation_time, 0.0);
    }

    #[test]
    fn state_exit_on_removed() {
        let mut state = UiState::new();
        state.apply_diff(&[DiffOp::Added(key(&[0]))]);
        state.apply_diff(&[DiffOp::Removed(key(&[0]))]);
        let ws = state.get(&key(&[0])).unwrap();
        assert!(ws.is_exiting());
        assert_eq!(ws.exit_time, Some(EXIT_DURATION));
    }

    #[test]
    fn timer_advancement() {
        let mut state = UiState::new();
        state.apply_diff(&[DiffOp::Added(key(&[0]))]);
        state.update(0.1);
        state.update(0.1);
        state.update(0.1);
        let ws = state.get(&key(&[0])).unwrap();
        assert!((ws.animation_time - 0.3).abs() < 0.001);
    }

    #[test]
    fn state_purged_after_exit() {
        let mut state = UiState::new();
        state.apply_diff(&[DiffOp::Added(key(&[0]))]);
        state.apply_diff(&[DiffOp::Removed(key(&[0]))]);
        // Advance past exit duration
        state.update(EXIT_DURATION + 0.01);
        assert!(state.get(&key(&[0])).is_none());
        assert!(state.is_empty());
    }

    #[test]
    fn hover_time_interpolation() {
        let mut state = UiState::new();
        state.apply_diff(&[DiffOp::Added(key(&[0]))]);
        state.get_mut(&key(&[0])).unwrap().hovered = true;
        state.update(0.05);
        let ht = state.get(&key(&[0])).unwrap().hover_time;
        assert!(ht > 0.0 && ht < 1.0);

        // Unhover
        state.get_mut(&key(&[0])).unwrap().hovered = false;
        state.update(1.0); // long enough to reach 0
        assert_eq!(state.get(&key(&[0])).unwrap().hover_time, 0.0);
    }

    #[test]
    fn rapid_add_remove_cycle() {
        let mut state = UiState::new();
        // Frame 1: add
        state.apply_diff(&[DiffOp::Added(key(&[0]))]);
        state.update(0.016);
        // Frame 2: remove
        state.apply_diff(&[DiffOp::Removed(key(&[0]))]);
        state.update(0.016);
        // Frame 3: add again (before exit completes)
        state.apply_diff(&[DiffOp::Added(key(&[0]))]);
        let ws = state.get(&key(&[0])).unwrap();
        // Should be fresh state, not still exiting
        assert!(!ws.is_exiting());
        assert_eq!(ws.animation_time, 0.0);
    }

    #[test]
    fn multi_frame_lifecycle() {
        let mut state = UiState::new();

        // Frames 1-3: add two widgets
        state.apply_diff(&[
            DiffOp::Added(key(&[0])),
            DiffOp::Added(key(&[1])),
        ]);
        for _ in 0..3 {
            state.update(0.016);
        }
        assert_eq!(state.len(), 2);

        // Frames 4-5: remove first widget
        state.apply_diff(&[DiffOp::Removed(key(&[0]))]);
        state.update(0.016);
        assert_eq!(state.len(), 2); // still present during exit

        // Frame 6+: exit completes
        state.update(EXIT_DURATION);
        assert_eq!(state.len(), 1);
        assert!(state.get(&key(&[0])).is_none());
        assert!(state.get(&key(&[1])).is_some());
    }
}
