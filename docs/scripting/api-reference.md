# Lua API Reference

All globals available to scripts running inside `ScriptedGame`. Every global is
registered before `game.init()` is called.

---

## engine

Configuration, texture loading, screen info, and scene management.

### Textures & Screen

| Function | Signature | Description |
|----------|-----------|-------------|
| `engine.load_texture` | `(path: string) → integer` | Load texture from embedded assets; returns a texture ID integer. Call in `init` or `on_enter`. |
| `engine.screen_size` | `() → width, height` | Current screen dimensions in logical points. |
| `engine.set_anti_aliasing` | `(mode: string)` | Request AA mode for this session: `"none"`, `"msaa2x"`, `"msaa4x"`, `"msaa8x"`. Applied after `init` returns. |

### Scene Management

| Function | Signature | Description |
|----------|-----------|-------------|
| `engine.set_scene` | `(scene: table)` | Activate a scene; calls `scene.on_enter()`. The scene's `update`/`render` replace `game.update`/`game.render` from this point on. |
| `engine.switch_scene` | `(scene: table)` | Transition to a new scene: calls `on_exit` on the current scene, then `on_enter` on the new one. |

### UI

| Function | Signature | Description |
|----------|-----------|-------------|
| `engine.create_ui` | `(font_path: string) → UI` | Create a UI handle using the given font asset. Reuse the handle across frames. |

### Render Targets

| Function | Signature | Description |
|----------|-----------|-------------|
| `engine.create_render_target` | `(w: integer, h: integer) → target_id, texture_id` | Create an offscreen render target. Returns the target ID and its associated texture ID. |
| `engine.draw_overlay` | `(texture_id, x, y, w, h)` | Composite a render-target texture onto the screen. Coordinates are in screen-space (-0.5…0.5). |
| `engine.draw_overlay_bordered` | `(texture_id, x, y, w, h, border_width, border_color)` | Like `draw_overlay` with a colored border. `border_color` is a hex integer. |

### Legacy / Low-level

These are available for simple cases without a World:

| Function | Signature | Description |
|----------|-----------|-------------|
| `engine.set_background` | `(hex)` or `(r, g, b)` | Set clear color. Prefer `world:set_background`. |
| `engine.draw_rect` | `(x, y, w, h, r, g, b)` | Draw a colored rectangle. Prefer `world:auto_render`. |

---

## input

Input state, refreshed automatically before each `update`. Query raw key/touch state
directly — no action mapping required.

| Function | Signature | Description |
|----------|-----------|-------------|
| `input.is_key_pressed` | `(key: string) → bool` | `true` while key is held down. |
| `input.is_key_just_pressed` | `(key: string) → bool` | `true` only on the frame the key was first pressed. |
| `input.axis_x` | `() → number` | Horizontal axis in [-1, 1] from joystick or touch joystick. |
| `input.axis_y` | `() → number` | Vertical axis in [-1, 1]. |
| `input.touches_just_began` | `() → [{x, y}, ...]` | Array of new touch-start positions this frame. |

**Key name strings:**
- Arrow keys: `"ArrowUp"`, `"ArrowDown"`, `"ArrowLeft"`, `"ArrowRight"`
- Common: `"Space"`, `"Enter"`, `"Escape"`, `"Tab"`, `"Backspace"`
- Modifiers: `"ShiftLeft"`, `"ShiftRight"`, `"ControlLeft"`, `"ControlRight"`, `"AltLeft"`, `"AltRight"`
- Letters: `"A"`–`"Z"` (uppercase)
- Digits: `"0"`–`"9"` or `"Digit0"`–`"Digit9"`

---

## events

String-keyed pub/sub event bus and collision callbacks.

### String Events

| Function | Signature | Description |
|----------|-----------|-------------|
| `events.on` | `(name: string, fn)` | Register a callback for a named event. Multiple listeners are allowed. |
| `events.emit` | `(name: string, data?: table)` | Emit a named event with an optional data payload. Callbacks fire at end of frame. |

### Collision Events

| Function | Signature | Description |
|----------|-----------|-------------|
| `events.on_collision` | `(fn(a, b, info))` | Called for every collision pair each frame. |
| `events.on_collision_for` | `(id, fn(other, info))` | Called when `id` collides with anything. |
| `events.on_collision_between` | `(a, b, fn(info))` | Called when `a` and `b` collide. |

**Collision info table fields:**
```lua
info.normal_x      -- contact normal X
info.normal_y      -- contact normal Y
info.penetration   -- penetration depth
info.contact_x     -- contact point X
info.contact_y     -- contact point Y
```

