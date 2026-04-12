# API Reference

All globals available to scripts running inside `ScriptedGame`. Every global is
registered before `game.init()` is called.

TypeScript type declarations live at `crates/unison-scripting/types/`. The signatures
below match those declarations exactly.

---

## engine

Configuration, texture loading, screen info, scene management, UI, and render targets.

### Textures & Screen

#### engine.load_texture

Load a texture from embedded assets. Returns a texture ID integer. Call in `init`
or `on_enter`.

**Lua:**
```lua
local tex = engine.load_texture("textures/player.png")
```

**TypeScript:**
```typescript
const tex: TextureId = engine.load_texture("textures/player.png");
```

#### engine.screen_size

Current screen dimensions in logical points.

**Lua:**
```lua
local w, h = engine.screen_size()
```

**TypeScript:**
```typescript
const [w, h] = engine.screen_size();
```

#### engine.set_anti_aliasing

Request AA mode for this session. Applied after `init` returns.

**Lua:**
```lua
engine.set_anti_aliasing("msaa4x")  -- "none", "msaa2x", "msaa4x", "msaa8x"
```

**TypeScript:**
```typescript
engine.set_anti_aliasing("msaa4x");  // "none" | "msaa2x" | "msaa4x" | "msaa8x"
```

### Scene Management

#### engine.set_scene

Activate a scene. Calls `scene.on_enter()` if present. The scene's `update`/`render`
replace `game.update`/`game.render` from this point on.

**Lua:**
```lua
engine.set_scene(scene_table)
```

**TypeScript:**
```typescript
engine.set_scene(scene);
```

#### engine.switch_scene

Transition to a new scene: calls `on_exit` on the current scene, then `on_enter` on the
new one.

**Lua:**
```lua
engine.switch_scene(require("scenes/menu"))
```

**TypeScript:**
```typescript
import * as menu from "./scenes/menu";
engine.switch_scene(menu);
```

### UI

#### engine.create_ui

Create a UI handle using the given font asset. Reuse the handle across frames.

**Lua:**
```lua
local ui = engine.create_ui("fonts/DejaVuSans-Bold.ttf")
```

**TypeScript:**
```typescript
const ui: UI = engine.create_ui("fonts/DejaVuSans-Bold.ttf");
```

### Render Targets

#### engine.create_render_target

Create an offscreen render target. Returns the target ID and its associated texture ID.

**Lua:**
```lua
local target_id, texture_id = engine.create_render_target(512, 512)
```

**TypeScript:**
```typescript
const [target_id, texture_id] = engine.create_render_target(512, 512);
```

#### engine.draw_overlay

Composite a render-target texture onto the screen. Coordinates are in screen-space
(-0.5 to 0.5).

**Lua:**
```lua
engine.draw_overlay(texture_id, x, y, w, h)
```

**TypeScript:**
```typescript
engine.draw_overlay(texture_id, x, y, w, h);
```

#### engine.draw_overlay_bordered

Like `draw_overlay` with a colored border. `border_color` is a hex integer.

**Lua:**
```lua
engine.draw_overlay_bordered(texture_id, x, y, w, h, border_width, border_color)
```

**TypeScript:**
```typescript
engine.draw_overlay_bordered(texture_id, x, y, w, h, border_width, border_color);
```

### Legacy / Low-level

These are available for simple cases without a World.

#### engine.set_background

Set clear color. Prefer `world:set_background` / `world.set_background`.

**Lua:**
```lua
engine.set_background(0x1a1a2e)
engine.set_background(r, g, b)
```

**TypeScript:**
```typescript
engine.set_background(0x1a1a2e);
engine.set_background(r, g, b);
```

#### engine.draw_rect

Draw a colored rectangle. Prefer `world:auto_render` / `world.auto_render`.

**Lua:**
```lua
engine.draw_rect(x, y, w, h, r, g, b)
```

**TypeScript:**
```typescript
engine.draw_rect(x, y, w, h, r, g, b);
```

---

## input

