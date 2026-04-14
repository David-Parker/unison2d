# unison-scripting

Lua 5.4 scripting for Unison 2D. Implements the `Game` trait internally, forwarding lifecycle calls into an embedded Lua VM. All game code is written in Lua (or TypeScript transpiled to Lua via TSTL) — Rust authoring is not a supported path for game code.

## Purpose

- Embed a full Lua 5.4 VM in the game binary (vendored C source, no system Lua required)
- Implement `Game` trait so `ScriptedGame` is the drop-in platform entry point
- Expose all engine functionality to Lua via the single `unison.*` global namespace
- Support all three platforms: Web (wasm32), iOS (aarch64-apple-ios), Android

## Key Types

### `ScriptedGame`

```rust
pub struct ScriptedGame { /* ... */ }

impl ScriptedGame {
    pub fn new(script_src: impl Into<String>) -> Self;
    pub fn from_asset(path: impl Into<String>, assets: &'static [EmbeddedAsset]) -> Self;
}

impl Game for ScriptedGame {
    fn init(&mut self, engine: &mut Engine);
    fn update(&mut self, engine: &mut Engine);
    fn render(&mut self, engine: &mut Engine);
}
```

`ScriptedGame` owns the Lua VM. Pass it to a platform's `run()` function.

## Lua Lifecycle

The script passed to `ScriptedGame::new()` (or loaded from assets via `from_asset()`) is executed once during `init()`. It **must return a table** with `init`, `update`, and `render` keys. Missing functions are silently ignored (no panic).

```lua
local game = {}
local world, donut

function game.init()
    world = unison.World.new()
    world:set_gravity(-9.8)
    world:set_ground(-4.5)
    local tex = unison.assets.load_texture("textures/donut-pink.png")
    donut = world.objects:spawn_soft_body({
        mesh = "ring", mesh_params = {1.0, 0.25, 24, 8},
        material = "rubber",
        position = {0, 3.5},
        texture = tex,
    })
    world.cameras:follow("main", donut, { smoothing = 0.08 })
end

function game.update(dt)
    if unison.input.is_key_pressed("ArrowRight") then
        world.objects:apply_force(donut, 80, 0)
    end
    if unison.input.is_key_just_pressed("Space") and world.objects:is_grounded(donut) then
        world.objects:apply_impulse(donut, 0, 10)
    end
    world:step(dt)
end

function game.render()
    world:render()
end

return game
```

---

## World

Create and manage a physics world with objects, cameras, and rendering.

### Constructor

| Function | Signature | Description |
|----------|-----------|-------------|
| `unison.World.new` | `() → World` | Create a new World with default settings (main camera, -9.8 gravity) |

### Configuration

| Method | Signature | Description |
|--------|-----------|-------------|
| `world:set_background` | `(hex: integer)` | Set background color from hex (e.g. `0x1a1a2e`) |
| `world:set_gravity` | `(g: number)` | Set gravity strength (negative = downward) |
| `world:set_ground` | `(y: number)` | Set flat ground plane at Y position |
| `world:set_ground_restitution` | `(r: number)` | Set ground bounce factor (0=no bounce, 1=perfect) |
| `world:set_ground_friction` | `(f: number)` | Set ground friction (0=ice, 1=sticky) |

### Simulation & Rendering

| Method | Signature | Description |
|--------|-----------|-------------|
| `world:step` | `(dt: number)` | Advance physics simulation by `dt` seconds |
| `world:render` | `()` | Render all objects and lighting through the main camera |
| `world:render_to_targets` | `(mapping: table)` | Render each named camera to a specific render target |
| `world:draw_overlay` | `(tex, x, y, w, h)` | Composite a render-target texture onto the screen |
| `world:draw_overlay_bordered` | `(tex, x, y, w, h, bw, bc)` | Like draw_overlay with a colored border |

---

## Objects

Spawn, despawn, and interact with physics objects via `world.objects`. All spawn functions return an integer **object ID**.

### Spawning

| Method | Signature | Description |
|--------|-----------|-------------|
| `world.objects:spawn_soft_body` | `(desc: table) → id` | Spawn a soft body from descriptor table |
| `world.objects:spawn_rigid_body` | `(desc: table) → id` | Spawn a rigid body from descriptor table |
| `world.objects:spawn_static_rect` | `(desc: table) → id` | Spawn a static rectangle |
| `world.objects:spawn_sprite` | `(desc: table) → id` | Spawn a sprite (visual only, no physics) |
| `world.objects:despawn` | `(id: integer)` | Remove an object from the world |