---

## World

Physics world containing objects, cameras, lighting, and rendering. Each scene
typically creates its own World.

### Constructor

| Function | Signature | Description |
|----------|-----------|-------------|
| `World.new` | `() → World` | Create a new World. Default: `"main"` camera, gravity -9.8. |

### World Configuration

| Method | Signature | Description |
|--------|-----------|-------------|
| `world:set_background` | `(hex: integer)` | Set background clear color (e.g. `0x1a1a2e`). |
| `world:set_gravity` | `(g: number)` | Set gravity (negative = downward, e.g. `-9.8`). |
| `world:set_ground` | `(y: number)` | Add a flat ground plane at world Y. |
| `world:set_ground_restitution` | `(r: number)` | Ground bounciness: 0 = no bounce, 1 = perfect elastic. |
| `world:set_ground_friction` | `(f: number)` | Ground friction: 0 = frictionless, 1 = sticky. |

### Simulation & Rendering

| Method | Signature | Description |
|--------|-----------|-------------|
| `world:step` | `(dt: number)` | Advance physics by `dt` seconds. Call in `update`. |
| `world:auto_render` | `()` | Render all objects and lighting through the main camera. Call in `render`. |
| `world:render_to_targets` | `(mapping: table)` | Render each named camera to a specific render target. See Render Targets section. |

---

## Objects

Spawn and interact with physics objects. All spawn functions return an integer **object ID**.

### Spawning

| Method | Signature | Description |
|--------|-----------|-------------|
| `world:spawn_soft_body` | `(desc: table) → id` | Spawn a deformable soft body. |
| `world:spawn_rigid_body` | `(desc: table) → id` | Spawn a rigid body with AABB or circle collider. |
| `world:spawn_static_rect` | `(pos, size, color) → id` | Spawn an immovable rectangle. `pos` and `size` are `{x, y}` tables; `color` is hex. |
| `world:spawn_sprite` | `(desc: table) → id` | Spawn a visual-only sprite (no physics). |
| `world:despawn` | `(id: integer)` | Remove an object from the world. |

### Soft Body Descriptor

```lua
{
    mesh = "ring",                       -- shape type (see below)
    mesh_params = {1.0, 0.25, 24, 8},   -- shape parameters
    material = "rubber",                 -- material preset or custom table
    position = {x, y},
    color = 0xFFFFFF,                    -- optional hex tint
    texture = texture_id,                -- optional, from engine.load_texture
}
```

**Mesh types:**

| Mesh | Params |
|------|--------|
| `"ring"` | `{outer_radius, inner_radius, segments, radial_divisions}` |
| `"square"` | `{size, divisions}` |
| `"ellipse"` | `{radius_x, radius_y, segments, rings}` |
| `"star"` | `{outer_radius, inner_radius, points, divisions}` |
| `"blob"` | `{radius, variation, segments, rings, seed}` |
| `"rounded_box"` | `{width, height, corner_radius, corner_segments}` |

**Material presets:** `"rubber"`, `"jello"`, `"wood"`, `"metal"`, `"slime"`

**Custom material:** `{ density = 900, edge_compliance = 5e-6, area_compliance = 2e-5 }`

### Rigid Body Descriptor

```lua
{
    collider = "aabb",     -- "aabb" or "circle"
    half_width = 2.0,      -- for "aabb"
    half_height = 0.5,     -- for "aabb"
    radius = 1.0,          -- for "circle"
    position = {x, y},
    color = 0x00FF00,      -- optional hex
    is_static = false,     -- optional, default false
}
```

### Sprite Descriptor

```lua
{
    texture = texture_id,  -- optional
    position = {x, y},
    size = {w, h},         -- optional, default {1, 1}
    rotation = 0,          -- radians, optional
    color = 0xFFFFFF,      -- optional hex tint
}
```

### Physics Interaction

| Method | Signature | Description |
|--------|-----------|-------------|
| `world:apply_force` | `(id, fx, fy)` | Apply a continuous force each frame (call in `update`). |
| `world:apply_impulse` | `(id, ix, iy)` | Apply an instantaneous velocity change. |
| `world:apply_torque` | `(id, torque, dt)` | Apply rotational torque. |

### Queries

| Method | Signature | Description |
|--------|-----------|-------------|
| `world:get_position` | `(id) → x, y` | Get object center position. Returns two numbers. |
| `world:get_velocity` | `(id) → vx, vy` | Get object velocity. Returns two numbers. |
| `world:is_grounded` | `(id) → bool` | `true` if the object is resting on the ground plane. |
| `world:is_touching` | `(a, b) → bool` | `true` if objects `a` and `b` are in contact. |