Input state, refreshed automatically before each `update`. Query raw key/touch state
directly — no action mapping required.

#### input.is_key_pressed

`true` while the key is held down.

**Lua:**
```lua
if input.is_key_pressed("ArrowRight") then ... end
```

**TypeScript:**
```typescript
if (input.is_key_pressed("ArrowRight")) { ... }
```

#### input.is_key_just_pressed

`true` only on the frame the key was first pressed.

**Lua:**
```lua
if input.is_key_just_pressed("Space") then ... end
```

**TypeScript:**
```typescript
if (input.is_key_just_pressed("Space")) { ... }
```

#### input.axis_x

Horizontal axis in [-1, 1] from joystick or touch joystick.

**Lua:**
```lua
local ax = input.axis_x()
```

**TypeScript:**
```typescript
const ax: number = input.axis_x();
```

#### input.axis_y

Vertical axis in [-1, 1].

**Lua:**
```lua
local ay = input.axis_y()
```

**TypeScript:**
```typescript
const ay: number = input.axis_y();
```

#### input.touches_just_began

Array of new touch-start positions this frame.

**Lua:**
```lua
local touches = input.touches_just_began()  -- [{x, y}, ...]
for _, t in ipairs(touches) do
    debug.log(t.x, t.y)
end
```

**TypeScript:**
```typescript
const touches: TouchPosition[] = input.touches_just_began();
for (const t of touches) {
    debug.log(t.x, t.y);
}
```

**Key name strings:**
- Arrow keys: `"ArrowUp"`, `"ArrowDown"`, `"ArrowLeft"`, `"ArrowRight"`
- Common: `"Space"`, `"Enter"`, `"Escape"`, `"Tab"`, `"Backspace"`
- Modifiers: `"ShiftLeft"`, `"ShiftRight"`, `"ControlLeft"`, `"ControlRight"`, `"AltLeft"`, `"AltRight"`
- Letters: `"A"` -- `"Z"` (uppercase)
- Digits: `"0"` -- `"9"` or `"Digit0"` -- `"Digit9"`

---

## events

String-keyed pub/sub event bus and collision callbacks.

### String Events

#### events.on

Register a callback for a named event. Multiple listeners are allowed.

**Lua:**
```lua
events.on("level_complete", function(data)
    debug.log("done!", data.score)
end)
```

**TypeScript:**
```typescript
events.on("level_complete", (data) => {
    debug.log("done!", data.score);
});
```

#### events.emit

Emit a named event with an optional data payload. Callbacks fire at end of frame.

**Lua:**
```lua
events.emit("level_complete", { score = 1234 })
```

**TypeScript:**
```typescript
events.emit("level_complete", { score: 1234 });
```

#### events.clear

Clear all string-keyed event handlers and pending events. Collision handlers are NOT
cleared. Call in `on_exit`.

**Lua:**
```lua
events.clear()
```

**TypeScript:**
```typescript
events.clear();
```

### Collision Events

#### events.on_collision

Called for every collision pair each frame.

**Lua:**
```lua
events.on_collision(function(a, b, info)
    debug.log(info.normal_x, info.normal_y)
end)
```

**TypeScript:**
```typescript
events.on_collision((a: ObjectId, b: ObjectId, info: CollisionInfo) => {
    debug.log(info.normal_x, info.normal_y);
});
```

#### events.on_collision_for

Called when the given object collides with anything.

**Lua:**
```lua
events.on_collision_for(player_id, function(other, info)
    debug.log("hit", other)
end)
```

**TypeScript:**
```typescript
events.on_collision_for(player_id, (other: ObjectId, info: CollisionInfo) => {
    debug.log("hit", other);
});
```

#### events.on_collision_between

Called when objects `a` and `b` collide.

**Lua:**
```lua
events.on_collision_between(player_id, spike_id, function(info)
    debug.log("ouch!")
end)
```

**TypeScript:**
```typescript
events.on_collision_between(player_id, spike_id, (info: CollisionInfo) => {
    debug.log("ouch!");
});
```

