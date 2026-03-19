# Unison 2D

A Rust game engine built for the LLM agent era. No GUIs — everything is controlled through code and configuration. Platform-agnostic: compile for Web, iOS, and Android from the same codebase.

**For the complete engine API, read [`docs/API.md`](docs/API.md).**

## Crate Structure

```
crates/
├── unison2d/        # Core crate (Engine, Game trait, re-exports)
├── unison-math/     # Shared Vec2, Color, Rect types
├── unison-physics/  # XPBD soft body & rigid body physics
├── unison-render/   # Rendering abstractions
├── unison-lighting/ # 2D dynamic lighting
├── unison-profiler/ # Profiling utilities
├── unison-input/    # Two-layer input (raw + actions)
└── unison-web/      # Web platform (WebGL2, DOM input, rAF loop)
```

## Crates

**unison2d** — Core crate. Provides the `Engine` struct and `Game` trait (batteries-included API). Re-exports all subsystems under `unison2d::{math, physics, render, lighting, profiler, input}`. Key types: `Engine<A>`, `Game`, `ObjectId`, `SoftBodyDesc`, `RigidBodyDesc`.

**unison-input** — Two-layer input system. Layer 1: `InputState` (raw keyboard/mouse/touch). Layer 2: `ActionMap<A>` (maps raw inputs to game-defined actions). Key types: `KeyCode`, `MouseButton`, `Touch`, `TouchPhase`. Depends on: `unison-math`.

**unison-web** — Web platform crate. WebGL2 `Renderer` implementation, DOM event wiring, `requestAnimationFrame` game loop. Entry point: `unison_web::run(game)`. Depends on: `unison-render`, `unison-input`, `unison-math`, `unison-profiler`, `unison2d`, `web-sys`, `wasm-bindgen`.

**unison-math** — Shared types used across all engine crates. Key types: `Vec2`, `Color`, `Rect`. All types provide `From` conversions for arrays and tuples. Zero dependencies.

**unison-physics** — XPBD soft body simulation. Key types: `PhysicsWorld`, `BodyHandle`, `BodyConfig`, `Material`, `CollisionGroups`. Also has rigid bodies, mesh generation, and simulation tracing. Depends on: `unison-math`, `unison-profiler`.

**unison-render** — Platform-agnostic rendering traits. Key types: `Renderer` (trait), `RenderCommand`, `Color` (re-exported from unison-math), `TextureId`, `TextureDescriptor`, `Sprite`, `SpriteSheet`, `Camera`. Depends on: `unison-math`.

**unison-lighting** — 2D dynamic lighting with soft shadows. Uses `Vec2`, `Color`, and `Rect` from unison-math. Key types: `LightingManager`, `Light`, `LightType` (Point/Spot/Directional/Area), `ShadowMap`, `ShadowCaster`, `ShadowQuality`, `LightingRenderer` (trait). Depends on: `unison-math`, `unison-profiler`, `serde`.

**unison-profiler** — Lightweight function-level profiling. Key API: `profile_scope!("name")` macro, `Profiler::get_stats()`, `Profiler::format_stats()`. Hierarchical scoping, FPS budget tracking. Zero dependencies.

## Documentation

Engine docs live in `docs/`. Start with [API.md](docs/API.md) for the complete API reference, or [INDEX.md](docs/INDEX.md) to find per-crate docs.

- **Read** the relevant doc before working with a crate
- **Update** the doc whenever you change a crate's public API (add, remove, or modify types/methods/traits)
- One doc per crate: `math.md`, `physics.md`, `render.md`, `lighting.md`, `profiler.md`, `input.md`, `web.md`, `engine.md`

## Rules

- Code must compile before a task is considered complete
- All tests must pass
- Commit frequently in small logical units
- Commit format: `[PREFIX]: Description` — PREFIX is one of: `CHORE`, `FIX`, `MINOR`, `MAJOR`, `FEAT`