### Display Properties

| Method | Signature | Description |
|--------|-----------|-------------|
| `world:set_z_order` | `(id, z: integer)` | Set draw order. Higher values draw on top. |
| `world:set_casts_shadow` | `(id, bool)` | Enable or disable shadow casting for this object. |
| `world:set_position` | `(id, x, y)` | Teleport object to an exact position. |

---

## Camera

Camera methods on the World object. A `"main"` camera is created automatically.

| Method | Signature | Description |
|--------|-----------|-------------|
| `world:camera_follow` | `(name, id, smoothing)` | Make camera follow an object. `smoothing`: 0 = frozen, 1 = instant snap. |
| `world:camera_follow_with_offset` | `(name, id, smoothing, ox, oy)` | Follow with a world-space offset applied to the look-at point. |
| `world:camera_add` | `(name, width, height)` | Add a named camera with the given viewport size in world units. |
| `world:camera_get_position` | `(name) → x, y` | Get the current camera center position. |

---

## Lighting

Lighting methods on the World object.

### System Configuration

| Method | Signature | Description |
|--------|-----------|-------------|
| `world:lighting_set_enabled` | `(bool)` | Enable or disable the entire lighting system. |
| `world:lighting_set_ambient` | `(r, g, b, a)` | Set ambient light color as RGBA floats (0–1). |
| `world:lighting_set_ground_shadow` | `(y)` or `(nil)` | Add a ground shadow plane at Y, or pass `nil` to disable. |

### Point Lights

| Method | Signature | Description |
|--------|-----------|-------------|
| `world:add_point_light` | `(desc: table) → handle` | Add a point light; returns a light handle integer. |
| `world:set_light_intensity` | `(handle, intensity)` | Update light intensity (multiplier). |
| `world:light_follow` | `(handle, id)` | Make the light track an object each frame. |
| `world:light_follow_with_offset` | `(handle, id, ox, oy)` | Track object with a world-space offset. |
| `world:light_unfollow` | `(handle)` | Stop the light from tracking. |

**Point light descriptor:**
```lua
{
    position = {x, y},
    color = 0xFFDD44,       -- optional hex, default white
    intensity = 2.0,        -- optional multiplier, default 1.0
    radius = 8.0,           -- world units, optional default 5.0
    casts_shadows = true,   -- optional, default false
    shadow = "soft",        -- "hard", "soft", or custom table
}
```

**Custom shadow table:** `{ filter = "pcf5", distance = 6.0, strength = 0.7 }`

### Directional Lights

| Method | Signature | Description |
|--------|-----------|-------------|
| `world:add_directional_light` | `(desc: table) → handle` | Add a directional light. |
| `world:set_directional_light_direction` | `(handle, dx, dy)` | Update the light's direction vector. |

**Directional light descriptor:**
```lua
{
    direction = {-0.5, -1.0},
    color = 0xFFFFFF,           -- optional
    intensity = 0.8,            -- optional
    casts_shadows = true,       -- optional
    shadow = { filter = "pcf5", distance = 6.0, strength = 0.7 },
}
```

---

## Render Layers

Named render layers with independent lighting and clear settings. Useful for
backgrounds, foregrounds, and unlit HUD elements.

| Method | Signature | Description |
|--------|-----------|-------------|
| `world:create_render_layer` | `(name, desc) → handle` | Create a new named layer; appended after existing layers. |
| `world:create_render_layer_before` | `(name, before_name, desc) → handle` | Insert a layer before another by name. |
| `world:set_layer_clear_color` | `(handle, hex)` | Update a layer's clear color at runtime. |
| `world:default_layer` | `() → handle` | Get the handle for the default scene layer. |
| `world:draw_to` | `(layer, shape, params, z)` | Draw a shape to a specific layer at depth `z`. |
| `world:draw` | `(shape, params, z)` | Draw to the default layer. |
| `world:draw_unlit` | `(shape, params, z)` | Draw to the default layer, unaffected by the lightmap. |

**Layer descriptor:** `{ lit = false, clear_color = 0x020206 }`

**Shape types and param tables:**

| Shape | Params |
|-------|--------|
| `"rect"` | `{ x, y, width, height, color }` |
| `"line"` | `{ x1, y1, x2, y2, color, width }` |
| `"circle"` | `{ x, y, radius, color }` |

---

## UI

Declarative UI built from nested Lua tables. Button clicks are emitted as string events.

