//! Unison UI — declarative, React-like UI system for the Unison 2D engine.
//!
//! Game code describes a UI tree each frame. The system diffs it against
//! the previous frame, manages widget state (hover, focus, animations),
//! and emits [`RenderCommand`]s through the engine's overlay system.

pub mod diff;
pub mod facade;
pub mod input;
pub mod layout;
pub mod node;
pub mod render;
pub mod state;
pub mod style;
pub mod text;

// Macro module (macros are exported at crate root via #[macro_export])
#[doc(hidden)]
pub mod ui_macro;
