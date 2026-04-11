# Unison 2D — API Reference

Complete type and method reference for the Unison 2D engine. For tutorials and patterns, see the [User Guide](guide/README.md).

For per-crate deep dives, see the [api/](api/) directory.

---

## Architecture

```
Game (your struct)
├── Engine<A>        — input/actions, renderer, compositing, assets
├── World            — physics, objects, cameras, lighting, environment
│   ├── ObjectSystem   — soft bodies, rigid bodies, sprites
│   ├── CameraSystem
│   ├── LightingSystem — point lights, directional lights, shadows
│   └── Environment    — background color
└── Level<S> (trait) — optional scene abstraction with shared state
    └── Ctx<S>           — unified context (input, renderer, events, dt, shared)
```

- **Engine** is a thin shell — only input mapping, renderer access, and asset store
- **World** is where all simulation lives — games own one or more Worlds
- **Level** is an optional trait for organizing scenes with lifecycle hooks

## Game Trait

```rust
pub trait Game {
    type Action: Copy + Eq + Hash + 'static;
    fn init(&mut self, engine: &mut Engine<Self::Action>);    // called once
    fn update(&mut self, engine: &mut Engine<Self::Action>);  // per fixed timestep (60Hz)
    fn render(&mut self, engine: &mut Engine<Self::Action>);  // once per frame (required)
}
```

- Define your `Action` enum for input mapping
- `update()` runs at fixed 60Hz — step your world(s) here
- `render()` is required — game controls all rendering

### Scripted Games (Lua)

For Lua scripting, use `ScriptedGame` from the `unison-scripting` crate instead of implementing `Game` directly. See [api/scripting.md](api/scripting.md) for the full Lua API reference.

```lua
local world = World.new()
world:set_gravity(-9.8)
local id = world:spawn_soft_body({mesh="ring", mesh_params={1,0.25,24,8}, material="rubber", position={0,5}})
world:camera_follow("main", id, 0.08)
```

Key globals: `World`, `engine`, `input` — all documented in the scripting reference.

## World

Self-contained simulation owning physics, objects, cameras, and environment.

```rust
let mut world = World::new();
world.set_background(Color::from_hex(0x1a1a2e));
world.objects.set_gravity(-9.8);
world.objects.set_ground(-5.0);
```

| Method | Description |
|--------|-------------|
| `new()` | Default world (main camera 26.67×15) |
| `set_background(color)` | Set clear color (convenience for `environment.background_color`) |
| `background_color()` | Get clear color |
| `step(dt)` | Advance physics + update camera/light follows |
| `create_render_layer(name, config)` | Create a named render layer, returns `RenderLayerId` |
| `create_render_layer_before(name, config, before)` | Create a layer inserted before another layer |
| `default_layer()` | Get the default scene layer ID |
| `set_layer_clear_color(layer, color)` | Update a layer's clear color |
| `draw_to(layer, command, z_order)` | Queue a render command to a specific layer |
| `draw(command, z_order)` | Queue to the default scene layer (sorted with objects, affected by lighting) |
| `draw_unlit(command, z_order)` | Queue a world-space render command drawn after all layers (not darkened) |
| `draw_overlay(command, z_order)` | Queue a screen-space overlay command (after all layers) |
| `light_follow(light, object)` | Make a point light follow an object each step |
| `light_follow_with_offset(light, object, offset)` | Follow with a fixed Vec2 offset |
| `set_light_follow_offset(light, offset)` | Change offset on an already-following light |
| `light_unfollow(light)` | Stop following |
| `auto_render(renderer)` | Render all layers through "main" camera |
| `render_to_targets(renderer, &[(&str, RenderTargetId)])` | Multi-camera render with layers |
| `spawn_soft_body(SoftBodyDesc)` | Spawn a soft body |
| `spawn_rigid_body(RigidBodyDesc)` | Spawn a rigid body |
| `spawn_static_rect(position, size, color)` | Spawn a static rectangle |
| `spawn_sprite(SpriteDesc)` | Spawn a sprite (no physics) |
| `despawn(id)` | Despawn any object |

### Environment

Rendering configuration, accessible via `world.environment`:

```rust
world.environment.background_color = Color::from_hex(0x1a1a2e);
// Or use the convenience method:
world.set_background(Color::from_hex(0x1a1a2e));
```

## Creating Objects

Spawn objects through `world.spawn_*()`. The World routes each object to the right subsystem(s).

### Soft Bodies