### Soft Body Descriptor

```lua
{
    mesh = "ring",                 -- "ring", "square", "ellipse", "star", "blob", "rounded_box"
    mesh_params = {1.0, 0.25, 24, 8},  -- depends on mesh type (see below)
    material = "rubber",           -- preset string or custom table
    position = {x, y},
    color = 0xFFFFFF,              -- hex integer (optional, default white)
    texture = texture_id,          -- from unison.assets.load_texture (optional)
}
```

**Mesh types and params:**

| Mesh | Params |
|------|--------|
| `"ring"` | `{outer_radius, inner_radius, segments, radial_divisions}` |
| `"square"` | `{size, divisions}` |
| `"ellipse"` | `{radius_x, radius_y, segments, rings}` |
| `"star"` | `{outer_radius, inner_radius, points, divisions}` |
| `"blob"` | `{radius, variation, segments, rings, seed}` |
| `"rounded_box"` | `{width, height, corner_radius, corner_segments}` |

**Material presets:** `"rubber"`, `"jello"`, `"wood"`, `"metal"`, `"slime"`

**Custom material:** `{density = 900, edge_compliance = 5e-6, area_compliance = 2e-5}`

### Rigid Body Descriptor

```lua
{
    collider = "aabb",       -- "aabb" or "circle"
    half_width = 2,          -- for "aabb"
    half_height = 0.5,       -- for "aabb"
    radius = 1.0,            -- for "circle"
    position = {x, y},
    color = 0x00FF00,
    is_static = true,        -- optional, default false
}
```

### Sprite Descriptor

```lua
{
    texture = texture_id,    -- optional
    position = {x, y},
    size = {w, h},           -- optional, default {1,1}
    rotation = 0,            -- radians, optional
    color = 0xFFFFFF,        -- optional
}
```

### Physics Interaction

| Method | Signature | Description |
|--------|-----------|-------------|
| `world.objects:apply_force` | `(id, fx, fy)` | Apply a continuous force (use in update) |
| `world.objects:apply_impulse` | `(id, ix, iy)` | Apply an instant velocity change |
| `world.objects:apply_torque` | `(id, torque)` | Apply rotational torque |

### Queries

| Method | Signature | Description |
|--------|-----------|-------------|
| `world.objects:position` | `(id) → x, y` | Get object center position |
| `world.objects:velocity` | `(id) → vx, vy` | Get object velocity |
| `world.objects:is_grounded` | `(id) → bool` | True if object is resting on ground |
| `world.objects:is_touching` | `(a, b) → bool` | True if two objects are in contact |

### Display Properties

| Method | Signature | Description |
|--------|-----------|-------------|
| `world.objects:set_z_order` | `(id, z: integer)` | Set draw order (higher = on top) |
| `world.objects:set_casts_shadow` | `(id, bool)` | Enable/disable shadow casting |
| `world.objects:set_position` | `(id, x, y)` | Teleport object to position |

---

## Input

`unison.input` — refreshed each frame automatically.

| Function | Signature | Description |
|----------|-----------|-------------|
| `unison.input.is_key_pressed` | `(key: string) → bool` | True while key is held down |
| `unison.input.is_key_just_pressed` | `(key: string) → bool` | True only on the frame the key was pressed |
| `unison.input.is_key_just_released` | `(key: string) → bool` | True only on the frame the key was released |
| `unison.input.touches_started` | `() → [{x, y}, ...]` | Array of new touch positions this frame |
| `unison.input.is_mouse_button_pressed` | `(btn) → bool` | True while mouse button held (0=Left, 1=Right, 2=Middle) |
| `unison.input.is_mouse_button_just_pressed` | `(btn) → bool` | True on the frame the mouse button was first pressed |
| `unison.input.mouse_position` | `() → x, y` | Current mouse position in screen space |
| `unison.input.is_pointer_just_pressed` | `() → bool` | True if touch began or primary mouse button just pressed |
| `unison.input.pointer_position` | `() → x, y` | Active touch or held-mouse position |
| `unison.input.bind_action` | `(name, opts)` | Bind a named action to keys and/or mouse buttons |
| `unison.input.bind_axis` | `(name, opts)` | Bind a named axis to negative/positive keys or joystick |
| `unison.input.is_action_pressed` | `(name) → bool` | True while any bound input is held |
| `unison.input.is_action_just_pressed` | `(name) → bool` | True only on the frame the action was first triggered |
| `unison.input.is_action_just_released` | `(name) → bool` | True only on the frame all bound inputs were released |
| `unison.input.axis` | `(name) → number` | Digital axis value in [-1, 1] |

