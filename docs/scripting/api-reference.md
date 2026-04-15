# API Reference

All globals available to scripts running inside `ScriptedGame`. Every global is
registered before `game.init()` is called.

TypeScript type declarations live at `crates/unison-scripting/types/`. The signatures
below match those declarations exactly.

---

## unison.assets

Asset loading service.

#### unison.assets.load_texture

Load a texture from embedded assets. Returns a texture ID integer (0 on error).
Call in `init` or `on_enter`.

**Lua:**
```lua
local tex = unison.assets.load_texture("textures/player.png")
```

**TypeScript:**
```typescript
const tex: TextureId = unison.assets.load_texture("textures/player.png");
```

#### unison.assets.load_sound

Load an audio file from embedded assets. Returns a `SoundId` integer (0 on error).
Call in `init` or `on_enter`.

**Lua:**
```lua
local snd = unison.assets.load_sound("sfx/jump.ogg")
```

**TypeScript:**
```typescript
const snd: SoundId = unison.assets.load_sound("sfx/jump.ogg");
```

---

## unison.renderer

Renderer configuration and screen info.

#### unison.renderer.screen_size

Current screen dimensions in logical points.

**Lua:**
```lua
local w, h = unison.renderer.screen_size()
```

**TypeScript:**
```typescript
const [w, h] = unison.renderer.screen_size();
```

#### unison.renderer.anti_aliasing

Current anti-aliasing mode, or nil if not set.

**Lua:**
```lua
local mode = unison.renderer.anti_aliasing()
```

**TypeScript:**
```typescript
const mode = unison.renderer.anti_aliasing();
```

#### unison.renderer.set_anti_aliasing

Request AA mode for this session. Applied after `init` returns.

**Lua:**
```lua
unison.renderer.set_anti_aliasing("msaa4x")  -- "none", "msaa2x", "msaa4x", "msaa8x"
```

**TypeScript:**
```typescript
unison.renderer.set_anti_aliasing("msaa4x");  // "none" | "msaa2x" | "msaa4x" | "msaa8x"
```

#### unison.renderer.create_target

Create an offscreen render target. Returns the target ID and its associated texture ID.
Call in `init` or `on_enter`.

**Lua:**
```lua
local target_id, texture_id = unison.renderer.create_target(512, 512)
```

**TypeScript:**
```typescript
const [target_id, texture_id] = unison.renderer.create_target(512, 512);
```

---

## unison.input

Input state, refreshed automatically before each `update`.

### Raw Input

#### unison.input.is_key_pressed

`true` while the key is held down.

**Lua:**
```lua
if unison.input.is_key_pressed("ArrowRight") then ... end
```

**TypeScript:**
```typescript
if (unison.input.is_key_pressed("ArrowRight")) { ... }
```

#### unison.input.is_key_just_pressed

`true` only on the frame the key was first pressed.

**Lua:**
```lua
if unison.input.is_key_just_pressed("Space") then ... end
```

**TypeScript:**
```typescript
if (unison.input.is_key_just_pressed("Space")) { ... }
```

#### unison.input.is_key_just_released

`true` only on the frame the key was released.

**Lua:**
```lua
if unison.input.is_key_just_released("Space") then ... end
```

**TypeScript:**
```typescript
if (unison.input.is_key_just_released("Space")) { ... }
```

#### unison.input.touches_started

Array of new touch-start positions this frame.

**Lua:**
```lua
local touches = unison.input.touches_started()  -- [{x, y}, ...]
for _, t in ipairs(touches) do
    unison.debug.log(t.x, t.y)
end
```

**TypeScript:**
```typescript
const touches: TouchPosition[] = unison.input.touches_started();
for (const t of touches) {
    unison.debug.log(t.x, t.y);
}
```

#### unison.input.is_mouse_button_pressed

`true` while a mouse button is held. Button: 0=Left, 1=Right, 2=Middle.

**Lua:**
```lua
if unison.input.is_mouse_button_pressed(0) then ... end
```

**TypeScript:**
```typescript
if (unison.input.is_mouse_button_pressed(0)) { ... }
```

#### unison.input.is_mouse_button_just_pressed

`true` on the frame a mouse button was first pressed.