```rust
let mesh = create_ring_mesh(outer_radius, inner_radius, segments, radial_divs);
let id = world.spawn_soft_body(SoftBodyDesc {
    mesh,
    material: Material::RUBBER,
    position: Vec2::new(x, y),
    color: Color::from_hex(0xd4943a),
    texture: TextureId::NONE,  // or a loaded TextureId
});
```

**Mesh presets** (from `unison2d::physics::mesh`):
- `create_ring_mesh(outer_r, inner_r, segments, radial_divs)` — donut/ring
- `create_square_mesh(size, divisions)` — square
- `create_ellipse_mesh(rx, ry, segments, rings)` — ellipse
- `create_star_mesh(outer_r, inner_r, points, rings)` — star
- `create_blob_mesh(radius, segments, rings)` — organic blob
- `create_rounded_box_mesh(width, height, corner_radius, corner_segments)` — rounded rectangle

**Materials** (from `unison2d::physics::Material`):
- `Material::SLIME` — ultra-soft, blobby
- `Material::JELLO` — soft, jiggly
- `Material::RUBBER` — bouncy (default)
- `Material::WOOD` — stiff
- `Material::METAL` — nearly rigid
- Custom: `Material::new(density, edge_compliance, area_compliance)`

### Rigid Bodies

```rust
let id = world.spawn_rigid_body(RigidBodyDesc {
    collider: Collider::aabb(half_width, half_height),
    position: Vec2::new(x, y),
    color: Color::from_hex(0x4a3728),
    is_static: true,
});
```

**Colliders** (from `unison2d::physics::Collider`):
- `Collider::aabb(half_width, half_height)` — rectangle
- `Collider::circle(radius)` — circle

### Sprites (no physics)

Purely visual objects — a textured or colored quad with a transform.

```rust
let id = world.spawn_sprite(SpriteDesc {
    texture: TextureId::NONE,       // solid color (or a loaded texture)
    position: Vec2::new(x, y),
    size: Vec2::new(2.0, 2.0),
    rotation: 0.0,
    color: Color::from_hex(0xff9f43),
});

// Move/rotate sprites directly (through the ObjectSystem):
world.objects.set_sprite_position(id, Vec2::new(3.0, 4.0));
world.objects.set_sprite_rotation(id, 0.5);
let pos = world.objects.get_sprite_position(id);  // -> Option<Vec2>
```

### Static Rectangles (convenience)

```rust
let id = world.spawn_static_rect(position, size, color);
```

### Despawn

```rust
world.despawn(id);
```

## Prefab Trait

A lightweight trait for reusable object spawning templates.

```rust
use unison2d::Prefab;

struct EnemyPrefab;

impl Prefab for EnemyPrefab {
    fn spawn(&self, world: &mut World, position: Vec2) -> ObjectId {
        world.spawn_soft_body(SoftBodyDesc {
            mesh: create_square_mesh(0.8, 3),
            material: Material::RUBBER,
            position,
            color: Color::from_hex(0xe74c3c),
            texture: TextureId::NONE,
        })
    }
}

// Usage:
let enemy = EnemyPrefab.spawn(&mut world, Vec2::new(5.0, 3.0));
```

## Physics & Movement

```rust
world.objects.apply_force(id, Vec2::new(10.0, 0.0));   // continuous, call each frame
world.objects.apply_torque(id, -5.0, dt);               // continuous rotation
world.objects.apply_impulse(id, Vec2::new(0.0, 8.0));   // instantaneous velocity change
world.objects.set_velocity(id, Vec2::new(5.0, 0.0));    // set velocity directly
let pos = world.objects.get_position(id);                // get current position
world.objects.set_position(id, Vec2::new(0.0, 0.0));    // teleport
let vel = world.objects.get_velocity(id);                // get current velocity
let grounded = world.objects.is_grounded(id);            // touching ground?
```

### Physics Configuration

```rust
world.objects.set_gravity(-9.8);                   // gravity magnitude (applied in -Y direction)
world.objects.set_ground(-5.0);                    // flat ground at y=-5
world.objects.clear_ground();                      // remove ground
world.objects.set_ground_friction(0.8);            // 0=ice, 1=sticky
world.objects.set_ground_restitution(0.3);         // 0=no bounce, 1=perfect bounce
```

### Raw Physics Access

```rust
world.objects.physics()       // -> &PhysicsWorld
world.objects.physics_mut()   // -> &mut PhysicsWorld
```

## Input & Actions

Input bindings live on the Engine. World/objects are separate.

