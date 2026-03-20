# unison-assets

Build-time asset embedding and runtime asset store. Files in an asset directory are gzip-compressed at build time and baked into the binary via `include_bytes!`. At runtime, `AssetStore` decompresses and serves them by relative path.

Works on all platforms (WASM, iOS, Android) — no filesystem APIs needed at runtime.

## Build-Time: `embed_assets`

Called from your game's `build.rs`. Requires the `build` feature.

```rust
// build.rs
fn main() {
    unison_assets::build::embed_assets("project/assets", "assets.rs");
}
```

| Parameter | Description |
|-----------|-------------|
| `asset_dir` | Path to the asset directory (relative to crate root) |
| `output_filename` | Name of the generated Rust file (written to `$OUT_DIR`) |

The function:
1. Walks `asset_dir` recursively
2. Gzip-compresses each file (best compression)
3. Writes compressed files to `$OUT_DIR/_assets_compressed/`
4. Generates a Rust source file with an `ASSETS` constant containing `include_bytes!` entries
5. Emits `cargo:rerun-if-changed` for each asset file and the directory itself

## Runtime: `AssetStore`

Lives on `Engine`. Load the generated table once at startup, then query by path.

```rust
mod assets {
    include!(concat!(env!("OUT_DIR"), "/assets.rs"));
}

// In Game::init:
engine.assets_mut().load_embedded(assets::ASSETS);

// Later:
let png_bytes = engine.assets().get("textures/donut-pink.png");
```

### Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `-> AssetStore` | Create an empty store |
| `load_embedded(table)` | `(&mut self, &[EmbeddedAsset])` | Decompress and load a generated asset table |
| `get(path)` | `(&self, &str) -> Option<&[u8]>` | Get asset bytes by relative path |
| `contains(path)` | `(&self, &str) -> bool` | Check if an asset exists |
| `len()` | `(&self) -> usize` | Number of loaded assets |
| `is_empty()` | `(&self) -> bool` | Whether the store is empty |
| `paths()` | `(&self) -> impl Iterator<Item = &str>` | Iterate all asset paths |

### Types

```rust
/// A (path, compressed_bytes) pair from the generated asset table.
pub type EmbeddedAsset = (&'static str, &'static [u8]);
```

## Asset Keys

Keys are relative paths from the asset directory root, using forward slashes:

```
project/assets/
├── textures/
│   ├── donut-pink.png    → "textures/donut-pink.png"
│   └── donut-chocolate.png → "textures/donut-chocolate.png"
└── levels/
    └── level1.json       → "levels/level1.json"
```

## Dependencies

- `flate2` — gzip compression (build time) and decompression (runtime)
