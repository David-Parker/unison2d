# Unison 2D — API Reference

Single-file reference for building games with Unison 2D. Read this to learn the entire engine API.

## Quick Start

```rust
use wasm_bindgen::prelude::*;
use unison2d::*;
use unison2d::math::{Color, Vec2};
use unison2d::physics::{Material, mesh::create_ring_mesh};
use unison2d::input::KeyCode;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
enum Action { MoveLeft, MoveRight, Jump }

struct MyGame {
    world: World,
    player: ObjectId,
}

impl Game for MyGame {
    type Action = Action;

    fn init(&mut self, engine: &mut Engine<Action>) {
        self.world.set_background(Color::from_hex(0x1a1a2e));
        self.world.objects.set_gravity(Vec2::new(0.0, -9.8));
        self.world.objects.set_ground(-5.0);

        self.player = self.world.objects.spawn_soft_body(SoftBodyDesc {
            mesh: create_ring_mesh(1.0, 0.4, 16, 6),
            material: Material::RUBBER,
            position: Vec2::new(0.0, 3.0),
            color: Color::from_hex(0xd4943a),
        });

        self.world.objects.spawn_static_rect(
            Vec2::new(0.0, -5.5), Vec2::new(30.0, 2.0), Color::from_hex(0x2d5016),
        );
        self.world.cameras.follow("main", self.player, 0.08);

        engine.bind_key(KeyCode::ArrowLeft, Action::MoveLeft);
        engine.bind_key(KeyCode::ArrowRight, Action::MoveRight);
        engine.bind_key(KeyCode::Space, Action::Jump);
    }

    fn update(&mut self, engine: &mut Engine<Action>) {
        let move_x = engine.action_axis(Action::MoveLeft, Action::MoveRight);
        if move_x != 0.0 {
            self.world.objects.apply_force(self.player, Vec2::new(move_x * 1200.0, 0.0));
        }
        if engine.action_just_started(Action::Jump) && self.world.objects.is_grounded(self.player) {
            self.world.objects.apply_impulse(self.player, Vec2::new(0.0, 8.0));
        }
        self.world.step(engine.dt());
    }

    fn render(&mut self, engine: &mut Engine<Action>) {
        if let Some(r) = engine.renderer_mut() {
            self.world.auto_render(r);
        }
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    unison_web::run(MyGame {
        world: World::new(),
        player: ObjectId::PLACEHOLDER,
    });
}
```

Run with `make dev`.

---

## Architecture

```
Game (your struct)
├── Engine<A>        — input/actions, renderer, compositing
├── World            — physics, objects, cameras, lighting
│   ├── ObjectSystem
│   ├── CameraSystem
│   └── LightingSystem
└── Level (trait)    — optional scene abstraction
```

- **Engine** is a thin shell — only input mapping and renderer access
- **World** is where all simulation lives — games own one or more Worlds
- **Level** is an optional trait for organizing scenes

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

## World

Self-contained simulation owning physics, objects, cameras, and lighting.

```rust
let mut world = World::new();
world.set_background(Color::from_hex(0x1a1a2e));
world.objects.set_gravity(Vec2::new(0.0, -9.8));
world.objects.set_ground(-5.0);
```

| Method | Description |
|--------|-------------|
| `new()` | Default world (main camera 20×15) |
| `set_background(color)` | Set clear color |
| `background_color()` | Get clear color |
| `step(dt)` | Advance physics + update camera follows |
| `auto_render(renderer)` | Render through "main" camera |
| `render_to_targets(renderer, &[(&str, RenderTargetId)])` | Multi-camera render |

## Creating Objects

All object operations go through `world.objects`.

### Soft Bodies

```rust
let mesh = create_ring_mesh(outer_radius, inner_radius, segments, radial_divs);
let id = world.objects.spawn_soft_body(SoftBodyDesc {
    mesh,
    material: Material::RUBBER,
    position: Vec2::new(x, y),
    color: Color::from_hex(0xd4943a),
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
let id = world.objects.spawn_rigid_body(RigidBodyDesc {
    collider: Collider::aabb(half_width, half_height),
    position: Vec2::new(x, y),
    color: Color::from_hex(0x4a3728),
    is_static: true,
});
```

**Colliders** (from `unison2d::physics::Collider`):
- `Collider::aabb(half_width, half_height)` — rectangle
- `Collider::circle(radius)` — circle

### Static Rectangles (convenience)