**Collision info fields:**

| Field | Type | Description |
|-------|------|-------------|
| `normal_x` | number | Contact normal X |
| `normal_y` | number | Contact normal Y |
| `penetration` | number | Penetration depth |
| `contact_x` | number | Contact point X |
| `contact_y` | number | Contact point Y |

---

## World

Physics world containing objects, cameras, lighting, and rendering. Each scene
typically creates its own World.

### Constructor

#### World.new

Create a new World. Default: `"main"` camera, gravity -9.8.

**Lua:**
```lua
local world = World.new()
```

**TypeScript:**
```typescript
const world: World = World.new();
```

### World Configuration

#### world:set_background / world.set_background

Set background clear color (e.g. `0x1a1a2e`).

**Lua:**
```lua
world:set_background(0x1a1a2e)
```

**TypeScript:**
```typescript
world.set_background(0x1a1a2e);
```

#### world:set_gravity / world.set_gravity

Set gravity (negative = downward, e.g. `-9.8`).

**Lua:**
```lua
world:set_gravity(-9.8)
```

**TypeScript:**
```typescript
world.set_gravity(-9.8);
```

#### world:set_ground / world.set_ground

Add a flat ground plane at world Y.

**Lua:**
```lua
world:set_ground(-4.5)
```

**TypeScript:**
```typescript
world.set_ground(-4.5);
```

#### world:set_ground_restitution / world.set_ground_restitution

Ground bounciness: 0 = no bounce, 1 = perfect elastic.

**Lua:**
```lua
world:set_ground_restitution(0.2)
```

**TypeScript:**
```typescript
world.set_ground_restitution(0.2);
```

#### world:set_ground_friction / world.set_ground_friction

Ground friction: 0 = frictionless, 1 = sticky.

**Lua:**
```lua
world:set_ground_friction(0.5)
```

**TypeScript:**
```typescript
world.set_ground_friction(0.5);
```

### Simulation & Rendering

#### world:step / world.step

Advance physics by `dt` seconds. Call in `update`.

**Lua:**
```lua
world:step(dt)
```

**TypeScript:**
```typescript
world.step(dt);
```

#### world:auto_render / world.auto_render

Render all objects and lighting through the main camera. Call in `render`.

**Lua:**
```lua
world:auto_render()
```

**TypeScript:**
```typescript
world.auto_render();
```

#### world:render_to_targets / world.render_to_targets

Render each named camera to a specific render target.

**Lua:**
```lua
world:render_to_targets({
    {"main", "screen"},
    {"minimap", minimap_target_id},
})
```

**TypeScript:**
```typescript
world.render_to_targets([
    ["main", "screen"],
    ["minimap", minimap_target_id],
]);
```

---

## Objects

Spawn and interact with physics objects. All spawn functions return an integer
**object ID**.

### Spawning

#### world:spawn_soft_body / world.spawn_soft_body

Spawn a deformable soft body. Returns an object ID.

**Lua:**
```lua
local id = world:spawn_soft_body({
    mesh = "ring",
    mesh_params = {1.0, 0.25, 24, 8},
    material = "rubber",
    position = {0, 3},
    color = 0xd4943a,
    texture = tex_id,
})
```

**TypeScript:**
```typescript
const id: ObjectId = world.spawn_soft_body({
    mesh: "ring",
    mesh_params: [1.0, 0.25, 24, 8],
    material: "rubber",
    position: [0, 3],
    color: 0xd4943a,
    texture: tex_id,
});
```

**Mesh types:**

| Mesh | Params |
|------|--------|
| `"ring"` | `[outer_radius, inner_radius, segments, radial_divisions]` |
| `"square"` | `[size, divisions?]` |
| `"ellipse"` | `[radius_x, radius_y, segments, rings]` |
| `"star"` | `[outer_radius, inner_radius, points, divisions?]` |
| `"blob"` | `[radius, variation, segments, rings, seed?]` |
| `"rounded_box"` | `[width, height, corner_radius, corner_segments]` |

