<p align="center">
  <img src="unison2d.png" alt="Unison 2D" width="400">
</p>

# Unison 2D

A Rust 2D game engine built for the LLM agent era. No GUIs — everything is controlled through code and configuration. Platform-agnostic: compile for Web, iOS, and Android from the same codebase.

## Crates

```
unison2d/crates/
├── unison2d/       # Core crate (re-exports all subsystems)
├── unison-physics/ # XPBD soft body & rigid body physics
├── unison-render/  # Platform-agnostic rendering traits
├── unison-lighting/# 2D dynamic lighting & shadows
└── unison-profiler/# Function-level profiling
```

All subsystems are independent. Use `unison2d` to get everything, or depend on individual crates.

## Usage

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
use unison2d::lighting::{LightingManager, Light};
use unison2d::profiler::{Profiler, profile_scope};
```

## Documentation

See [docs/INDEX.md](docs/INDEX.md) for per-crate API documentation.

## License

MIT
