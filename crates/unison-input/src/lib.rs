//! Unison Input — Two-layer input system for the Unison 2D engine.
//!
//! **Layer 1 (Raw):** [`InputState`] tracks keyboard, mouse, and touch state.
//! Platform crates feed native events into it via mutation methods.
//!
//! **Layer 2 (Actions):** [`ActionMap`] maps raw inputs to game-defined actions.
//! Game code reads actions through `Engine` methods, staying platform-agnostic.

mod types;
mod state;
mod action;
mod buffer;

pub use types::{KeyCode, MouseButton, Touch, TouchPhase};
pub use state::InputState;
pub use action::ActionMap;
pub use buffer::InputBuffer;
