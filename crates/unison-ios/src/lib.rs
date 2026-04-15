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

// Keep the audio FFI symbols (`engine_audio_suspend`, `engine_audio_resume_system`,
// `engine_audio_arm`) alive through static-lib linking so iOS (and any other
// consumer of this crate) can link against them from Swift. Without an explicit
// reference from the platform crate, the linker's dead-code elimination may
// strip them — the `#[no_mangle]` only guarantees the export symbol exists in
// the `unison2d` rlib, not that it survives the downstream staticlib link.
#[doc(hidden)]
#[allow(dead_code, non_upper_case_globals)]
pub static __UNISON_IOS_AUDIO_FFI_KEEPALIVE: [unsafe extern "C" fn(*mut unison2d::Engine); 3] = [
    unison2d::engine_audio_suspend,
    unison2d::engine_audio_resume_system,
    unison2d::engine_audio_arm,
];
