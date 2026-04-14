//! Unison Assets — build-time asset embedding and runtime asset store.
//!
//! Assets are compressed at build time via `build.rs` and baked into the binary
//! as `&[u8]` slices. At runtime, [`AssetStore`] decompresses and serves them
//! by relative path.
//!
//! Game code loads assets via `unison.assets.load_texture("path")` in Lua/TypeScript.
//! See [scripting docs](../docs/api/scripting.md) for the Lua API.
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
//! # Runtime (internal engine use)
//!
//! ```ignore
//! // Include the generated asset table
//! mod assets {
//!     include!(concat!(env!("OUT_DIR"), "/assets.rs"));
//! }
//!
//! // ScriptedGame::from_asset loads embedded assets automatically.
//! // For direct engine use:
//! engine.assets_mut().load_embedded(assets::ASSETS);
//! let png_bytes = engine.assets().get("textures/donut-pink.png");
//! ```

mod store;

#[cfg(feature = "build")]
pub mod build;

pub use store::{AssetStore, EmbeddedAsset};
