# Unison 2D

A Rust game engine built for the LLM agent era. No GUIs — everything is controlled through code and configuration. Platform-agnostic: compile for Web, iOS, and Android from the same codebase.

## Architecture

```
Game (your struct, implements Game trait)
├── Engine<A>        — input/actions, renderer access, compositing
├── World            — self-contained simulation
│   ├── ObjectSystem   — physics world + object registry
│   ├── CameraSystem   — named cameras + follow targets
│   └── LightingSystem — point lights, directional lights, + lightmap compositing
└── Level (trait)    — optional scene abstraction
```

- **Engine** is a thin shell — only input mapping, renderer access, asset loading, and compositing
- **World** owns all simulation — games create and manage their own World(s)
- **Level** is an optional trait for organizing self-contained scenes

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
| `unison-core` | [math.md](docs/api/math.md) | Shared Vec2, Color, Rect types |
| `unison-assets` | [assets.md](docs/api/assets.md) | Build-time asset embedding & runtime store |
| `unison-web` | [web.md](docs/api/web.md) | Web platform (WebGL2, DOM input, rAF loop) |
| `unison-ios` | [ios.md](docs/api/ios.md) | iOS platform (Metal renderer, touch input, frame loop) |
| `unison-android` | [android.md](docs/api/android.md) | Android platform (GLES 3.0 renderer, touch input, JNI frame loop) |
| `unison-profiler` | [profiler.md](docs/api/profiler.md) | Function-level profiling |
| `unison-tests` | — | Headless e2e / simulation tests (physics, rendering, etc.) |

All crates are re-exported from `unison2d::{math, physics, render, lighting, profiler, input, assets, ui}`.

## Docs

```
docs/
├── API.md            # Quick reference — all engine types & methods in one file
├── api/              # Per-crate deep dives (linked in table above)
└── guide/            # Patterns & best practices
    ├── README.md         # Guide overview — start here for tutorials
    ├── getting-started.md
    ├── levels.md
    ├── prefabs.md
    └── patterns.md
```

**How to navigate:**
- **Need a type signature or method?** → [API.md](docs/API.md)
- **Need to understand a subsystem?** → per-crate doc (table above)
- **Need patterns or how-to?** → [guide/README.md](docs/guide/README.md)

## Rules

- Code must compile before a task is considered complete
- All tests must pass
- Commit frequently in small logical units
- Commit format: `[PREFIX]: Description` — PREFIX is one of: `CHORE`, `FIX`, `MINOR`, `MAJOR`, `FEAT`
