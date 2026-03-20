# Unison 2D

A Rust game engine built for the LLM agent era. No GUIs — everything is controlled through code and configuration. Platform-agnostic: compile for Web, iOS, and Android from the same codebase.

**For the complete engine API, read [`docs/API.md`](docs/API.md).**

## Crate Structure

```
crates/
├── unison2d/        # Core crate (World, Engine, Game trait, Level trait)
├── unison-math/     # Shared Vec2, Color, Rect types
├── unison-physics/  # XPBD soft body & rigid body physics
├── unison-render/   # Rendering abstractions
├── unison-lighting/ # 2D dynamic lighting
├── unison-profiler/ # Profiling utilities
├── unison-input/    # Two-layer input (raw + actions)
├── unison-assets/   # Build-time asset embedding & runtime store
└── unison-web/      # Web platform (WebGL2, DOM input, rAF loop)
```

## Architecture

```
Game (your struct, implements Game trait)
├── Engine<A>        — input/actions, renderer access, compositing
├── World            — self-contained simulation
│   ├── ObjectSystem   — physics world + object registry
│   ├── CameraSystem   — named cameras + follow targets
│   └── LightingSystem — dynamic lights & shadows
└── Level (trait)    — optional scene abstraction
```

- **Engine** is a thin shell — only input mapping, renderer access, asset loading, and compositing
- **World** owns all simulation — games create and manage their own World(s)
- **Level** is an optional trait for organizing self-contained scenes

## Crates

**unison2d** — Core crate. Provides `World`, `Engine<A>`, `Game` trait, and `Level` trait. World composes `ObjectSystem`, `CameraSystem`, and `LightingSystem`. Re-exports all subsystems under `unison2d::{math, physics, render, lighting, profiler, input, assets}`. Key types: `World`, `ObjectSystem`, `CameraSystem`, `Engine<A>`, `Game`, `Level`, `ObjectId`, `SoftBodyDesc`, `RigidBodyDesc`.

**unison-input** — Two-layer input system. Layer 1: `InputState` (raw keyboard/mouse/touch). Layer 2: `ActionMap<A>` (maps raw inputs to game-defined actions). Key types: `KeyCode`, `MouseButton`, `Touch`, `TouchPhase`. Depends on: `unison-math`.

**unison-assets** — Build-time asset embedding and runtime asset store. A `build.rs` helper walks an asset directory, gzip-compresses each file, and generates Rust source with `include_bytes!`. At runtime, `AssetStore` decompresses and serves assets by relative path. Key types: `AssetStore`, `EmbeddedAsset`. Key build function: `build::embed_assets(dir, output)`. Depends on: `flate2`.

**unison-web** — Web platform crate. WebGL2 `Renderer` implementation with render target (FBO) support, DOM event wiring, `requestAnimationFrame` game loop. Entry point: `unison_web::run(game)`. Depends on: `unison-render`, `unison-input`, `unison-math`, `unison-profiler`, `unison2d`, `web-sys`, `wasm-bindgen`.

**unison-math** — Shared types used across all engine crates. Key types: `Vec2`, `Color`, `Rect`. All types provide `From` conversions for arrays and tuples. Zero dependencies.

**unison-physics** — XPBD soft body simulation. Key types: `PhysicsWorld`, `BodyHandle`, `BodyConfig`, `Material`, `CollisionGroups`. Also has rigid bodies, mesh generation, and simulation tracing. Depends on: `unison-math`, `unison-profiler`.

**unison-render** — Platform-agnostic rendering traits. Key types: `Renderer` (trait), `RenderCommand`, `RenderTargetId`, `Color` (re-exported from unison-math), `TextureId`, `TextureDescriptor`, `Sprite`, `SpriteSheet`, `Camera`. Key function: `decode_image(bytes)` — decodes PNG/JPEG/GIF/BMP/WebP into a `TextureDescriptor`. Depends on: `unison-math`, `image`.

**unison-lighting** — 2D dynamic lighting with soft shadows. Uses `Vec2`, `Color`, and `Rect` from unison-math. Key types: `LightingSystem`, `Light`, `LightType` (Point/Spot/Directional/Area), `ShadowMap`, `ShadowCaster`, `ShadowQuality`, `LightingRenderer` (trait). Depends on: `unison-math`, `unison-profiler`.

**unison-profiler** — Lightweight function-level profiling. Key API: `profile_scope!("name")` macro, `Profiler::get_stats()`, `Profiler::format_stats()`. Hierarchical scoping, FPS budget tracking. Zero dependencies.

## Documentation

Engine docs live in `docs/`. Start with [INDEX.md](docs/INDEX.md) for the full table of contents.

```
docs/
├── INDEX.md              # Table of contents — start here
├── API.md                # Single-file API reference (types + methods)
├── api/                  # Per-crate deep dives
│   ├── engine.md         # Core: World, Engine, Game, Level
│   ├── input.md          # Input system
│   ├── physics.md        # XPBD physics
│   ├── render.md         # Rendering traits
│   ├── lighting.md       # Dynamic lighting
│   ├── math.md           # Vec2, Color, Rect
│   ├── web.md            # Web platform
│   ├── profiler.md       # Profiling
│   └── assets.md         # Asset embedding & store
└── guide/                # User guide — patterns and best practices
    ├── README.md         # Guide overview
    ├── getting-started.md
    ├── levels.md
    ├── prefabs.md
    └── patterns.md
```

- **Read** the relevant doc before working with a crate
- **Update** `docs/api/*.md` when you change a crate's public API (add, remove, or modify types/methods/traits)
- **Update** `docs/guide/*.md` when you change a common pattern or best practice
- **Update** `docs/API.md` when you add or change top-level engine types

## Rules

- Code must compile before a task is considered complete
- All tests must pass
- Commit frequently in small logical units
- Commit format: `[PREFIX]: Description` — PREFIX is one of: `CHORE`, `FIX`, `MINOR`, `MAJOR`, `FEAT`