### Define Actions

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
enum Action { MoveLeft, MoveRight, Jump, Dash }
```

### Bind Inputs

```rust
engine.bind_key(KeyCode::ArrowLeft, Action::MoveLeft);
engine.bind_key(KeyCode::Space, Action::Jump);
engine.bind_mouse_button(MouseButton::Left, Action::Shoot);
```

### Query Actions

```rust
engine.action_active(Action::Jump)          // held this frame?
engine.action_just_started(Action::Jump)    // pressed this frame?
engine.action_just_ended(Action::Jump)      // released this frame?
engine.action_axis(Action::MoveLeft, Action::MoveRight)  // -1.0, 0.0, or 1.0
```

### Raw Input (for Levels)

Inside a Level, use `ctx.input` directly:

```rust
ctx.input.is_key_pressed(KeyCode::ArrowLeft)       // held?
ctx.input.is_key_just_pressed(KeyCode::Space)      // just pressed?
ctx.input.axis()                                   // analog axis (Vec2, e.g., joystick)
```

### Available KeyCodes

`ArrowUp`, `ArrowDown`, `ArrowLeft`, `ArrowRight`, `Space`, `Enter`, `Escape`, `Tab`, `Backspace`, `ShiftLeft`, `ShiftRight`, `ControlLeft`, `ControlRight`, `AltLeft`, `AltRight`, `A`-`Z`, `Digit0`-`Digit9`

## Cameras

Cameras live on the world's `CameraSystem`. A default "main" camera (26.67×15) is created automatically.

```rust
world.cameras.follow("main", player_id, 0.08);  // follow with smoothing
world.cameras.follow_with_offset("main", player_id, 0.08, Vec2::new(0.0, 3.0));  // follow with offset
world.cameras.set_follow_offset("main", Vec2::new(0.0, 3.0));  // change offset while following
world.cameras.unfollow("main");                  // stop following
world.cameras.get_mut("main").unwrap().zoom = 2.0;  // zoom in
world.cameras.get_mut("main").unwrap().set_position(x, y);  // manual position

// Add more cameras
world.cameras.add("minimap", Camera::new(100.0, 75.0));

// Iterate
for (name, camera) in world.cameras.iter() { /* ... */ }
```

## Lighting

Lighting lives on the world's `LightingSystem`, accessible via `world.lighting`. See [api/lighting.md](api/lighting.md) for the full deep dive.

### Setup

```rust
world.lighting.set_enabled(true);
world.lighting.set_ambient(Color::new(0.1, 0.1, 0.15, 1.0));
```

### Point Lights

```rust
use unison2d::lighting::{PointLight, ShadowSettings};

let light = world.lighting.add_light(PointLight {
    position: Vec2::new(5.0, 3.0),
    color: Color::new(1.0, 0.9, 0.7, 1.0),
    intensity: 1.0,
    radius: 8.0,
    casts_shadows: true,
    shadow: ShadowSettings::soft(),
});
```

### Directional Lights

```rust
use unison2d::lighting::{DirectionalLight, ShadowSettings, ShadowFilter};

world.lighting.add_directional_light(DirectionalLight {
    direction: Vec2::new(0.5, -1.0),
    color: Color::new(1.0, 0.95, 0.8, 1.0),
    intensity: 0.7,
    casts_shadows: true,
    shadow: ShadowSettings {
        filter: ShadowFilter::Pcf13,
        distance: 12.0,
        attenuation: 4.0,
        ..ShadowSettings::default()
    },
});
```

### Shadow Settings

```rust
ShadowSettings {
    filter: ShadowFilter::None,  // None, Pcf5, Pcf13
    strength: 1.0,               // 0.0=invisible, 1.0=full black
    distance: 0.0,               // max shadow distance (0.0=full radius)
    attenuation: 1.0,            // fade curve (0.0=solid, higher=faster fade)
}

ShadowSettings::hard()   // hard shadows with defaults
ShadowSettings::soft()   // soft shadows with PCF5
```

### Per-Object Shadow Control

```rust
world.objects.set_casts_shadow(id, false);  // disable shadows for this object
world.lighting.set_ground_shadow(Some(-4.5));  // clip shadows at ground Y
```

### Draw Order

```rust
world.objects.set_z_order(id, 1);  // higher values draw on top (default: 0)
```

## Rendering

### Simple (single camera)

```rust
fn render(&mut self, engine: &mut Engine<Action>) {
    if let Some(r) = engine.renderer_mut() {
        self.world.auto_render(r);
    }
}
```

### Multi-Camera with Compositing

```rust
// In init: create render targets
let (target, texture) = engine.create_render_target(800, 600)?;

