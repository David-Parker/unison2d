# Unison 2D Engine — Documentation

## User Guide

Start here to learn patterns and best practices for building games.

| Guide | What you'll learn |
|-------|-------------------|
| [Getting Started](guide/getting-started.md) | Minimal game, project setup, first soft body on screen |
| [Levels](guide/levels.md) | Level trait, shared state, events, transitions, lifecycle hooks |
| [Prefabs & Shared Code](guide/prefabs.md) | Reusable spawn templates, shared helpers across levels |
| [Patterns](guide/patterns.md) | Platformer movement, spawning, cameras, PiP, despawning |

## API Reference

- [**API.md**](API.md) — single-file reference for all engine types and methods

Per-crate deep dives:

| Crate | Description | Doc |
|-------|-------------|-----|
| `unison2d` | Core crate: World, Engine, Game trait, Level trait | [api/engine.md](api/engine.md) |
| `unison-physics` | XPBD soft body & rigid body physics | [api/physics.md](api/physics.md) |
| `unison-render` | Platform-agnostic rendering traits | [api/render.md](api/render.md) |
| `unison-lighting` | 2D lighting with lightmap compositing | [api/lighting.md](api/lighting.md) |
| `unison-input` | Two-layer input (raw state + action mapping) | [api/input.md](api/input.md) |
| `unison-core` | Shared Vec2, Color, Rect types | [api/math.md](api/math.md) |
| `unison-assets` | Build-time asset embedding & runtime store | [api/assets.md](api/assets.md) |
| `unison-ui` | Declarative React-like UI system (HUDs, menus, buttons) | [api/ui.md](api/ui.md) |
| `unison-web` | Web platform (WebGL2, DOM input, rAF loop) | [api/web.md](api/web.md) |
| `unison-ios` | iOS platform (Metal renderer, touch input, frame loop) | [api/ios.md](api/ios.md) |
| `unison-profiler` | Function-level profiling | [api/profiler.md](api/profiler.md) |

## Integration Tests

`crates/unison-tests/` contains headless e2e and simulation tests. Unit tests stay in their respective crates; multi-frame simulation, regression, and stress tests live here.

```bash
cargo test -p unison-tests          # Run integration tests only
cargo test --workspace              # Run everything
```

## Plans

| Plan | Status |
|------|--------|
| [2D Lighting System](plans/lighting.md) | Phase 1–3 complete (Point Lights, Directional Lights, Shadow Casting) — Phase 4 future (Normal Maps) |

## Rules

- **Read** the relevant doc before working with a crate
- **Update** the doc whenever you change a crate's public API
- **Update** the guide whenever you change a common pattern or best practice