**Lua:**
```lua
if unison.input.is_mouse_button_just_pressed(0) then ... end
```

**TypeScript:**
```typescript
if (unison.input.is_mouse_button_just_pressed(0)) { ... }
```

#### unison.input.is_mouse_button_just_released

`true` on the frame a mouse button was released.

**Lua:**
```lua
if unison.input.is_mouse_button_just_released(0) then ... end
```

**TypeScript:**
```typescript
if (unison.input.is_mouse_button_just_released(0)) { ... }
```

#### unison.input.mouse_position

Current mouse position in screen space.

**Lua:**
```lua
local mx, my = unison.input.mouse_position()
```

**TypeScript:**
```typescript
const [mx, my] = unison.input.mouse_position();
```

#### unison.input.is_pointer_just_pressed

Cross-platform tap/click detector: `true` if a touch began this frame OR the primary
(left) mouse button was just pressed.

**Lua:**
```lua
if unison.input.is_pointer_just_pressed() then ... end
```

**TypeScript:**
```typescript
if (unison.input.is_pointer_just_pressed()) { ... }
```

#### unison.input.pointer_position

Cross-platform "pointer is currently held" position. Returns the active touch or mouse
position. Returns `nil, nil` when no pointer is active.

**Lua:**
```lua
local px, py = unison.input.pointer_position()
if px then ... end
```

**TypeScript:**
```typescript
const [px, py] = unison.input.pointer_position();
if (px !== undefined) { ... }
```

**Key name strings:**
- Arrow keys: `"ArrowUp"`, `"ArrowDown"`, `"ArrowLeft"`, `"ArrowRight"`
- Common: `"Space"`, `"Enter"`, `"Escape"`, `"Tab"`, `"Backspace"`
- Modifiers: `"ShiftLeft"`, `"ShiftRight"`, `"ControlLeft"`, `"ControlRight"`, `"AltLeft"`, `"AltRight"`
- Letters: `"A"` -- `"Z"` (uppercase)
- Digits: `"0"` -- `"9"` or `"Digit0"` -- `"Digit9"`

### Action Map

Bind named actions and axes for cross-platform input handling.

#### unison.input.bind_action

Bind a named action to a set of keys and/or mouse buttons.

**Lua:**
```lua
unison.input.bind_action("jump", { keys = {"Space"} })
unison.input.bind_action("fire", { keys = {"Z"}, mouse_buttons = {0} })
```

**TypeScript:**
```typescript
unison.input.bind_action("jump", { keys: ["Space"] });
unison.input.bind_action("fire", { keys: ["Z"], mouse_buttons: [0] });
```

#### unison.input.bind_axis

Bind a named axis to negative/positive keys and/or a joystick axis.

**Lua:**
```lua
unison.input.bind_axis("move", { negative = "ArrowLeft", positive = "ArrowRight" })
```

**TypeScript:**
```typescript
unison.input.bind_axis("move", { negative: "ArrowLeft", positive: "ArrowRight" });
```

#### unison.input.is_action_pressed

`true` while any input bound to `name` is held.

**Lua:**
```lua
if unison.input.is_action_pressed("jump") then ... end
```

**TypeScript:**
```typescript
if (unison.input.is_action_pressed("jump")) { ... }
```

#### unison.input.is_action_just_pressed

`true` only on the frame the action was first triggered.

**Lua:**
```lua
if unison.input.is_action_just_pressed("jump") then ... end
```

**TypeScript:**
```typescript
if (unison.input.is_action_just_pressed("jump")) { ... }
```

#### unison.input.is_action_just_released

`true` only on the frame all bound inputs were released.

**Lua:**
```lua
if unison.input.is_action_just_released("jump") then ... end
```

**TypeScript:**
```typescript
if (unison.input.is_action_just_released("jump")) { ... }
```

#### unison.input.axis

Digital axis value in [-1, 1] from negative/positive actions, plus raw joystick if bound.

**Lua:**
```lua
local ax = unison.input.axis("move")
```

**TypeScript:**
```typescript
const ax: number = unison.input.axis("move");
```

---

## unison.scenes

Scene management service.

#### unison.scenes.set