**Key names:** `"ArrowUp"`, `"ArrowDown"`, `"ArrowLeft"`, `"ArrowRight"`, `"Space"`, `"Enter"`, `"Escape"`, `"Tab"`, `"Backspace"`, `"ShiftLeft"`, `"ShiftRight"`, `"ControlLeft"`, `"ControlRight"`, `"AltLeft"`, `"AltRight"`, single letters `"A"`–`"Z"`, digits `"0"`–`"9"` or `"Digit0"`–`"Digit9"`.

---

## Cameras

Camera management via `world.cameras`.

| Method | Signature | Description |
|--------|-----------|-------------|
| `world.cameras:add` | `(name, width, height)` | Add a named camera with viewport size |
| `world.cameras:follow` | `(name, id, opts?)` | Make camera follow an object (opts: smoothing, offset) |
| `world.cameras:unfollow` | `(name)` | Stop following |
| `world.cameras:position` | `(name) → x, y` | Get camera center position |
| `world.cameras:screen_to_world` | `(sx, sy) → wx, wy` | Convert screen-space to world-space |

---

## Assets

| Function | Signature | Description |
|----------|-----------|-------------|
| `unison.assets.load_texture` | `(path: string) → integer` | Load texture from embedded assets, returns texture ID |

---

## Renderer

| Function | Signature | Description |
|----------|-----------|-------------|
| `unison.renderer.screen_size` | `() → width, height` | Get screen dimensions in logical points |
| `unison.renderer.anti_aliasing` | `() → mode` | Current AA mode |
| `unison.renderer.set_anti_aliasing` | `(mode: string)` | Set AA mode: `"none"`, `"msaa2x"`, `"msaa4x"`, `"msaa8x"` |
| `unison.renderer.create_target` | `(w, h) → target_id, tex_id` | Create an offscreen render target |

---

## Lighting

Control the lighting system through `world.lights`.

### System Configuration

| Method | Signature | Description |
|--------|-----------|-------------|
| `world.lights:set_enabled` | `(bool)` | Enable/disable the lighting system |
| `world.lights:set_ambient` | `(r, g, b, a)` | Set ambient light color |
| `world.lights:set_ground_shadow` | `(y)` or `(nil)` | Set ground shadow plane, or nil to disable |

### Point Lights

| Method | Signature | Description |
|--------|-----------|-------------|
| `world.lights:add_point` | `(desc) → handle` | Add point light from descriptor table |
| `world.lights:set_intensity` | `(handle, intensity)` | Update light intensity |
| `world.lights:follow` | `(handle, object_id, opts?)` | Make light track an object (opts: offset) |
| `world.lights:unfollow` | `(handle)` | Stop tracking |

**Point light descriptor:**
```lua
{
    position = {x, y},
    color = 0xFFDD44,        -- hex color (optional, default white)
    intensity = 2.0,         -- multiplier (optional, default 1.0)
    radius = 8.0,            -- world units (optional, default 5.0)
    casts_shadows = true,    -- optional, default false
    shadow = "soft",         -- "hard", "soft", or custom table
}
```

### Directional Lights

| Method | Signature | Description |
|--------|-----------|-------------|
| `world.lights:add_directional` | `(desc) → handle` | Add directional light |
| `world.lights:set_direction` | `(handle, dx, dy)` | Update direction |

**Directional light descriptor:**
```lua
{
    direction = {-0.5, -1.0},
    color = 0xFFFFFF,
    intensity = 0.8,
    casts_shadows = true,
    shadow = { filter = "pcf5", distance = 6.0, strength = 0.7 },
}
```

---

## Events

`unison.events` — string-keyed pub/sub.