// In render:
if let Some(r) = engine.renderer_mut() {
    self.world.render_to_targets(r, &[
        ("main", self.main_target),
        ("minimap", self.minimap_target),
    ]);
}

engine.begin_composite(Color::BLACK);
engine.composite_layer(self.main_texture, Rect::from_position(Vec2::ZERO, Vec2::ONE));
engine.composite_layer(self.minimap_texture,
    Rect::from_position(Vec2::new(0.7, 0.7), Vec2::new(0.25, 0.25)));
engine.end_composite();
```

### Custom Drawing

```rust
fn render(&mut self, engine: &mut Engine<Action>) {
    if let Some(r) = engine.renderer_mut() {
        self.world.auto_render(r);
        r.draw(RenderCommand::Line {
            start: [0.0, 0.0],
            end: [5.0, 5.0],
            color: Color::RED,
            width: 0.05,
        });
    }
}
```

### Render Layers

Layers render in creation order. Lit layers are affected by lighting; unlit layers bypass it. Use `create_render_layer_before` to insert layers before the default scene layer.

```rust
// Create an unlit sky layer behind the scene
let sky = world.create_render_layer_before(
    "sky",
    RenderLayerConfig { lit: false, clear_color: sky_color },
    world.default_layer(),
);

world.draw_to(sky, sun_disc, 0);   // sky layer — no shadows
world.draw(tree_mesh, 0);           // default scene layer — lit
```

| Type | Description |
|------|-------------|
| `RenderLayerId` | Handle returned by `create_render_layer` / `create_render_layer_before` |
| `RenderLayerConfig { lit, clear_color }` | Layer configuration: `lit: bool`, `clear_color: Color` |

### Colors

```rust
Color::from_hex(0xff9f43)           // from hex
Color::from_rgba8(255, 159, 67, 255) // from RGBA bytes
Color::WHITE, Color::BLACK, Color::RED, Color::GREEN, Color::BLUE
color.lerp(other, 0.5)              // linear interpolation
```

### Math Utilities

```rust
use unison2d::core::{lerp, smoothstep, Rng};

lerp(0.0, 10.0, 0.5)     // 5.0 — scalar linear interpolation
smoothstep(0.5)           // 0.5 — smooth ease-in/ease-out

let mut rng = Rng::new(42);       // deterministic xorshift32 PRNG
rng.range_f32(0.0, 1.0)           // f32 in [0, 1)
rng.range_u32(1, 7)               // u32 in [1, 7)
```

### Shape Primitives

Factory functions that return `RenderCommand::Mesh` — no renderer changes needed.

```rust
use unison2d::render::primitives::{circle, gradient_circle};

world.draw(circle(center, radius, color), 0);
world.draw(gradient_circle(center, radius, glow_color), 0);  // radial gradient (opaque center, transparent edge)
```

## Assets

Assets are embedded into the binary at build time via `build.rs` and served at runtime through `AssetStore` on the Engine.

### Build Setup

In your game's `build.rs`:

```rust
fn main() {
    unison_assets::build::embed_assets("project/assets", "assets.rs");
}
```

In your game's `Cargo.toml`:

```toml
[build-dependencies]
unison-assets = { path = "unison2d/crates/unison-assets", features = ["build"] }
```

In your game code, include the generated module and load at init:

```rust
mod assets {
    include!(concat!(env!("OUT_DIR"), "/assets.rs"));
}

fn init(&mut self, engine: &mut Engine<Action>) {
    engine.assets_mut().load_embedded(assets::ASSETS);
}
```

### Querying Assets

```rust
engine.assets().get("textures/donut-pink.png")  // -> Option<&[u8]>
engine.assets().contains("textures/player.png")  // -> bool
engine.assets().len()                             // number of loaded assets
engine.assets().paths()                           // iterate all asset paths
```

Asset keys are relative paths from the asset directory root, using forward slashes.

### Decoding Images

`unison2d::render::decode_image` decodes raw image bytes (PNG, JPEG, GIF, BMP, WebP) into a `TextureDescriptor`:

```rust
use unison2d::render::decode_image;

let bytes = engine.assets().get("textures/donut-pink.png").unwrap();
let desc = decode_image(bytes).expect("Failed to decode image");
let texture_id = engine.renderer_mut().unwrap()
    .create_texture(&desc).expect("Failed to upload texture");