Activate a scene. Calls `scene.on_enter()` if present. The scene's `update`/`render`
replace `game.update`/`game.render` from this point on.

**Lua:**
```lua
unison.scenes.set(scene_table)
```

**TypeScript:**
```typescript
unison.scenes.set(scene);
```

#### unison.scenes.current

Returns the current scene table, or nil if no scene is active.

**Lua:**
```lua
local s = unison.scenes.current()
```

**TypeScript:**
```typescript
const s = unison.scenes.current();
```

---

## unison.events

String-keyed pub/sub event bus.

#### unison.events.on

Register a callback for a named event. Multiple listeners are allowed.

**Lua:**
```lua
unison.events.on("level_complete", function(data)
    unison.debug.log("done!", data.score)
end)
```

**TypeScript:**
```typescript
unison.events.on("level_complete", (data) => {
    unison.debug.log("done!", data.score);
});
```

#### unison.events.emit

Emit a named event with an optional data payload. Callbacks fire at end of frame.

**Lua:**
```lua
unison.events.emit("level_complete", { score = 1234 })
```

**TypeScript:**
```typescript
unison.events.emit("level_complete", { score: 1234 });
```

#### unison.events.clear

Clear all string-keyed event handlers and pending events. Call in `on_exit`.

**Lua:**
```lua
unison.events.clear()
```

**TypeScript:**
```typescript
unison.events.clear();
```

---

## unison.audio

Music, SFX, and mix-bus service. See [api/audio.md](../api/audio.md) for the subsystem deep-dive (buses, spatial attenuation, web gesture gating, platform notes).

