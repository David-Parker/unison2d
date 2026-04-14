# Unison 2D Engine — Documentation

Game code is written in **Lua** or **TypeScript** (transpiled to Lua at build time via TSTL). `unison-scripting::ScriptedGame` is the sole game-entry implementation; all gameplay authoring goes through the `unison.*` Lua namespace.

## Scripting (Lua + TypeScript) — canonical game code

| Guide | What you'll learn |
|-------|-------------------|
| [Overview](scripting/README.md) | Pick your language — comparison table and links |
| [Getting Started (Lua)](scripting/getting-started/lua.md) | Setup, script lifecycle, minimal example, multi-file require |
| [Getting Started (TypeScript)](scripting/getting-started/typescript.md) | TSTL setup, type declarations, build workflow |
| [Concepts](scripting/concepts.md) | Language-neutral: lifecycle, scenes, events, worlds |
| [API Reference](scripting/api-reference.md) | All globals with Lua + TypeScript signatures side-by-side |
| [Hot Reload](scripting/hot-reload.md) | Level 1 vs Level 2 reload, ScriptWatcher, TSTL watch, web strategy |

## API Reference

Per-crate deep dives:

| Crate | Description | Doc |
|-------|-------------|-----|
| `unison2d` | Core crate: World, Engine, Game trait (internal plumbing) | [api/engine.md](api/engine.md) |
| `unison-physics` | XPBD soft body & rigid body physics | [api/physics.md](api/physics.md) |
| `unison-render` | Platform-agnostic rendering traits, textures, sprites | [api/render.md](api/render.md) |
| `unison-lighting` | 2D lighting with lightmap compositing | [api/lighting.md](api/lighting.md) |
| `unison-input` | Raw input state + Lua action map | [api/input.md](api/input.md) |
| `unison-core` | Shared Vec2, Color, Rect types | [api/math.md](api/math.md) |
| `unison-assets` | Build-time asset embedding & runtime store | [api/assets.md](api/assets.md) |
| `unison-ui` | Declarative React-like UI system (HUDs, menus, buttons) | [api/ui.md](api/ui.md) |
| `unison-scripting` | Lua 5.4 scripting — `ScriptedGame` implementing `Game` trait | [api/scripting.md](api/scripting.md) |
| `unison-cli` | `unison` CLI — scaffold, build, dev, test, link, doctor | [../crates/unison-cli/README.md](../crates/unison-cli/README.md) |
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