```

Or use the convenience method on Engine:

```rust
let texture = engine.load_texture("textures/donut-pink.png")?;
```

## Level Trait

Optional scene abstraction with shared state and lifecycle hooks. Each level owns a World.

```rust
pub trait Level<S = ()> {
    fn world(&self) -> &World;
    fn world_mut(&mut self) -> &mut World;
    fn update(&mut self, ctx: &mut Ctx<S>);
    fn render(&mut self, ctx: &mut Ctx<S>);

    // Lifecycle hooks (default no-op):
    fn on_enter(&mut self) {}
    fn on_exit(&mut self) {}
    fn on_pause(&mut self) {}
    fn on_resume(&mut self) {}
}
```

### Ctx (Unified Context)

Single context passed to levels for both update and render. Replaces the old split `LevelContext` / `RenderContext`:

```rust
pub struct Ctx<'a, S = ()> {
    pub input: &'a InputState,                      // raw input for this frame
    pub dt: f32,                                     // fixed timestep delta
    pub shared: &'a mut S,                           // shared state from the Game
    pub renderer: &'a mut dyn Renderer<Error = String>, // renderer
    pub events: &'a mut EventBus<World>,             // event bus
}
```

Build from the engine:

```rust
let mut ctx = engine.ctx(&mut self.shared);
level.update(&mut ctx);
level.render(&mut ctx);
```

| Method | Description |
|--------|-------------|
| `screen_size()` | Get screen/canvas size in pixels |
| `create_render_target(w, h)` | Create offscreen render target, returns `(RenderTargetId, TextureId)` |
| `bind_render_target(id)` | Bind render target for draw calls |
| `destroy_render_target(id)` | Destroy offscreen render target |
| `draw_overlay(texture, position, size)` | Draw render-target texture as screen-space overlay (0..1 NDC) |
| `draw_overlay_bordered(texture, pos, size, border, color)` | Same, with a colored border |
| `create_ui::<E>(font_bytes) -> Result<Ui<E>>` | Create UI pre-wired to the event bus |
| `flush_events(world)` | Translate collision events and fire all event handlers |
| `on_collision(world, handler) -> HandlerId` | Register handler for all collisions |
| `on_collision_for(world, object, handler) -> HandlerId` | Collisions involving a specific object |
| `on_collision_between(world, a, b, handler) -> HandlerId` | Collisions between two specific objects |

## Event System

The `EventBus<World>` provides type-erased pub/sub messaging. Access via `ctx.events` or `engine.events()`.

### Registering Handlers

```rust
ctx.events.on::<MyEvent>(|event, world| { /* handle */ });
ctx.events.off(id);  // unsubscribe
```

### Emitting Events

```rust
ctx.events.emit(MyEvent { score: 100 });
```

### Pull-Based Consumption

```rust
// Drain events of a specific type without firing handlers
for action in ctx.events.drain::<MenuAction>() {
    ctx.shared.selected_level = action.level;
}
```

### Collision Events

Auto-enabled when registering collision handlers:

```rust
ctx.on_collision(&mut self.world, |event, world| {
    println!("{:?} hit {:?}", event.object_a, event.object_b);
});

ctx.on_collision_between(&mut self.world, player, coin, |event, world| {
    world.objects.despawn(event.object_b);
});
```

Call `ctx.flush_events(&mut self.world)` after `world.step(dt)` each frame.

`CollisionEvent`: `object_a`, `object_b`, `normal`, `penetration`, `contact_point`.

## Project Setup

```
your-game/
├── project/lib.rs          # Platform entry point (Rust); game code is in Lua
├── project/assets/         # Game assets + Lua scripts (embedded at build time)
├── build.rs                # Calls unison_assets::build::embed_assets()
├── unison2d/               # Engine (git submodule)
├── Cargo.toml              # depends on unison2d + unison-scripting + unison-web + wasm-bindgen
├── index.html              # Canvas element with id="canvas"
├── Trunk.toml              # Build config
└── Makefile                # make dev / make build
```

**Cargo.toml:**
```toml
[dependencies]
unison2d = { path = "unison2d/crates/unison2d" }
unison-web = { path = "unison2d/crates/unison-web" }
wasm-bindgen = "0.2"

[build-dependencies]
unison-assets = { path = "unison2d/crates/unison-assets", features = ["build"] }
```

**Commands:** `make dev` (dev server), `make build` (production), `cargo test` (tests)