**Material presets:** `"rubber"`, `"jello"`, `"wood"`, `"metal"`, `"slime"`

**Custom material:**

**Lua:** `{ density = 900, edge_compliance = 5e-6, area_compliance = 2e-5 }`

**TypeScript:** `{ density: 900, edge_compliance: 5e-6, area_compliance: 2e-5 }`

#### world:spawn_rigid_body / world.spawn_rigid_body

Spawn a rigid body with AABB or circle collider. Returns an object ID.

**Lua:**
```lua
local id = world:spawn_rigid_body({
    collider = "aabb",
    half_width = 2.0,
    half_height = 0.5,
    position = {0, 1},
    color = 0x00FF00,
    is_static = false,
})
```

**TypeScript:**
```typescript
const id: ObjectId = world.spawn_rigid_body({
    collider: "aabb",
    half_width: 2.0,
    half_height: 0.5,
    position: [0, 1],
    color: 0x00FF00,
    is_static: false,
});
```

**Rigid body fields:**

| Field | Type | Description |
|-------|------|-------------|
| `collider` | `"aabb"` or `"circle"` | Collider shape |
| `half_width` | number | For `"aabb"` |
| `half_height` | number | For `"aabb"` |
| `radius` | number | For `"circle"` |
| `position` | `{x, y}` / `[number, number]` | World position |
| `color` | number | Optional hex color |
| `is_static` | boolean | Optional, default false |

#### world:spawn_static_rect / world.spawn_static_rect

Spawn an immovable rectangle. Returns an object ID.

**Lua:**
```lua
local id = world:spawn_static_rect({-3, 0}, {6, 0.5}, 0x333333)
```

**TypeScript:**
```typescript
const id: ObjectId = world.spawn_static_rect([-3, 0], [6, 0.5], 0x333333);
```

#### world:spawn_sprite / world.spawn_sprite

Spawn a visual-only sprite (no physics). Returns an object ID.

**Lua:**
```lua
local id = world:spawn_sprite({
    texture = tex_id,
    position = {0, 5},
    size = {2, 2},
    rotation = 0,
    color = 0xFFFFFF,
})
```

**TypeScript:**
```typescript
const id: ObjectId = world.spawn_sprite({
    texture: tex_id,
    position: [0, 5],
    size: [2, 2],
    rotation: 0,
    color: 0xFFFFFF,
});
```

#### world:despawn / world.despawn

Remove an object from the world.

**Lua:**
```lua
world:despawn(id)
```

**TypeScript:**
```typescript
world.despawn(id);
```

### Physics Interaction

#### world:apply_force / world.apply_force

Apply a continuous force each frame (call in `update`).

**Lua:**
```lua
world:apply_force(id, 60, 0)
```

**TypeScript:**
```typescript
world.apply_force(id, 60, 0);
```

#### world:apply_impulse / world.apply_impulse

Apply an instantaneous velocity change.

**Lua:**
```lua
world:apply_impulse(id, 0, 8)
```

**TypeScript:**
```typescript
world.apply_impulse(id, 0, 8);
```

#### world:apply_torque / world.apply_torque

Apply rotational torque.

**Lua:**
```lua
world:apply_torque(id, torque, dt)
```

**TypeScript:**
```typescript
world.apply_torque(id, torque, dt);
```

### Queries

#### world:get_position / world.get_position

Get object center position.

**Lua:**
```lua
local x, y = world:get_position(id)
```

**TypeScript:**
```typescript
const [x, y] = world.get_position(id);
```

#### world:get_velocity / world.get_velocity

Get object velocity.

**Lua:**
```lua
local vx, vy = world:get_velocity(id)
```

**TypeScript:**
```typescript
const [vx, vy] = world.get_velocity(id);
```

#### world:is_grounded / world.is_grounded

`true` if the object is resting on the ground plane.

**Lua:**
```lua
if world:is_grounded(id) then ... end
```

**TypeScript:**
```typescript
if (world.is_grounded(id)) { ... }
```