| Function | Signature | Description |
|----------|-----------|-------------|
| `unison.events.on` | `(name, callback)` | Register a callback for named event |
| `unison.events.emit` | `(name, data?)` | Emit event with optional data table |
| `unison.events.clear` | `()` | Clear all string-keyed event handlers and pending events |

### Collision Callbacks

| Function | Signature | Description |
|----------|-----------|-------------|
| `world:on_collision` | `(fn(a, b, info))` | Callback for any collision pair each frame |
| `world:on_collision_with` | `(id, fn(other, info))` | Callback for specific object |
| `world:on_collision_between` | `(a, b, fn(info))` | Callback for specific pair |

**Collision info table:** `{ normal_x, normal_y, penetration, contact_x, contact_y }`

---

## Scenes

Scenes are Lua tables with optional lifecycle functions. Set the active scene via `unison.scenes.set(scene)`.

| Function | Signature | Description |
|----------|-----------|-------------|
| `unison.scenes.set` | `(scene_table)` | Set initial scene, calls `on_enter` |
| `unison.scenes.current` | `() → scene or nil` | Return the active scene |

**Scene table format:**
```lua
local scene = {
    on_enter = function() ... end,   -- called when scene starts
    update   = function(dt) ... end, -- called each frame
    render   = function() ... end,   -- called each frame
    on_exit  = function() ... end,   -- called when switching away
}
```

When a scene is active, the scene's `update`/`render` are called instead of `game.update`/`game.render`.

---

## Render Layers

Create named render layers with different lighting/clear settings.

| Method | Signature | Description |
|--------|-----------|-------------|
| `world:create_render_layer` | `(name, desc) → handle` | Create a named layer |
| `world:create_render_layer_before` | `(name, before, desc) → handle` | Insert before existing layer |
| `world:set_layer_clear_color` | `(handle, hex)` | Update layer clear color |
| `world:default_layer` | `() → handle` | Get the default scene layer |
| `world:draw_to` | `(layer, shape, params, z)` | Draw shape to specific layer |
| `world:draw` | `(shape, params, z)` | Draw to default layer |
| `world:draw_unlit` | `(shape, params, z)` | Draw unlit (not affected by lightmap) |

**Layer descriptor:** `{ lit = false, clear_color = 0x020206 }`

**Shape types:** `"rect"` `{ x, y, width, height, color }`, `"line"` `{ x1, y1, x2, y2, color, width }`, `"circle"` `{ x, y, radius, color }`

---

## Math Utilities

### Color

| Function | Signature | Description |
|----------|-----------|-------------|
| `unison.Color.hex` | `(hex) → Color` | Create color from hex integer |
| `unison.Color.rgba` | `(r, g, b, a) → Color` | Create from RGBA components |
| `color:lerp` | `(other, t) → Color` | Interpolate between colors |

Color fields: `color.r`, `color.g`, `color.b`, `color.a`

### Rng

| Function | Signature | Description |
|----------|-----------|-------------|
| `unison.Rng.new` | `(seed) → Rng` | Create deterministic RNG |
| `rng:range` | `(min, max) → number` | Random float in [min, max) |
| `rng:range_int` | `(min, max) → integer` | Random integer in [min, max] |

### Math Extensions

| Function | Signature | Description |
|----------|-----------|-------------|
| `unison.math.lerp` | `(a, b, t) → number` | Linear interpolation |
| `unison.math.smoothstep` | `(edge0, edge1, x) → number` | Smooth Hermite interpolation |
| `unison.math.clamp` | `(x, min, max) → number` | Clamp value to range |

---

## UI

Declarative UI built from Lua tables.

| Function | Signature | Description |
|----------|-----------|-------------|
| `unison.UI.new` | `(font_path) → UI` | Create UI handle from font asset |
| `ui:frame` | `(tree_table)` | Render a UI frame from nested table tree |

**Node types:** `"column"`, `"row"`, `"panel"`, `"label"`, `"button"`, `"spacer"`

Button `on_click` values are emitted as string events. Listen with `unison.events.on("click_name", callback)`.

```lua
local ui = unison.UI.new("fonts/DejaVuSans-Bold.ttf")

ui:frame({
    { type = "column", anchor = "center", gap = 10, children = {
        { type = "label", text = "Title", font_size = 32 },
        { type = "button", text = "Play", on_click = "start_game",
          width = 200, height = 60, font_size = 24 },
    }},
})
```

