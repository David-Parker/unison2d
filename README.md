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

Traditional game engines are designed for humans clicking through editor panels. **Unison 2D is designed for agents writing code.** Every feature is accessible through a clean Rust API — no project files, no drag-and-drop, no hidden state.

Unison is composed of several crates containing low level sub-systems. A high level API unifies them together. There is no complex ECS, no DSL for your agent to learn, no scripting system. Point your agent to the docs and start building games for every platform.

- **Code-first** — Everything is controlled through code and configuration
- **Modular** — Use the full engine or pick individual crates
- **Cross-platform** — Compile for Web, iOS, and Android from one codebase
- **SIMD-accelerated physics** — XPBD soft body & rigid body simulation
- **Dynamic lighting** — 2D lights with real-time soft shadows
- **Declarative UI** — React-like UI system with diffing, layout, and input handling
- **Zero-cost profiling** — Hierarchical function-level profiling behind a feature gate

## Crates

```
unison2d/crates/
├── unison2d/        # Core engine — Game trait, re-exports everything
├── unison-math/     # Vec2, Color, Rect — zero dependencies
├── unison-physics/  # XPBD soft body & rigid body physics
├── unison-render/   # Platform-agnostic rendering traits
├── unison-lighting/ # 2D lighting with lightmap compositing and shadows
├── unison-input/    # Two-layer input (raw → action mapping)
├── unison-ui/       # Declarative UI (menus, HUDs, buttons, text)
├── unison-assets/   # Build-time asset embedding & runtime store
├── unison-profiler/ # Function-level profiling
├── unison-web/      # Web platform (WebGL2, DOM input, rAF loop)
├── unison-ios/      # iOS platform (Metal renderer, touch input, frame loop)
└── unison-tests/    # Headless e2e / simulation tests
```

All subsystems are independent. Use `unison2d` to get everything, or depend on individual crates.

## Quick Start

Add as a git submodule:

```bash
git submodule add https://github.com/David-Parker/unison2d.git
```

```toml
[dependencies]
unison2d = { path = "unison2d/crates/unison2d" }
```

Then access subsystems:

```rust
use unison2d::physics::{PhysicsWorld, BodyConfig, Material, Mesh};
use unison2d::render::{Renderer, Camera, Color};
use unison2d::lighting::{LightingSystem, PointLight, DirectionalLight, ShadowSettings};
use unison2d::input::{InputState, KeyCode};
use unison2d::ui::facade::Ui;
use unison2d::profiler::{Profiler, profile_scope};
use unison2d::math::Vec2;
```

### Feature Flags

| Flag | Effect |
|------|--------|
| `simd` | SIMD-accelerated physics |
| `profiling` | Enable hierarchical profiling across all crates |

```toml
unison2d = { path = "unison2d/crates/unison2d", features = ["simd", "profiling"] }
```

## Documentation

- [**User Guide**](docs/guide/README.md) — patterns, best practices, getting started
- [**API Reference**](docs/API.md) — single-file type and method reference
- [**Per-Crate Docs**](docs/INDEX.md) — deep dives into each subsystem

## License

[MIT](LICENSE)