#### world:is_touching / world.is_touching

`true` if objects `a` and `b` are in contact.

**Lua:**
```lua
if world:is_touching(a, b) then ... end
```

**TypeScript:**
```typescript
if (world.is_touching(a, b)) { ... }
```

### Display Properties

#### world:set_z_order / world.set_z_order

Set draw order. Higher values draw on top.

**Lua:**
```lua
world:set_z_order(id, 10)
```

**TypeScript:**
```typescript
world.set_z_order(id, 10);
```

#### world:set_casts_shadow / world.set_casts_shadow

Enable or disable shadow casting for this object.

**Lua:**
```lua
world:set_casts_shadow(id, true)
```

**TypeScript:**
```typescript
world.set_casts_shadow(id, true);
```

#### world:set_position / world.set_position

Teleport object to an exact position.

**Lua:**
```lua
world:set_position(id, x, y)
```

**TypeScript:**
```typescript
world.set_position(id, x, y);
```

---

## Camera

Camera methods on the World object. A `"main"` camera is created automatically.

#### world:camera_follow / world.camera_follow

Make camera follow an object. `smoothing`: 0 = frozen, 1 = instant snap.

**Lua:**
```lua
world:camera_follow("main", player_id, 0.1)
```

**TypeScript:**
```typescript
world.camera_follow("main", player_id, 0.1);
```

#### world:camera_follow_with_offset / world.camera_follow_with_offset

Follow with a world-space offset applied to the look-at point.

**Lua:**
```lua
world:camera_follow_with_offset("main", player_id, 0.1, 0, 2)
```

**TypeScript:**
```typescript
world.camera_follow_with_offset("main", player_id, 0.1, 0, 2);
```

#### world:camera_add / world.camera_add

Add a named camera with the given viewport size in world units.

**Lua:**
```lua
world:camera_add("minimap", 40, 30)
```

**TypeScript:**
```typescript
world.camera_add("minimap", 40, 30);
```

#### world:camera_get_position / world.camera_get_position

Get the current camera center position.

**Lua:**
```lua
local x, y = world:camera_get_position("main")
```

**TypeScript:**
```typescript
const [x, y] = world.camera_get_position("main");
```

---

## Lighting

Lighting methods on the World object.

### System Configuration

#### world:lighting_set_enabled / world.lighting_set_enabled

Enable or disable the entire lighting system.

**Lua:**
```lua
world:lighting_set_enabled(true)
```

**TypeScript:**
```typescript
world.lighting_set_enabled(true);
```

#### world:lighting_set_ambient / world.lighting_set_ambient

Set ambient light color as RGBA floats (0 to 1).

**Lua:**
```lua
world:lighting_set_ambient(0.05, 0.05, 0.1, 1.0)
```

**TypeScript:**
```typescript
world.lighting_set_ambient(0.05, 0.05, 0.1, 1.0);
```

#### world:lighting_set_ground_shadow / world.lighting_set_ground_shadow

Add a ground shadow plane at Y, or pass `nil` / `undefined` to disable.

**Lua:**
```lua
world:lighting_set_ground_shadow(-4.5)
world:lighting_set_ground_shadow(nil)
```

**TypeScript:**
```typescript
world.lighting_set_ground_shadow(-4.5);
world.lighting_set_ground_shadow(undefined);
```

### Point Lights

#### world:add_point_light / world.add_point_light

Add a point light. Returns a light handle.

**Lua:**
```lua
local light = world:add_point_light({
    position = {0, 3.5},
    color = 0xFFDD44,
    intensity = 2.0,
    radius = 8.0,
    casts_shadows = true,
    shadow = "soft",
})
```

**TypeScript:**
```typescript
const light: LightId = world.add_point_light({
    position: [0, 3.5],
    color: 0xFFDD44,
    intensity: 2.0,
    radius: 8.0,
    casts_shadows: true,
    shadow: "soft",
});
```

