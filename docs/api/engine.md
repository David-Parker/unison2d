# unison2d crate — Engine core

## Overview

`unison2d` is the core engine crate. It defines `World`, `Engine`, and the
`Game` trait. For game code written in Lua or TypeScript, these are internal
plumbing — you do not implement `Game` yourself. See
[docs/scripting/api-reference.md](../scripting/api-reference.md) for the
game-facing `unison.*` API.

## The Game trait (internal)

Only `unison_scripting::ScriptedGame` implements this in shipping code.
Platform crates call `init` once, then drive the loop by calling `update` and
`render` each frame.

```rust
pub trait Game: 'static {
    /// Called once after the engine is initialized.
    fn init(&mut self, engine: &mut Engine);

    /// Called once per fixed timestep tick (60 Hz).
    fn update(&mut self, engine: &mut Engine);

    /// Called once per frame for rendering.
    fn render(&mut self, engine: &mut Engine);
}
```

`ScriptedGame` bridges this lifecycle to Lua: `init` runs the entry script,
`update` calls the Lua `game.update(dt)` (or active scene's `update`),
`render` calls `game.render()` (or scene's `render`).

## The Engine struct (internal)

`Engine` is a thin shell. It does not own a world — games create and manage
their own `World`(s).

Fields exposed via methods:

| Method | Description |
|--------|-------------|
| `engine.input_state()` | Raw `InputState` snapshot for the current tick |
| `engine.renderer_mut()` | `Option<&mut dyn Renderer>` — platform renderer |
| `engine.dt()` | Fixed timestep delta (1/60 s) |
| `engine.assets()` | Read-only `AssetStore` |
| `engine.assets_mut()` | Mutable `AssetStore` (for loading embedded assets) |
| `engine.load_texture(path)` | Decode + upload a texture in one step (called internally by `unison.assets.load_texture`) |
| `engine.set_anti_aliasing(mode)` | Set MSAA mode for this session |
| `engine.anti_aliasing()` | Current AA mode |
| `engine.create_render_target(w, h)` | Create an offscreen FBO; returns `(RenderTargetId, TextureId)` |
| `engine.destroy_render_target(id)` | Destroy an FBO (keeps its texture) |

## World (user-facing via Lua)

`World` is the simulation container. Create one with `unison.World.new()` in
Lua, or `World::new()` in Rust.

It owns three subsystems:

- **`world.objects`** — `ObjectSystem`: physics objects, rigid bodies, soft
  bodies, sprites
- **`world.cameras`** — `CameraSystem`: named cameras and follow targets
- **`world.lights`** — `LightingSystem`: point lights, directional lights,
  ambient, and lightmap compositing

Key `World` methods (Lua colon-syntax):

| Method | Description |
|--------|-------------|
| `world:set_background(hex)` | Clear color as a hex integer |
| `world:set_gravity(g)` | Gravity strength (negative = downward) |
| `world:set_ground(y)` | Flat ground plane at world Y |
| `world:set_ground_restitution(r)` | Ground bounciness 0–1 |
| `world:set_ground_friction(f)` | Ground friction 0–1 |
| `world:step(dt)` | Advance physics + camera/light follows |
| `world:render()` | Render all objects through the main camera |
| `world:render_to_targets(mapping)` | Multi-camera render to offscreen targets |
| `world:draw_overlay(tex, x, y, w, h)` | Composite a render target onto the screen |
| `world:draw(shape, params, z)` | Draw a shape to the default lit layer |
| `world:draw_unlit(shape, params, z)` | Draw a shape bypassing the lightmap |
| `world:create_render_layer(name, opts)` | Add a named render layer; returns `RenderLayerId` |
| `world:on_collision(cb)` | Register a callback for every collision pair |
| `world:on_collision_with(id, cb)` | Callback when a specific object collides with anything |
| `world:on_collision_between(a, b, cb)` | Callback when objects `a` and `b` collide |

For a complete listing of `world.objects`, `world.cameras`, and `world.lights`
methods, see [docs/scripting/api-reference.md](../scripting/api-reference.md).

## Subsystem re-exports

`unison2d` re-exports all subsystem crates at their canonical names:

```rust
pub use unison_core as core;
pub use unison_physics as physics;
pub use unison_render as render;
pub use unison_profiler as profiler;
pub use unison_input as input;
pub use unison_assets as assets;
pub use unison_lighting as lighting;
pub use unison_ui as ui;
```
