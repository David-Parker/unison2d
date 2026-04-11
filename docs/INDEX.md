# Unison 2D Engine — Documentation

Game code is written in **Lua** via the `unison-scripting` crate. The Rust `Game` trait remains available for advanced use cases (engine work, custom rendering paths), but Lua is the canonical path for writing a game.

## Scripting (Lua) — canonical game code

| Guide | What you'll learn |
|-------|-------------------|
| [Getting Started](scripting/getting-started.md) | Setup, script lifecycle, minimal example, multi-file require |
| [API Reference](scripting/api-reference.md) | All Lua globals: `engine`, `input`, `events`, `World`, `Color`, `Rng`, `math`, `debug` |
| [Scenes](guide/scripting-scenes.md) | Lua scene system — lifecycle hooks, switching, multi-level example |
| [Migration Guide](scripting/migration-guide.md) | Port a Rust game to Lua — before/after examples, gotchas |
| [Hot Reload](scripting/hot-reload.md) | Level 1 vs Level 2 reload, ScriptWatcher, web strategy, error overlay |

## User Guide (Rust, advanced)

Patterns and best practices for building games directly against the Rust `Game` / `Level` traits. For most games, prefer the Lua scripting path above.

| Guide | What you'll learn |
|-------|-------------------|
| [Getting Started](guide/getting-started.md) | Minimal Rust game, project setup, first soft body on screen |
| [Levels](guide/levels.md) | Level trait, shared state, events, transitions, lifecycle hooks |
| [Prefabs & Shared Code](guide/prefabs.md) | Reusable spawn templates, shared helpers across levels |
| [Patterns](guide/patterns.md) | Platformer movement, spawning, cameras, PiP, despawning |

## API Reference

- [**API.md**](API.md) — single-file reference for all Rust engine types and methods

Per-crate deep dives:

| Crate | Description | Doc |
|-------|-------------|-----|
| `unison2d` | Core crate: World, Engine, Game trait, Level trait | [api/engine.md](api/engine.md) |
| `unison-physics` | XPBD soft body & rigid body physics | [api/physics.md](api/physics.md) |
| `unison-render` | Platform-agnostic rendering traits, textures, sprites | [api/render.md](api/render.md) |
| `unison-lighting` | 2D lighting with lightmap compositing | [api/lighting.md](api/lighting.md) |
| `unison-input` | Two-layer input (raw state + action mapping) | [api/input.md](api/input.md) |
| `unison-core` | Shared Vec2, Color, Rect types | [api/math.md](api/math.md) |
| `unison-assets` | Build-time asset embedding & runtime store | [api/assets.md](api/assets.md) |
| `unison-ui` | Declarative React-like UI system (HUDs, menus, buttons) | [api/ui.md](api/ui.md) |
| `unison-scripting` | Lua 5.4 scripting — `ScriptedGame` implementing `Game` trait | [api/scripting.md](api/scripting.md) |
| `unison-web` | Web platform (WebGL2, DOM input, rAF loop) | [api/web.md](api/web.md) |
| `unison-ios` | iOS platform (Metal renderer, touch input, frame loop) | [api/ios.md](api/ios.md) |
| `unison-android` | Android platform (GLES 3.0 renderer, touch input, JNI loop) | [api/android.md](api/android.md) |
| `unison-profiler` | Function-level profiling | [api/profiler.md](api/profiler.md) |

## Integration Tests

`crates/unison-tests/` contains headless e2e and simulation tests. Unit tests stay in their respective crates; multi-frame simulation, regression, and stress tests live here.

```bash
cargo test -p unison-tests          # Run integration tests only
cargo test --workspace              # Run everything
```

## Rules

- **Read** the relevant doc before working with a crate
- **Update** the doc whenever you change a crate's public API
- **Update** the guide whenever you change a common pattern or best practice