**Point light descriptor fields:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `position` | `{x, y}` / `[number, number]` | required | Light position |
| `color` | number | white | Hex color |
| `intensity` | number | 1.0 | Intensity multiplier |
| `radius` | number | 5.0 | World units |
| `casts_shadows` | boolean | false | Enable shadows |
| `shadow` | `"hard"` / `"soft"` / config table | -- | Shadow settings |

**Custom shadow table:**

**Lua:** `{ filter = "pcf5", distance = 6.0, strength = 0.7 }`

**TypeScript:** `{ filter: "pcf5", distance: 6.0, strength: 0.7 }`

#### world:set_light_intensity / world.set_light_intensity

Update light intensity (multiplier).

**Lua:**
```lua
world:set_light_intensity(light, 3.0)
```

**TypeScript:**
```typescript
world.set_light_intensity(light, 3.0);
```

#### world:light_follow / world.light_follow

Make the light track an object each frame.

**Lua:**
```lua
world:light_follow(light, player_id)
```

**TypeScript:**
```typescript
world.light_follow(light, player_id);
```

#### world:light_follow_with_offset / world.light_follow_with_offset

Track object with a world-space offset.

**Lua:**
```lua
world:light_follow_with_offset(light, player_id, 0, 2)
```

**TypeScript:**
```typescript
world.light_follow_with_offset(light, player_id, 0, 2);
```

#### world:light_unfollow / world.light_unfollow

Stop the light from tracking.

**Lua:**
```lua
world:light_unfollow(light)
```

**TypeScript:**
```typescript
world.light_unfollow(light);
```

### Directional Lights

#### world:add_directional_light / world.add_directional_light

Add a directional light. Returns a light handle.

**Lua:**
```lua
local light = world:add_directional_light({
    direction = {-0.5, -1.0},
    color = 0xFFFFFF,
    intensity = 0.8,
    casts_shadows = true,
    shadow = { filter = "pcf5", distance = 6.0, strength = 0.7 },
})
```

**TypeScript:**
```typescript
const light: LightId = world.add_directional_light({
    direction: [-0.5, -1.0],
    color: 0xFFFFFF,
    intensity: 0.8,
    casts_shadows: true,
    shadow: { filter: "pcf5", distance: 6.0, strength: 0.7 },
});
```

#### world:set_directional_light_direction / world.set_directional_light_direction

Update the light's direction vector.

**Lua:**
```lua
world:set_directional_light_direction(light, -0.3, -1.0)
```

**TypeScript:**
```typescript
world.set_directional_light_direction(light, -0.3, -1.0);
```

---

## Render Layers

Named render layers with independent lighting and clear settings. Useful for
backgrounds, foregrounds, and unlit HUD elements.

#### world:create_render_layer / world.create_render_layer

Create a new named layer, appended after existing layers. Returns a layer handle.

**Lua:**
```lua
local bg = world:create_render_layer("background", { lit = false, clear_color = 0x020206 })
```

**TypeScript:**
```typescript
const bg: RenderLayerId = world.create_render_layer("background", { lit: false, clear_color: 0x020206 });
```

#### world:create_render_layer_before / world.create_render_layer_before

Insert a layer before another by handle. Returns a layer handle.

**Lua:**
```lua
local fg = world:create_render_layer_before("foreground", bg, { lit = true })
```

**TypeScript:**
```typescript
const fg: RenderLayerId = world.create_render_layer_before("foreground", bg, { lit: true });
```

#### world:set_layer_clear_color / world.set_layer_clear_color

Update a layer's clear color at runtime.

**Lua:**
```lua
world:set_layer_clear_color(bg, 0x111122)
```

**TypeScript:**
```typescript
world.set_layer_clear_color(bg, 0x111122);
```

#### world:default_layer / world.default_layer

Get the handle for the default scene layer.

**Lua:**
```lua
local layer = world:default_layer()
```

**TypeScript:**
```typescript
const layer: RenderLayerId = world.default_layer();
```

#### world:draw_to / world.draw_to

Draw a shape to a specific layer at depth `z`.