| Function | Signature | Description |
|----------|-----------|-------------|
| `engine.create_ui` | `(font_path: string) → UI` | Create a UI handle. Reuse across frames; only recreated if `font_path` changes. |
| `ui:frame` | `(tree: table)` | Render one frame of UI from a nested node table. Call in `render`. |

**Node types:**

| Type | Key fields |
|------|------------|
| `"column"` | `anchor`, `gap`, `padding`, `children` |
| `"row"` | `anchor`, `gap`, `padding`, `children` |
| `"panel"` | `anchor`, `padding`, `bg_color`, `children` |
| `"label"` | `text`, `font_size`, `font_color` |
| `"button"` | `text`, `on_click`, `width`, `height`, `font_size`, `font_color`, `bg_color` |
| `"icon"` | `texture` (texture ID integer) |
| `"progress_bar"` | `value` (0–1 float), `width`, `height` |
| `"spacer"` | `value` (size in pixels) |

**Anchor values:** `"top_left"`, `"top"`, `"top_right"`, `"left"`, `"center"`, `"right"`,
`"bottom_left"`, `"bottom"`, `"bottom_right"`

**Button click events:** A button's `on_click` string is emitted as a string event when
clicked. Listen with `events.on("my_button", callback)`.

**Example:**
```lua
ui:frame({
    { type = "panel", anchor = "center", padding = 20, bg_color = 0x1e1e2e, children = {
        { type = "column", gap = 12, children = {
            { type = "label", text = "Main Menu", font_size = 32 },
            { type = "button", text = "Play",
              on_click = "start_game", width = 200, height = 50 },
        }},
    }},
})
```

---

## Color

Color userdata with RGBA components and interpolation.

| Function | Signature | Description |
|----------|-----------|-------------|
| `Color.hex` | `(hex: integer) → Color` | Create a Color from a hex integer (e.g. `Color.hex(0xFF8800)`). |
| `Color.rgba` | `(r, g, b, a: number) → Color` | Create from RGBA floats in [0, 1]. |
| `color:lerp` | `(other: Color, t: number) → Color` | Linear interpolation between two Colors. |

**Fields:** `color.r`, `color.g`, `color.b`, `color.a` — read-only floats in [0, 1].

---

## Rng

Deterministic pseudo-random number generator (xorshift64).

| Function | Signature | Description |
|----------|-----------|-------------|
| `Rng.new` | `(seed: integer) → Rng` | Create a new RNG with the given seed. Seed 0 is treated as 1. |
| `rng:range` | `(min, max: number) → number` | Random float in `[min, max)`. |
| `rng:range_int` | `(min, max: integer) → integer` | Random integer in `[min, max]` (inclusive). |

---

## math extensions

Additional functions added to Lua's built-in `math` table.

| Function | Signature | Description |
|----------|-----------|-------------|
| `math.lerp` | `(a, b, t: number) → number` | Linear interpolation: `a + (b - a) * t`. |
| `math.smoothstep` | `(edge0, edge1, x: number) → number` | Smooth Hermite interpolation, clamped to [0, 1]. |
| `math.clamp` | `(x, min, max: number) → number` | Clamp `x` to `[min, max]`. |

All standard Lua `math.*` functions are available as usual.

---

## debug

Development utilities. Available in all builds, but intended for debug use.

| Function | Signature | Description |
|----------|-----------|-------------|
| `debug.log` | `(...)` | Print varargs to stderr, joined with tabs. Values are converted via `tostring`. |
| `debug.draw_point` | `(x, y, color: integer)` | Draw a 0.1-unit point at world position `(x, y)`. Color is hex. |
| `debug.show_physics` | `(enabled: bool)` | Toggle physics debug visualization. (Not yet wired.) |
| `debug.show_fps` | `(enabled: bool)` | Toggle FPS counter overlay. (Not yet wired.) |

The **error overlay** (`ErrorOverlay`) is separate from the `debug` global. In debug builds,
any Lua runtime error is automatically shown as a red bar at the top of the screen, with
the full message printed to stderr. No script code is needed to enable it.

---

## Scene Table Format

A scene is any Lua table with some or all of these functions:

```lua
local scene = {
    on_enter = function() ... end,   -- called when the scene starts
    update   = function(dt) ... end, -- called each frame
    render   = function() ... end,   -- called each frame
    on_exit  = function() ... end,   -- called when switching away
}
```

All four keys are optional. When a scene is active via `engine.set_scene()` or
`engine.switch_scene()`, the scene's `update`/`render` replace `game.update`/`game.render`.
