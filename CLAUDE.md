# Unison 2D

A Rust game engine built for the LLM agent era. No GUIs — everything is controlled through code and configuration. Platform-agnostic: compile for Web, iOS, and Android from the same codebase.

Game code is written in **Lua** using `unison-scripting` (`ScriptedGame`). The Rust
`Game` trait is available for advanced use cases, but scripting is the canonical
approach.

## Architecture

```
ScriptedGame (Lua VM, implements Game trait)  ← canonical for game code
├── engine global  — texture loading, screen size, scenes, UI, AA
├── input global   — raw key/touch state
├── events global  — string-keyed pub/sub + collision callbacks
└── World global   — self-contained simulation
    ├── ObjectSystem   — physics world + object registry
    ├── CameraSystem   — named cameras + follow targets
    └── LightingSystem — point lights, directional lights, + lightmap compositing
```

**TypeScript support:** TypeScript is an optional authoring layer. The engine
runtime and `unison-scripting` crate are Lua-only. Transpilation (via TSTL)
happens in the consumer project's build pipeline, not in the engine. Type
declarations live at `crates/unison-scripting/types/`.

Underlying Rust layer (used by `ScriptedGame` internally, available for advanced use):

```
Game trait (implement directly for Rust game code)
├── Engine<A>      — input/actions, renderer access, compositing
├── World          — self-contained simulation
└── Level (trait)  — optional scene abstraction
```

- **Engine** is a thin shell — only input mapping, renderer access, asset loading, and compositing
- **World** owns all simulation — games create and manage their own World(s)
- **Level** is an optional trait for organizing self-contained scenes in Rust

## Crates & Docs

Each crate has a per-crate deep dive in `docs/api/`. Read the relevant doc before working with a crate; update it when you change public API.

| Crate | Doc | Description |
|-------|-----|-------------|
| `unison2d` | [engine.md](docs/api/engine.md) | Core: World, Engine, Game trait, Level trait |
| `unison-physics` | [physics.md](docs/api/physics.md) | XPBD soft body & rigid body physics |
| `unison-render` | [render.md](docs/api/render.md) | Platform-agnostic rendering traits, textures, sprites |
| `unison-lighting` | [lighting.md](docs/api/lighting.md) | 2D lighting with lightmap compositing |
| `unison-input` | [input.md](docs/api/input.md) | Two-layer input (raw state + action mapping) |
| `unison-ui` | [ui.md](docs/api/ui.md) | Declarative React-like UI system (HUDs, menus, buttons) |
| `unison-core` | [math.md](docs/api/math.md) | Shared Vec2, Color, Rect, EventSink types |
| `unison-assets` | [assets.md](docs/api/assets.md) | Build-time asset embedding & runtime store |
| `unison-web` | [web.md](docs/api/web.md) | Web platform (WebGL2, DOM input, rAF loop) |
| `unison-ios` | [ios.md](docs/api/ios.md) | iOS platform (Metal renderer, touch input, frame loop) |
| `unison-android` | [android.md](docs/api/android.md) | Android platform (GLES 3.0 renderer, touch input, JNI frame loop) |
| `unison-profiler` | [profiler.md](docs/api/profiler.md) | Function-level profiling |
| `unison-scripting` | [scripting.md](docs/api/scripting.md) | Lua 5.4 scripting — ScriptedGame implementing Game trait |
| `unison-lua` | [../crates/unison-lua/README.md](crates/unison-lua/README.md) | Lua 5.4 interpreter fork (wasm32 support, substituted for `lua-src` via `[patch.crates-io]`) |
| `unison-cli` | [../crates/unison-cli/README.md](crates/unison-cli/README.md) | `unison` CLI — scaffold, build, dev, test, link, doctor |
| `unison-tests` | — | Headless e2e / simulation tests (physics, rendering, etc.) |

All crates are re-exported from `unison2d::{math, physics, render, lighting, profiler, input, assets, ui}`.

## Docs

```
docs/
├── API.md            # Quick reference — all engine types & methods
├── api/              # Per-crate deep dives (linked in table above)
└── scripting/        # Game authoring guides (Lua + TypeScript)
    ├── README.md             # Pick your language
    ├── getting-started/
    │   ├── lua.md            # Lua setup + tutorial
    │   └── typescript.md     # TypeScript setup + tutorial
    ├── concepts.md           # Language-neutral: lifecycle, scenes, events
    ├── api-reference.md      # All globals — Lua + TS side-by-side
    ├── hot-reload.md         # Hot reload for both languages
    └── rationale.md          # Why Lua 5.4, why no LuaJIT, why fork lua-src, why keep mlua
```

**How to navigate:**
- **Starting a new Lua game?** → [scripting/getting-started/lua.md](docs/scripting/getting-started/lua.md)
- **Starting a new TypeScript game?** → [scripting/getting-started/typescript.md](docs/scripting/getting-started/typescript.md)
- **Need a Lua/TS method signature?** → [scripting/api-reference.md](docs/scripting/api-reference.md)
- **Need a type signature or method (Rust)?** → [API.md](docs/API.md)
- **Need to understand a subsystem?** → per-crate doc (table above)

## Rules

- Code must compile before a task is considered complete
- All tests must pass
- Commit frequently in small logical units
- Commit format: `[PREFIX]: Description` — PREFIX is one of: `CHORE`, `FIX`, `MINOR`, `MAJOR`, `FEAT`