**Lua:**
```lua
world:draw_to(layer, "rect", { x = 0, y = 0, width = 2, height = 1, color = 0xFF0000 }, 5)
```

**TypeScript:**
```typescript
world.draw_to(layer, "rect", { x: 0, y: 0, width: 2, height: 1, color: 0xFF0000 }, 5);
```

#### world:draw / world.draw

Draw to the default layer.

**Lua:**
```lua
world:draw("circle", { x = 0, y = 0, radius = 1, color = 0x00FF00 }, 3)
```

**TypeScript:**
```typescript
world.draw("circle", { x: 0, y: 0, radius: 1, color: 0x00FF00 }, 3);
```

#### world:draw_unlit / world.draw_unlit

Draw to the default layer, unaffected by the lightmap.

**Lua:**
```lua
world:draw_unlit("line", { x1 = -1, y1 = 0, x2 = 1, y2 = 0, color = 0xFFFF00, width = 0.05 }, 10)
```

**TypeScript:**
```typescript
world.draw_unlit("line", { x1: -1, y1: 0, x2: 1, y2: 0, color: 0xFFFF00, width: 0.05 }, 10);
```

**Shape types and param tables:**

| Shape | Params |
|-------|--------|
| `"rect"` | `x, y, width, height, color` |
| `"line"` | `x1, y1, x2, y2, color, width?` |
| `"circle"` | `x, y, radius, color` |
| `"gradient_circle"` | `x, y, radius, color` |

Color can be a hex integer or an `[r, g, b]` / `[r, g, b, a]` float array.

---

## UI

Declarative UI built from nested tables. Button clicks are emitted as string events.

#### ui:frame / ui.frame

Render one frame of UI from a nested node table. Call in `render`.

**Lua:**
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

**TypeScript:**
```typescript
ui.frame([
    { type: "panel", anchor: "center", padding: 20, bg_color: 0x1e1e2e, children: [
        { type: "column", gap: 12, children: [
            { type: "label", text: "Main Menu", font_size: 32 },
            { type: "button", text: "Play",
              on_click: "start_game", width: 200, height: 50 },
        ]},
    ]},
]);
```

**Node types:**

| Type | Key fields |
|------|------------|
| `"column"` | `anchor`, `gap`, `padding`, `children` |
| `"row"` | `anchor`, `gap`, `padding`, `children` |
| `"panel"` | `anchor`, `padding`, `bg_color`, `children` |
| `"label"` | `text`, `font_size`, `font_color` |
| `"button"` | `text`, `on_click`, `width`, `height`, `font_size`, `font_color`, `bg_color` |
| `"icon"` | `texture` (texture ID integer) |
| `"progress_bar"` | `value` (0 to 1 float), `width`, `height` |
| `"spacer"` | `value` (size in pixels) |

**Anchor values:** `"top_left"`, `"top"`, `"top_right"`, `"left"`, `"center"`, `"right"`,
`"bottom_left"`, `"bottom"`, `"bottom_right"`

**Button click events:** A button's `on_click` string is emitted as a string event when
clicked. Listen with `events.on("my_button", callback)`.

---

## Color

Color userdata with RGBA components and interpolation.

#### Color.hex

Create a Color from a hex integer.

**Lua:**
```lua
local c = Color.hex(0xFF8800)
```

**TypeScript:**
```typescript
const c: Color = Color.hex(0xFF8800);
```

#### Color.rgba

Create from RGBA floats in [0, 1].

**Lua:**
```lua
local c = Color.rgba(1.0, 0.5, 0.0, 1.0)
```

**TypeScript:**
```typescript
const c: Color = Color.rgba(1.0, 0.5, 0.0, 1.0);
```

#### color:lerp / color.lerp

Linear interpolation between two Colors.

**Lua:**
```lua
local blended = c1:lerp(c2, 0.5)
```

**TypeScript:**
```typescript
const blended: Color = c1.lerp(c2, 0.5);
```

