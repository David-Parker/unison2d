//! Unison iOS — iOS platform crate for the Unison 2D engine.
//!
//! Provides:
//! - Metal renderer (implements the `Renderer` trait)
//! - Touch input helpers (feeds UIKit events into `InputBuffer`)
//! - `GameState<G>` frame loop (called from Swift via FFI)
//! - `export_game!` macro to generate all FFI boilerplate
//!
//! This crate is **generic** — it knows nothing about any specific game.
//!
//! # Usage
//!
//! ```ignore
//! // In your game crate — this is ALL the iOS code you need:
//! #[cfg(feature = "ios")]
//! unison_ios::export_game!(MyGame, MyGame::new());
//! ```

mod renderer;
mod shaders;
pub mod input;
mod game_loop;
mod export_macro;

pub use renderer::MetalRenderer;
pub use game_loop::GameState;
