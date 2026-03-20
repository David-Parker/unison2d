//! Unison Assets — build-time asset embedding and runtime asset store.
//!
//! Assets are compressed at build time via `build.rs` and baked into the binary
//! as `&[u8]` slices. At runtime, [`AssetStore`] decompresses and serves them
//! by relative path.
//!
//! # Build-time (in your game's `build.rs`)
//!
//! ```ignore
//! // build.rs
//! fn main() {
//!     unison_assets::build::embed_assets("project/assets", "assets.rs");
//! }
//! ```
//!
//! # Runtime (in your game code)
//!
//! ```ignore
//! // Include the generated asset table
//! mod assets {
//!     include!(concat!(env!("OUT_DIR"), "/assets.rs"));
//! }
//!
//! // In Game::init:
//! engine.assets_mut().load_embedded(assets::ASSETS);
//! let png_bytes = engine.assets().get("textures/donut-pink.png");
//! ```

mod store;

#[cfg(feature = "build")]
pub mod build;

pub use store::AssetStore;