**Fields:** `color.r`, `color.g`, `color.b`, `color.a` -- read-only floats in [0, 1].

---

## Rng

Deterministic pseudo-random number generator (xorshift64).

#### Rng.new

Create a new RNG with the given seed. Seed 0 is treated as 1.

**Lua:**
```lua
local rng = Rng.new(42)
```

**TypeScript:**
```typescript
const rng: Rng = Rng.new(42);
```

#### rng:range / rng.range

Random float in `[min, max)`.

**Lua:**
```lua
local val = rng:range(0, 10)
```

**TypeScript:**
```typescript
const val: number = rng.range(0, 10);
```

#### rng:range_int / rng.range_int

Random integer in `[min, max]` (inclusive).

**Lua:**
```lua
local val = rng:range_int(1, 6)
```

**TypeScript:**
```typescript
const val: number = rng.range_int(1, 6);
```

---

## math extensions

Additional functions added to Lua's built-in `math` table. In TypeScript, these are
available on the global `math` namespace.

#### math.lerp

Linear interpolation: `a + (b - a) * t`.

**Lua:**
```lua
local v = math.lerp(0, 100, 0.5)
```

**TypeScript:**
```typescript
const v: number = math.lerp(0, 100, 0.5);
```

#### math.smoothstep

Smooth Hermite interpolation, clamped to [0, 1].

**Lua:**
```lua
local v = math.smoothstep(0, 1, x)
```

**TypeScript:**
```typescript
const v: number = math.smoothstep(0, 1, x);
```

#### math.clamp

Clamp `x` to `[min, max]`.

**Lua:**
```lua
local v = math.clamp(x, 0, 1)
```

**TypeScript:**
```typescript
const v: number = math.clamp(x, 0, 1);
```

All standard Lua `math.*` functions are available as usual.

---

## debug

Development utilities. Available in all builds, but intended for debug use.

#### debug.log

Print varargs to stderr, joined with tabs. Values are converted via `tostring`.

**Lua:**
```lua
debug.log("player pos:", x, y)
```

**TypeScript:**
```typescript
debug.log("player pos:", x, y);
```

#### debug.draw_point

Draw a 0.1-unit point at world position `(x, y)`. Color is a hex integer.

**Lua:**
```lua
debug.draw_point(x, y, 0xFF0000)
```

**TypeScript:**
```typescript
debug.draw_point(x, y, 0xFF0000);
```

#### debug.show_physics

Toggle physics debug visualization. (Not yet wired.)

**Lua:**
```lua
debug.show_physics(true)
```

**TypeScript:**
```typescript
debug.show_physics(true);
```

#### debug.show_fps

Toggle FPS counter overlay. (Not yet wired.)

**Lua:**
```lua
debug.show_fps(true)
```

**TypeScript:**
```typescript
debug.show_fps(true);
```

The **error overlay** (`ErrorOverlay`) is separate from the `debug` global. In debug builds,
any runtime error is automatically shown as a red bar at the top of the screen, with
the full message printed to stderr. No script code is needed to enable it.

---

## Game Table Format

The entry script must return (Lua) or `export =` (TypeScript) a game table:

**Lua:**
```lua
local game = {}
function game.init() ... end
function game.update(dt) ... end
function game.render() ... end
return game
```

**TypeScript:**
```typescript
const game: Game = {
    init() { ... },
    update(dt: number) { ... },
    render() { ... },
};
export = game;
```

All three keys are optional.

## Scene Table Format

A scene is any table with some or all of these functions:

**Lua:**
```lua
local scene = {
    on_enter = function() ... end,
    update   = function(dt) ... end,
    render   = function() ... end,
    on_exit  = function() ... end,
}
```

**TypeScript:**
```typescript
const scene: Scene = {
    on_enter() { ... },
    update(dt: number) { ... },
    render() { ... },
    on_exit() { ... },
};
```

All four keys are optional. When a scene is active via `engine.set_scene()` or
`engine.switch_scene()`, the scene's `update`/`render` replace `game.update`/`game.render`.
