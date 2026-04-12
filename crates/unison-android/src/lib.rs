//! Unison Android — Android platform crate for the Unison 2D engine.
//!
//! Provides:
//! - OpenGL ES 3.0 renderer (implements the `Renderer` trait)
//! - Touch input helpers (feeds Android events into `InputBuffer`)
//! - `GameState<G>` frame loop (called from Kotlin via JNI)
//! - `export_game!` macro to generate all JNI boilerplate
//!
//! This crate is **generic** — it knows nothing about any specific game.
//!
//! # Usage
//!
//! ```ignore
//! // In your game crate — this is ALL the Android code you need:
//! #[cfg(feature = "android")]
//! unison_android::export_game!(MyGame, MyGame::new());
//! ```

mod renderer;
mod shaders;
pub mod input;
mod game_loop;
mod export_macro;

pub use renderer::GlesRenderer;
pub use game_loop::GameState;

// Re-export `jni` so the `export_game!` macro expansion can refer to it as
// `$crate::jni::...` without the game crate needing a direct `jni` dep.
#[doc(hidden)]
pub use jni;
