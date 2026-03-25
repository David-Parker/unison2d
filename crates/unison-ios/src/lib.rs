//! Unison iOS — iOS platform crate for the Unison 2D engine.
//!
//! Provides:
//! - Metal renderer (implements the `Renderer` trait)
//! - Touch input helpers (feeds UIKit events into `InputBuffer`)
//! - `GameState<G>` frame loop (called from Swift via FFI)
//!
//! This crate is **generic** — it knows nothing about any specific game.
//! The game crate (e.g., `donut-game`) provides the concrete `Game` type
//! and the `#[no_mangle] extern "C"` FFI entry points.
//!
//! # Usage
//!
//! ```ignore
//! // In your game crate's ios_ffi.rs:
//! use unison_ios::{MetalRenderer, GameState};
//! use unison2d::Engine;
//!
//! #[no_mangle]
//! pub extern "C" fn game_init(
//!     device: *mut std::ffi::c_void,
//!     layer: *mut std::ffi::c_void,
//!     width: f32,
//!     height: f32,
//! ) -> *mut std::ffi::c_void {
//!     let renderer = unsafe {
//!         MetalRenderer::new(device as _, layer as _, width, height)
//!     }.expect("Failed to create renderer");
//!
//!     let mut engine = Engine::<MyAction>::new();
//!     engine.renderer = Some(Box::new(renderer));
//!
//!     let mut state = GameState::new(MyGame::new(), engine);
//!     state.init();
//!
//!     Box::into_raw(Box::new(state)) as *mut std::ffi::c_void
//! }
//! ```

mod renderer;
mod shaders;
pub mod input;
mod game_loop;

pub use renderer::MetalRenderer;
pub use game_loop::GameState;
