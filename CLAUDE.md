# Unison 2D

A 2D game engine designed for the LLM-agent era. No GUIs — everything is code
and configuration. Platform-agnostic: one codebase compiles to Web, iOS, and
Android. Game code is authored in **Lua** (or TypeScript → Lua via TSTL).
`unison-scripting::ScriptedGame` is the canonical `Game`-trait implementation.

## Architecture

`ScriptedGame` hosts a Lua 5.4 VM and exposes the entire engine surface as a
single `unison.*` global. Platform crates (web, iOS, Android) drive the frame
loop by calling `ScriptedGame`'s `init` / `update` / `render`. The underlying
Rust types (`Game` trait, `World`, `Engine`) are internal plumbing — game code
never touches them directly.

```
unison.*  — the single Lua root namespace
├── assets      — texture loading
├── renderer    — screen size, anti-aliasing, render targets
├── input       — raw keys/mouse/touch + Lua action map
├── scenes      — scene management (set, current)
├── events      — string-keyed pub/sub (on, emit, clear)
├── UI          — declarative UI factory
├── debug       — log, draw_point, show_physics, show_fps
├── math        — lerp, smoothstep, clamp
├── World       — constructor → World instance
│   ├── objects   — spawn, despawn, physics, queries
│   ├── cameras   — named cameras, follow, screen-to-world
│   └── lights    — point/directional lights, ambient
├── Color       — hex / rgba constructors
└── Rng         — seeded deterministic RNG
```

TypeScript is an optional authoring layer. The runtime is Lua-only.
Transpilation via TSTL happens in the consumer project's build pipeline, not in
the engine. Type declarations live at `crates/unison-scripting/types/`.

## Crates & Docs

Each crate has a per-crate deep dive in `docs/api/`. Read the relevant doc
before working with a crate; update it when you change public API.

| Crate | Doc | Description |
|-------|-----|-------------|
| `unison2d` | [engine.md](docs/api/engine.md) | Core: World, Engine, Game trait (internal plumbing) |
| `unison-physics` | [physics.md](docs/api/physics.md) | XPBD soft body & rigid body physics |
| `unison-render` | [render.md](docs/api/render.md) | Platform-agnostic rendering traits, textures, sprites |
| `unison-lighting` | [lighting.md](docs/api/lighting.md) | 2D lighting with lightmap compositing |
| `unison-audio` | [audio.md](docs/api/audio.md) | Cross-platform audio: music, SFX, 2D-spatial, buses, tweens (kira-backed) |
| `unison-input` | [input.md](docs/api/input.md) | Raw input state + Lua action map |
| `unison-ui` | [ui.md](docs/api/ui.md) | Declarative React-like UI system (HUDs, menus, buttons) |
| `unison-core` | [math.md](docs/api/math.md) | Shared Vec2, Color, Rect types |
| `unison-assets` | [assets.md](docs/api/assets.md) | Build-time asset embedding & runtime store |
| `unison-web` | [web.md](docs/api/web.md) | Web platform (WebGL2, DOM input, rAF loop) |
| `unison-ios` | [ios.md](docs/api/ios.md) | iOS platform (Metal renderer, touch input, frame loop) |
| `unison-android` | [android.md](docs/api/android.md) | Android platform (GLES 3.0, touch input, JNI frame loop) |
| `unison-profiler` | [profiler.md](docs/api/profiler.md) | Function-level profiling |
| `unison-scripting` | [scripting.md](docs/api/scripting.md) | Lua 5.4 scripting — ScriptedGame + all `unison.*` bindings |
| `unison-lua` | [../crates/unison-lua/README.md](crates/unison-lua/README.md) | Lua 5.4 interpreter fork (wasm32 support) |
| `unison-cli` | [../crates/unison-cli/README.md](crates/unison-cli/README.md) | `unison` CLI — scaffold, build, dev, test, link, doctor |
| `unison-tests` | — | Headless e2e / simulation tests (physics, rendering, etc.) |

All subsystem crates are re-exported from `unison2d::{core, physics, render,
lighting, profiler, input, assets, ui}`.

## Docs

```
docs/
├── api/              # Per-crate deep dives (linked in table above)
└── scripting/        # Game authoring guides (Lua + TypeScript)
    ├── README.md             # Pick your language
    ├── getting-started/
    │   ├── lua.md            # Lua setup + tutorial
    │   └── typescript.md     # TypeScript setup + tutorial
    ├── concepts.md           # Language-neutral: lifecycle, scenes, events
    ├── api-reference.md      # All unison.* globals — Lua + TS side-by-side
    ├── hot-reload.md         # Hot reload for both languages
    └── rationale.md          # Why Lua 5.4, why fork lua-src, why keep mlua
```

**How to navigate:**
- **Starting a new Lua game?** → [scripting/getting-started/lua.md](docs/scripting/getting-started/lua.md)
- **Starting a new TypeScript game?** → [scripting/getting-started/typescript.md](docs/scripting/getting-started/typescript.md)
- **Need a Lua/TS method signature?** → [scripting/api-reference.md](docs/scripting/api-reference.md)
- **Need to understand a subsystem?** → per-crate doc (table above)

## Rules

- Code must compile before a task is considered complete
- All tests must pass
- Commit frequently in small logical units
- Commit format: `[PREFIX]: Description` — PREFIX is one of: `CHORE`, `FIX`, `MINOR`, `MAJOR`, `FEAT`
