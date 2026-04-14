//! Unison Input — Raw input state for the Unison 2D engine.
//!
//! [`InputState`] tracks keyboard, mouse, and touch state. Platform crates feed
//! native events into it via mutation methods. The scripting layer exposes a
//! Lua-facing action map built on top of raw input.

mod types;
mod state;
mod buffer;

pub use types::{KeyCode, MouseButton, Touch, TouchPhase};
pub use state::InputState;
pub use buffer::InputBuffer;