---

## debug Global

`unison.debug` — development utilities.

| Function | Signature | Description |
|----------|-----------|-------------|
| `unison.debug.log` | `(...)` | Print varargs to stderr, joined with tabs (uses `tostring` on each value) |
| `unison.debug.draw_point` | `(x, y, color: integer)` | Draw a 0.1-unit point at world position; color is hex |
| `unison.debug.show_physics` | `(enabled: bool)` | Toggle physics debug visualization (stub — not yet wired) |
| `unison.debug.show_fps` | `(enabled: bool)` | Toggle FPS counter overlay (stub — not yet wired) |

## Modules & require()

Scripts can use `require()` to load other Lua modules from embedded assets. All `.lua` files under `project/assets/scripts/` are automatically registered as modules.

```lua
-- Loads scripts/scenes/shared.lua
local shared = require("scenes/shared")
```

---

## Hot Reload

`ScriptedGame::reload(new_source: &str)` — replace the running script at runtime.

Two levels are attempted in order:

- **Level 2 (default) — VM-preserving:** Re-execute the new source in the existing VM,
  replacing `__game`. World state and all other globals are preserved. New `update`/`render`
  take effect on the next frame.
- **Level 1 (fallback) — Full restart:** If Level 2 fails, destroy the VM, create a fresh
  one, re-register all bindings, re-execute the script, and call `init()`. World state is lost.

In release builds `reload()` is a no-op (`#[cfg(not(debug_assertions))]`).

Use [`hot_reload::ScriptWatcher`] to poll the filesystem on native debug builds:

```rust
use unison_scripting::hot_reload::ScriptWatcher;
let mut watcher = ScriptWatcher::new("project/assets/scripts/main.lua");
// Each frame:
if let Some(src) = watcher.check() { game.reload(&src); }
```

`ScriptWatcher` is not compiled for `wasm32` or release. On web, Trunk's dev server
triggers a full page reload on file change — no in-process watcher needed.

See [docs/scripting/hot-reload.md](../scripting/hot-reload.md) for the full guide.

## Error Overlay

`ErrorOverlay` (in `unison_scripting::error_overlay`) captures Lua runtime errors
and renders a visible indicator in debug builds.

In **debug builds**: when any lifecycle call (`init`, `update`, `render`) returns a Lua
error, the message is stored and a red bar is drawn at the top of the screen on every
frame until the error is cleared. The full message is also printed to stderr.

In **release builds**: the overlay is compiled out entirely; errors go to stderr only.

No script code is needed to use the error overlay — it is always active in `ScriptedGame`
debug builds.

## Error Handling

- **Syntax errors** in the script: logged to stderr + error overlay, `init`/`update`/`render` become no-ops.
- **Runtime errors** in lifecycle functions: logged to stderr + error overlay, game continues.
- Neither type causes a panic.

## WASM Notes

Compiling for `wasm32-unknown-unknown` requires LLVM clang (Apple Clang lacks the WebAssembly backend):

```
brew install llvm
```

The `CC_wasm32_unknown_unknown` env var is pre-configured in the root `.cargo/config.toml`. The `unison-lua` workspace crate (at `crates/unison-lua/`, substituted for `lua-src` via `[patch.crates-io]`) adds `wasm32` build support and includes a minimal libc sysroot (`crates/unison-lua/wasm-sysroot/`). See [scripting/rationale.md](../scripting/rationale.md) for why the fork exists.

Lua's `setjmp`/`longjmp` error handling is replaced with a JS exception bridge (`wasm_lua_throw` / `wasm_protected_call` in `project/wasm_libc.rs`), patched in `crates/unison-lua/lua-5.4.7/ldo.c`.

## Script Loading

Scripts are loaded from embedded assets at runtime. Place Lua scripts in `project/assets/scripts/` — they are embedded at build time by `build.rs`. The entry point is `scripts/main.lua`.

```rust
// In project/lib.rs:
use unison_scripting::ScriptedGame;
let game = ScriptedGame::from_asset("scripts/main.lua", assets::ASSETS);
```

## See also

- [Scripting Rationale](../scripting/rationale.md) — design decisions behind Unison's scripting layer (Lua 5.4 vs 5.1, no LuaJIT, forked lua-src, mlua)
