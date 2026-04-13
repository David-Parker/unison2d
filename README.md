<p align="center">
  <img src="unison2d.png" alt="Unison 2D" width="400">
</p>

<h1 align="center">Unison 2D</h1>

<p align="center">
  <strong>A Rust 2D game engine built for the LLM agent era.</strong><br>
  No GUIs. No editors. Just code.
</p>

<p align="center">
  <a href="https://github.com/David-Parker/unison2d/actions"><img src="https://img.shields.io/github/actions/workflow/status/David-Parker/unison2d/ci.yml?style=flat-square&logo=github&label=CI" alt="CI"></a>
  <a href="https://github.com/David-Parker/unison2d/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="License: MIT"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-2021_edition-orange?style=flat-square&logo=rust" alt="Rust 2021"></a>
  <a href="#"><img src="https://img.shields.io/badge/platforms-Web_·_iOS_·_Android-8A2BE2?style=flat-square" alt="Platforms"></a>
  <a href="https://webassembly.org/"><img src="https://img.shields.io/badge/wasm-ready-654FF0?style=flat-square&logo=webassembly&logoColor=white" alt="WASM Ready"></a>
</p>

---

## Why Unison?

Traditional game engines are designed for humans clicking through editor panels. **Unison 2D is designed for agents writing code.** Every feature is accessible through a clean API — no project files, no drag-and-drop, no hidden state.

Game code is written in **Lua 5.4** via the `unison-scripting` crate, which implements the engine's `Game` trait on top of an embedded Lua VM. The Rust API is still first-class and available for engine work or advanced use cases, but Lua is the canonical way to write a game: fast iteration, hot reload, and no Rust recompile loop. Point your agent at the docs and start building games for every platform.

- **Lua scripting** — Canonical game code path; Rust API available for advanced use
- **Code-first** — Everything is controlled through code and configuration
- **Modular** — Use the full engine or pick individual crates
- **Cross-platform** — Compile for Web, iOS, and Android from one codebase
- **SIMD-accelerated physics** — XPBD soft body & rigid body simulation
- **Dynamic lighting** — 2D lights with real-time soft shadows
- **Declarative UI** — React-like UI system with diffing, layout, and input handling
- **TypeScript support** — Write games in TypeScript with full type safety and IDE tooling. TSTL transpiles to Lua at build time; the engine runtime stays Lua-only. See `crates/unison-scripting/types/` for declarations and `samples/` for working examples.
- **Hot reload** — Edit Lua, save, see changes without a rebuild
- **Zero-cost profiling** — Hierarchical function-level profiling behind a feature gate

## Crates

```
unison2d/crates/
├── unison2d/         # Core engine — World, Engine, Game trait, re-exports
├── unison-core/      # Vec2, Color, Rect — zero dependencies
├── unison-physics/   # XPBD soft body & rigid body physics
├── unison-render/    # Platform-agnostic rendering traits, textures, sprites
├── unison-lighting/  # 2D lighting with lightmap compositing and shadows
├── unison-input/     # Two-layer input (raw → action mapping)
├── unison-ui/        # Declarative UI (menus, HUDs, buttons, text)
├── unison-assets/    # Build-time asset embedding & runtime store
├── unison-profiler/  # Function-level profiling
├── unison-scripting/ # Lua 5.4 scripting — ScriptedGame implementing Game trait
├── unison-web/       # Web platform (WebGL2, DOM input, rAF loop)
├── unison-ios/       # iOS platform (Metal renderer, touch input, frame loop)
├── unison-android/   # Android platform (GLES 3.0 renderer, touch input, JNI loop)
├── unison-cli/       # `unison` CLI — scaffold, build, dev, test, link, doctor
└── unison-tests/     # Headless e2e / simulation tests
```

All subsystems are independent. Use `unison2d` to get everything, or depend on individual crates.

## Quick Start

Install the CLI and scaffold a new project:

```bash
cargo install --git https://github.com/David-Parker/unison2d unison-cli
unison new my-game
cd my-game
unison doctor     # check your toolchain
unison dev web    # run locally
```

See [`crates/unison-cli/README.md`](crates/unison-cli/README.md) for the full command reference.

---

For an existing project, or if you'd rather wire things up by hand, the raw layout looks like this:

### Lua game (canonical)

A Lua game's entire Rust-side scaffold is two lines of `lib.rs` — the `scripted_game_entry!` macro emits the web, iOS, and Android FFI glue for you. Everything else lives in `.lua` files under your asset dir.

```toml
[dependencies]
unison-scripting = { path = "unison2d/crates/unison-scripting", features = ["simd"] }
# wasm-bindgen must be a direct dep of the game crate because
# `#[wasm_bindgen(start)]` emits absolute `::wasm_bindgen` paths.
wasm-bindgen = { version = "0.2", optional = true }

[build-dependencies]
unison-assets = { path = "unison2d/crates/unison-assets", features = ["build"] }

[features]
default = ["web"]
web     = ["unison-scripting/web", "dep:wasm-bindgen"]
ios     = ["unison-scripting/ios"]
android = ["unison-scripting/android"]
```

```rust
// lib.rs — your entire platform scaffold
mod assets { include!(concat!(env!("OUT_DIR"), "/assets.rs")); }

unison_scripting::scripted_game_entry!("scripts/main.lua", assets::ASSETS);
```

```lua
-- scripts/main.lua
local game = {}

function game.init()
    world = World.new()
    world:set_background(0x1a1a2e)
    world:set_gravity(-9.8)
    world:set_ground(-4.5)
    box_id = world:spawn_rigid_body({
        collider = "aabb", half_width = 0.6, half_height = 0.6,
        position = {0, 2}, color = 0xe74c3c,
    })
    world:camera_follow("main", box_id, 0.1)
end

function game.update(dt) world:step(dt) end
function game.render()   world:auto_render() end

return game
```

See [docs/scripting/getting-started/lua.md](docs/scripting/getting-started/lua.md) for the full walkthrough.

### TypeScript Game

TypeScript games transpile to Lua via [TypeScriptToLua](https://typescripttolua.github.io/).
See `samples/ts-minimal/` for a working example and `docs/scripting/getting-started/typescript.md` for the full setup guide.

Type declarations: `crates/unison-scripting/types/`

### Rust game (advanced)

For engine work, custom rendering, or performance-sensitive code paths, implement the `Game` trait in Rust directly:

```rust
use unison2d::physics::{Material, mesh::create_ring_mesh};
use unison2d::render::Color;
use unison2d::math::Vec2;
```

See [docs/API.md](docs/API.md) for the Rust type and method reference.

### Feature Flags

| Flag | Effect |
|------|--------|
| `simd` | SIMD-accelerated physics |
| `profiling` | Enable hierarchical profiling across all crates |

```toml
unison2d = { path = "unison2d/crates/unison2d", features = ["simd", "profiling"] }
```

## Documentation

- [**Lua Getting Started**](docs/scripting/getting-started/lua.md) — setup, lifecycle, minimal example, hot reload
- [**TypeScript Getting Started**](docs/scripting/getting-started/typescript.md) — TSTL setup, type declarations, workflow
- [**Concepts**](docs/scripting/concepts.md) — language-neutral: lifecycle, scenes, events
- [**API Reference**](docs/scripting/api-reference.md) — all globals (Lua + TypeScript side-by-side)
- [**Per-Crate Docs**](docs/api/) — deep dives into each subsystem

## License

[MIT](LICENSE)
