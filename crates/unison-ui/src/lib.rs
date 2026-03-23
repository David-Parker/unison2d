//! Unison UI — declarative, React-like UI system for the Unison 2D engine.
//!
//! Game code describes a UI tree each frame. The system diffs it against
//! the previous frame, manages widget state (hover, focus, animations),
//! and emits [`RenderCommand`]s through the engine's overlay system.

pub mod text;
