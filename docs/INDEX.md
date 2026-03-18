# Unison 2D Engine — Documentation Index

Read the relevant doc before working with a crate. Update docs when changing public APIs.

| Crate | Description | Doc |
|-------|-------------|-----|
| `unison-math` | Shared Vec2, Color, Rect types | [math.md](math.md) |
| `unison-physics` | XPBD soft body & rigid body physics | [physics.md](physics.md) |
| `unison-render` | Platform-agnostic rendering traits | [render.md](render.md) |
| `unison-lighting` | 2D dynamic lighting & shadows | [lighting.md](lighting.md) |
| `unison-profiler` | Function-level profiling | [profiler.md](profiler.md) |

The `unison2d` crate re-exports all of the above as `unison2d::{math, physics, render, lighting, profiler}`.