```rust
let id = world.objects.spawn_static_rect(position, size, color);
```

### Despawn

```rust
world.objects.despawn(id);
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
world.objects.set_gravity(Vec2::new(0.0, -9.8));  // gravity direction + magnitude
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

### Available KeyCodes

`ArrowUp`, `ArrowDown`, `ArrowLeft`, `ArrowRight`, `Space`, `Enter`, `Escape`, `Tab`, `Backspace`, `ShiftLeft`, `ShiftRight`, `ControlLeft`, `ControlRight`, `AltLeft`, `AltRight`, `A`-`Z`, `Digit0`-`Digit9`

## Cameras

Cameras live on the world's `CameraSystem`. A default "main" camera (20×15) is created automatically.

```rust
world.cameras.follow("main", player_id, 0.08);  // follow with smoothing
world.cameras.unfollow("main");                  // stop following
world.cameras.get_mut("main").unwrap().zoom = 2.0;  // zoom in
world.cameras.get_mut("main").unwrap().set_position(x, y);  // manual position

// Add more cameras
world.cameras.add("minimap", Camera::new(100.0, 75.0));

// Iterate
for (name, camera) in world.cameras.iter() { /* ... */ }
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
        // Draw additional stuff after auto_render:
        r.draw(RenderCommand::Line {
            start: [0.0, 0.0],
            end: [5.0, 5.0],
            color: Color::RED,
            width: 0.05,
        });
    }
}
```

### Colors

```rust
Color::from_hex(0xff9f43)           // from hex
Color::from_rgba8(255, 159, 67, 255) // from RGBA bytes
Color::WHITE, Color::BLACK, Color::RED, Color::GREEN, Color::BLUE
```

## Level Trait

Optional scene abstraction. Each level owns a World.

```rust
pub trait Level {
    fn world(&self) -> &World;
    fn world_mut(&mut self) -> &mut World;
    fn update(&mut self, input: &InputState, dt: f32);
    fn render(&mut self, renderer: &mut dyn Renderer<Error = String>);
}
```

Levels take `&InputState` (not generic over actions), enabling `Vec<Box<dyn Level>>`.

```rust
struct GameplayLevel { world: World, player: ObjectId }

impl Level for GameplayLevel {
    fn world(&self) -> &World { &self.world }
    fn world_mut(&mut self) -> &mut World { &mut self.world }
    fn update(&mut self, input: &InputState, dt: f32) {
        // game logic...
        self.world.step(dt);
    }
    fn render(&mut self, renderer: &mut dyn Renderer<Error = String>) {
        self.world.auto_render(renderer);
    }
}
```

## Common Patterns

### Platformer Movement

```rust
fn update(&mut self, engine: &mut Engine<Action>) {
    let move_x = engine.action_axis(Action::MoveLeft, Action::MoveRight);
    if move_x != 0.0 {
        self.world.objects.apply_force(self.player, Vec2::new(move_x * 1200.0, 0.0));
    }
    if engine.action_just_started(Action::Jump) && self.world.objects.is_grounded(self.player) {
        self.world.objects.apply_impulse(self.player, Vec2::new(0.0, 8.0));
    }
    self.world.step(engine.dt());
}
```

### Spawning Objects on Input

```rust
fn update(&mut self, engine: &mut Engine<Action>) {
    if engine.action_just_started(Action::Spawn) {
        let pos = self.world.objects.get_position(self.player) + Vec2::new(2.0, 0.0);
        self.world.objects.spawn_soft_body(SoftBodyDesc {
            mesh: create_square_mesh(0.5, 3),
            material: Material::JELLO,
            position: pos,
            color: Color::from_hex(0x6c5ce7),
        });
    }
    self.world.step(engine.dt());
}
```

### Camera Zoom

```rust
fn update(&mut self, engine: &mut Engine<Action>) {
    if engine.action_active(Action::ZoomIn) {
        self.world.cameras.get_mut("main").unwrap().zoom *= 1.02;
    }
    if engine.action_active(Action::ZoomOut) {
        self.world.cameras.get_mut("main").unwrap().zoom *= 0.98;
    }
    self.world.step(engine.dt());
}
```

## Project Setup

```
your-game/
├── project/lib.rs          # Game code
├── unison2d/               # Engine (git submodule)
├── Cargo.toml              # depends on unison2d + unison-web + wasm-bindgen
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
```

**Commands:** `make dev` (dev server), `make build` (production), `cargo test` (tests)