Sound loading lives on [`unison.assets.load_sound`](#unisonassetsload_sound).

#### unison.audio.unload

Free a previously loaded sound.

**Lua:**
```lua
unison.audio.unload(snd)
```

**TypeScript:**
```typescript
unison.audio.unload(snd);
```

#### unison.audio.play

Play a non-positional sound. Returns a `PlaybackId` (0 if the call was deferred by the web pre-arm queue).

**Lua:**
```lua
local pb = unison.audio.play(snd, { volume = 0.8, pitch = 1.1, looping = false, bus = "sfx", fade_in = 0.05 })
```

**TypeScript:**
```typescript
const pb: PlaybackId = unison.audio.play(snd, { volume: 0.8, pitch: 1.1, looping: false, bus: "sfx", fade_in: 0.05 });
```

#### unison.audio.stop

Stop a playback, optionally fading out over `fade_out` seconds.

**Lua:**
```lua
unison.audio.stop(pb, { fade_out = 0.25 })
```

**TypeScript:**
```typescript
unison.audio.stop(pb, { fade_out: 0.25 });
```

#### unison.audio.pause / unison.audio.resume

Pause or resume a playback (its handle stays valid).

**Lua:**
```lua
unison.audio.pause(pb)
unison.audio.resume(pb)
```

**TypeScript:**
```typescript
unison.audio.pause(pb);
unison.audio.resume(pb);
```

#### unison.audio.is_playing

`true` while the playback is still producing audio.

**Lua:**
```lua
if unison.audio.is_playing(pb) then ... end
```

**TypeScript:**
```typescript
if (unison.audio.is_playing(pb)) { ... }
```

#### unison.audio.set_volume / unison.audio.set_pitch

Update a playback's volume or pitch, optionally tweened over `tween` seconds.

**Lua:**
```lua
unison.audio.set_volume(pb, 0.3, { tween = 0.5 })
unison.audio.set_pitch(pb, 1.25, { tween = 0.2 })
```

**TypeScript:**
```typescript
unison.audio.set_volume(pb, 0.3, { tween: 0.5 });
unison.audio.set_pitch(pb, 1.25, { tween: 0.2 });
```

#### unison.audio.play_music

Start a music track. Exclusive — replaces the current track, with optional `crossfade` in seconds. Music loops by default.

**Lua:**
```lua
local music = unison.audio.play_music(track, { volume = 0.7, crossfade = 1.5 })
```

**TypeScript:**
```typescript
const music: PlaybackId = unison.audio.play_music(track, { volume: 0.7, crossfade: 1.5 });
```

#### unison.audio.stop_music / pause_music / resume_music / current_music

Control the currently-active music track.

**Lua:**
```lua
unison.audio.stop_music({ fade_out = 1.0 })
unison.audio.pause_music()
unison.audio.resume_music()
local pb = unison.audio.current_music()  -- nil if none
```

**TypeScript:**
```typescript
unison.audio.stop_music({ fade_out: 1.0 });
unison.audio.pause_music();
unison.audio.resume_music();
const pb: PlaybackId | undefined = unison.audio.current_music();
```

#### unison.audio.set_master_volume

Set the master output volume. Applies on top of all bus volumes.

**Lua:**
```lua
unison.audio.set_master_volume(0.6, { tween = 0.5 })
```

**TypeScript:**
```typescript
unison.audio.set_master_volume(0.6, { tween: 0.5 });
```

#### unison.audio.set_bus_volume

Set the volume of a named bus. Unknown bus names are a no-op.

**Lua:**
```lua
unison.audio.set_bus_volume("music", 0.4, { tween = 0.25 })
```

**TypeScript:**
```typescript
unison.audio.set_bus_volume("music", 0.4, { tween: 0.25 });
```

#### unison.audio.create_bus

Create a named bus. Idempotent — repeated calls with the same name are no-ops. Built-in buses (`"master"`, `"music"`, `"sfx"`) are always present.

**Lua:**
```lua
unison.audio.create_bus("ui")
```

**TypeScript:**
```typescript
unison.audio.create_bus("ui");
```

#### unison.audio.stop_all

Stop every non-spatial playback, optionally fading out. Spatial (world-scoped) playbacks are stopped via `world:clear_sounds` — see below.

**Lua:**
```lua
unison.audio.stop_all({ fade_out = 0.5 })
```

**TypeScript:**
```typescript
unison.audio.stop_all({ fade_out: 0.5 });
```

### Spatial audio (world-scoped)

Spatial sounds are attached to a specific `World` so that `world:clear_sounds` stops only that world's voices. See [api/audio.md](../api/audio.md) for the V1 attenuation caveats (static at play time in the kira backend).

#### world:play_sound_at / world.play_sound_at

Play a positional sound at world position `(x, y)`. Returns a `PlaybackId`.

**Lua:**
```lua
local pb = world:play_sound_at(snd, 4.0, 2.0, {
    volume = 0.9, pitch = 1.0, looping = false,
    max_distance = 25.0, rolloff = "inverse", bus = "sfx",
})
```

**TypeScript:**
```typescript
const pb: PlaybackId = world.play_sound_at(snd, 4.0, 2.0, {
    volume: 0.9, pitch: 1.0, looping: false,
    max_distance: 25.0, rolloff: "inverse", bus: "sfx",
});
```

**Rolloff values:** `"inverse"` (inverse-square, default), `"linear"`.

#### world:set_sound_position / world.set_sound_position

Update a spatial playback's position. Forward-compat: under the V1 kira backend this does not currently change the audible output (see [api/audio.md](../api/audio.md)).

**Lua:**
```lua
world:set_sound_position(pb, x, y)
```

**TypeScript:**
```typescript
world.set_sound_position(pb, x, y);
```

#### world:clear_sounds / world.clear_sounds

Stop every spatial playback belonging to this world, optionally fading out. Call in `on_exit` or when tearing down a scene.

**Lua:**
```lua
world:clear_sounds({ fade_out = 0.3 })
```

**TypeScript:**
```typescript
world.clear_sounds({ fade_out: 0.3 });
```

---

## unison.UI

Declarative UI factory.

#### unison.UI.new

Create a UI handle using the given font asset. Reuse the handle across frames.

**Lua:**
```lua
local ui = unison.UI.new("fonts/DejaVuSans-Bold.ttf")
```

**TypeScript:**
```typescript
const ui: UI = unison.UI.new("fonts/DejaVuSans-Bold.ttf");
```

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
clicked. Listen with `unison.events.on("my_button", callback)`.

See [api/ui.md](../api/ui.md) for full node property reference.

---

## unison.debug

Development utilities. Available in all builds, but intended for debug use.

#### unison.debug.log

Print varargs to stderr, joined with tabs. Values are converted via `tostring`.

**Lua:**
```lua
unison.debug.log("player pos:", x, y)
```

**TypeScript:**
```typescript
unison.debug.log("player pos:", x, y);
```

#### unison.debug.draw_point

Draw a 0.1-unit point at world position `(x, y)`. Color is a hex integer.

**Lua:**
```lua
unison.debug.draw_point(x, y, 0xFF0000)
```

**TypeScript:**
```typescript
unison.debug.draw_point(x, y, 0xFF0000);
```

#### unison.debug.show_physics

Toggle physics debug visualization. (Not yet wired.)

**Lua:**
```lua
unison.debug.show_physics(true)
```

**TypeScript:**
```typescript
unison.debug.show_physics(true);
```

#### unison.debug.show_fps

Toggle FPS counter overlay. (Not yet wired.)

**Lua:**
```lua
unison.debug.show_fps(true)
```

**TypeScript:**
```typescript
unison.debug.show_fps(true);
```

The **error overlay** (`ErrorOverlay`) is separate from the `debug` global. In debug builds,
any runtime error is automatically shown as a red bar at the top of the screen, with
the full message printed to stderr. No script code is needed to enable it.

---

## unison.math

Math utility extensions.

#### unison.math.lerp

Linear interpolation: `a + (b - a) * t`.

**Lua:**
```lua
local v = unison.math.lerp(0, 100, 0.5)
```

**TypeScript:**
```typescript
const v: number = unison.math.lerp(0, 100, 0.5);
```

#### unison.math.smoothstep

Smooth Hermite interpolation, clamped to [0, 1].

**Lua:**
```lua
local v = unison.math.smoothstep(0, 1, x)
```

**TypeScript:**
```typescript
const v: number = unison.math.smoothstep(0, 1, x);
```

#### unison.math.clamp

Clamp `x` to `[min, max]`.

**Lua:**
```lua
local v = unison.math.clamp(x, 0, 1)
```

**TypeScript:**
```typescript
const v: number = unison.math.clamp(x, 0, 1);
```

All standard Lua `math.*` functions are available as usual.

---

## unison.World

Physics world containing objects, cameras, lighting, and rendering. Each scene
typically creates its own World.

### Constructor

#### unison.World.new

Create a new World. Default: `"main"` camera, gravity -9.8.

**Lua:**
```lua
local world = unison.World.new()
```

**TypeScript:**
```typescript
const world: World = unison.World.new();
```

### World Configuration

| Method | Description |
|--------|-------------|
| `world:set_background(hex)` | Set background clear color (e.g. `0x1a1a2e`) |
| `world:set_gravity(g)` | Set gravity (negative = downward, e.g. `-9.8`) |
| `world:set_ground(y)` | Add a flat ground plane at world Y |
| `world:set_ground_restitution(r)` | Ground bounciness: 0 = no bounce, 1 = perfect elastic |
| `world:set_ground_friction(f)` | Ground friction: 0 = frictionless, 1 = sticky |

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

#### world:render / world.render

Render all objects and lighting through the main camera. Call in `render`.

**Lua:**
```lua
world:render()
```

**TypeScript:**
```typescript
world.render();
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

#### world:draw_overlay / world.draw_overlay

Composite a render-target texture onto the screen. Coordinates are in screen-space.

**Lua:**
```lua
world:draw_overlay(texture_id, x, y, w, h)
```

**TypeScript:**
```typescript
world.draw_overlay(texture_id, x, y, w, h);
```

#### world:draw_overlay_bordered / world.draw_overlay_bordered

Like `draw_overlay` with a colored border. `border_color` is a hex integer.

**Lua:**
```lua
world:draw_overlay_bordered(texture_id, x, y, w, h, border_width, border_color)
```

**TypeScript:**
```typescript
world.draw_overlay_bordered(texture_id, x, y, w, h, border_width, border_color);
```

### Collision Callbacks

#### world:on_collision / world.on_collision

Called for every collision pair each frame.

**Lua:**
```lua
world:on_collision(function(a, b, info)
    unison.debug.log(info.normal_x, info.normal_y)
end)
```

**TypeScript:**
```typescript
world.on_collision((a: ObjectId, b: ObjectId, info: CollisionInfo) => {
    unison.debug.log(info.normal_x, info.normal_y);
});
```

#### world:on_collision_with / world.on_collision_with

Called when the given object collides with anything.

**Lua:**
```lua
world:on_collision_with(player_id, function(other, info)
    unison.debug.log("hit", other)
end)
```

**TypeScript:**
```typescript
world.on_collision_with(player_id, (other: ObjectId, info: CollisionInfo) => {
    unison.debug.log("hit", other);
});
```

#### world:on_collision_between / world.on_collision_between

Called when objects `a` and `b` collide.

**Lua:**
```lua
world:on_collision_between(player_id, spike_id, function(info)
    unison.debug.log("ouch!")
end)
```

**TypeScript:**
```typescript
world.on_collision_between(player_id, spike_id, (info: CollisionInfo) => {
    unison.debug.log("ouch!");
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

## world.objects

Object management facade. All spawn functions return an integer **object ID**.

### Spawning

#### world.objects:spawn_soft_body

Spawn a deformable soft body. Returns an object ID.

**Lua:**
```lua
local id = world.objects:spawn_soft_body({
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
const id: ObjectId = world.objects.spawn_soft_body({
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

#### world.objects:spawn_rigid_body

Spawn a rigid body with AABB or circle collider. Returns an object ID.

**Lua:**
```lua
local id = world.objects:spawn_rigid_body({
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
const id: ObjectId = world.objects.spawn_rigid_body({
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

#### world.objects:spawn_static_rect

Spawn an immovable rectangle. Returns an object ID.

**Lua:**
```lua
local id = world.objects:spawn_static_rect({ position = {-3, 0}, size = {6, 0.5}, color = 0x333333 })
```

**TypeScript:**
```typescript
const id: ObjectId = world.objects.spawn_static_rect({ position: [-3, 0], size: [6, 0.5], color: 0x333333 });
```

#### world.objects:spawn_sprite

Spawn a visual-only sprite (no physics). Returns an object ID.

**Lua:**
```lua
local id = world.objects:spawn_sprite({
    texture = tex_id,
    position = {0, 5},
    size = {2, 2},
    rotation = 0,
    color = 0xFFFFFF,
})
```

**TypeScript:**
```typescript
const id: ObjectId = world.objects.spawn_sprite({
    texture: tex_id,
    position: [0, 5],
    size: [2, 2],
    rotation: 0,
    color: 0xFFFFFF,
});
```

#### world.objects:despawn

Remove an object from the world.

**Lua:**
```lua
world.objects:despawn(id)
```

**TypeScript:**
```typescript
world.objects.despawn(id);
```

### Physics Interaction

| Method | Description |
|--------|-------------|
| `world.objects:apply_force(id, fx, fy)` | Apply a continuous force each frame (call in `update`) |
| `world.objects:apply_impulse(id, ix, iy)` | Apply an instantaneous velocity change |
| `world.objects:apply_torque(id, torque)` | Apply rotational torque |

### Queries

| Method | Description |
|--------|-------------|
| `world.objects:position(id) → x, y` | Get object center position |
| `world.objects:velocity(id) → vx, vy` | Get object velocity |
| `world.objects:is_grounded(id) → bool` | True if object is resting on ground plane |
| `world.objects:is_touching(a, b) → bool` | True if objects `a` and `b` are in contact |

### Display Properties

| Method | Description |
|--------|-------------|
| `world.objects:set_z_order(id, z)` | Set draw order (higher = on top) |
| `world.objects:set_casts_shadow(id, bool)` | Enable/disable shadow casting |
| `world.objects:set_position(id, x, y)` | Teleport object to exact position |

---

## world.cameras

Camera management facade. A `"main"` camera is created automatically.

| Method | Description |
|--------|-------------|
| `world.cameras:add(name, width, height)` | Add a named camera with viewport size in world units |
| `world.cameras:follow(name, id, opts?)` | Follow an object; `opts.smoothing` 0=frozen, 1=instant; `opts.offset=[ox,oy]` |
| `world.cameras:unfollow(name)` | Stop following |
| `world.cameras:position(name) → x, y` | Get camera center position |
| `world.cameras:screen_to_world(sx, sy) → wx, wy` | Convert screen-space to world-space using the main camera |

**Example:**

**Lua:**
```lua
world.cameras:follow("main", player_id, { smoothing = 0.1 })
```

**TypeScript:**
```typescript
world.cameras.follow("main", player_id, { smoothing: 0.1 });
```

---

## world.lights

Lighting management facade.

### System Configuration

| Method | Description |
|--------|-------------|
| `world.lights:set_enabled(bool)` | Enable or disable the entire lighting system |
| `world.lights:set_ambient(r, g, b, a)` | Set ambient light color as RGBA floats [0,1] |
| `world.lights:set_ground_shadow(y or nil)` | Ground shadow plane at Y, or nil/false to disable |

### Point Lights

#### world.lights:add_point

Add a point light. Returns a light handle.

**Lua:**
```lua
local light = world.lights:add_point({
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
const light: LightId = world.lights.add_point({
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

| Method | Description |
|--------|-------------|
| `world.lights:set_intensity(handle, intensity)` | Update light intensity (multiplier) |
| `world.lights:follow(handle, id, opts?)` | Track object each frame; `opts.offset=[ox,oy]` |
| `world.lights:unfollow(handle)` | Stop tracking |

### Directional Lights

#### world.lights:add_directional

Add a directional light. Returns a light handle.

**Lua:**
```lua
local light = world.lights:add_directional({
    direction = {-0.5, -1.0},
    color = 0xFFFFFF,
    intensity = 0.8,
    casts_shadows = true,
    shadow = { filter = "pcf5", distance = 6.0, strength = 0.7 },
})
```

**TypeScript:**
```typescript
const light: LightId = world.lights.add_directional({
    direction: [-0.5, -1.0],
    color: 0xFFFFFF,
    intensity: 0.8,
    casts_shadows: true,
    shadow: { filter: "pcf5", distance: 6.0, strength: 0.7 },
});
```

| Method | Description |
|--------|-------------|
| `world.lights:set_direction(handle, dx, dy)` | Update directional light direction |

---

## Render Layers

Named render layers with independent lighting and clear settings.

| Method | Description |
|--------|-------------|
| `world:create_render_layer(name, desc) → handle` | Create a named layer |
| `world:create_render_layer_before(name, before, desc) → handle` | Insert before existing layer |
| `world:set_layer_clear_color(handle, hex)` | Update layer clear color |
| `world:default_layer() → handle` | Get the default scene layer |
| `world:draw_to(layer, shape, params, z)` | Draw shape to specific layer |
| `world:draw(shape, params, z)` | Draw to default layer |
| `world:draw_unlit(shape, params, z)` | Draw unlit (not affected by lightmap) |

**Layer descriptor:** `{ lit = false, clear_color = 0x020206 }`

**Shape types and param tables:**

| Shape | Params |
|-------|--------|
| `"rect"` | `x, y, width, height, color` |
| `"line"` | `x1, y1, x2, y2, color, width?` |
| `"circle"` | `x, y, radius, color` |
| `"gradient_circle"` | `x, y, radius, color` |

Color can be a hex integer or an `[r, g, b]` / `[r, g, b, a]` float array.

---

## unison.Color

Color userdata with RGBA components and interpolation.

#### unison.Color.hex

Create a Color from a hex integer.

**Lua:**
```lua
local c = unison.Color.hex(0xFF8800)
```

**TypeScript:**
```typescript
const c: Color = unison.Color.hex(0xFF8800);
```

#### unison.Color.rgba

Create from RGBA floats in [0, 1].

**Lua:**
```lua
local c = unison.Color.rgba(1.0, 0.5, 0.0, 1.0)
```

**TypeScript:**
```typescript
const c: Color = unison.Color.rgba(1.0, 0.5, 0.0, 1.0);
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

## unison.Rng

Deterministic pseudo-random number generator (xorshift64).

#### unison.Rng.new

Create a new RNG with the given seed. Seed 0 is treated as 1.

**Lua:**
```lua
local rng = unison.Rng.new(42)
```

**TypeScript:**
```typescript
const rng: Rng = unison.Rng.new(42);
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

All four keys are optional. When a scene is active via `unison.scenes.set()`, the
scene's `update(dt)`/`render` replace `game.update`/`game.render`.
