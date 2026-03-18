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

struct MyGame { player: ObjectId }

impl Game for MyGame {
    type Action = Action;

    fn init(&mut self, engine: &mut Engine<Action>) {
        engine.set_background(Color::from_hex(0x1a1a2e));
        engine.set_gravity(Vec2::new(0.0, -9.8));
        engine.set_ground(-5.0);

        self.player = engine.spawn_soft_body(SoftBodyDesc {
            mesh: create_ring_mesh(1.0, 0.4, 16, 6),
            material: Material::RUBBER,
            position: Vec2::new(0.0, 3.0),
            color: Color::from_hex(0xd4943a),
        });

        engine.spawn_static_rect(Vec2::new(0.0, -5.5), Vec2::new(30.0, 2.0), Color::from_hex(0x2d5016));
        engine.camera_follow(self.player, 0.08);
        engine.bind_key(KeyCode::ArrowLeft, Action::MoveLeft);
        engine.bind_key(KeyCode::ArrowRight, Action::MoveRight);
        engine.bind_key(KeyCode::Space, Action::Jump);
    }

    fn update(&mut self, engine: &mut Engine<Action>) {
        let move_x = engine.action_axis(Action::MoveLeft, Action::MoveRight);
        engine.apply_force(self.player, Vec2::new(move_x * 25.0, 0.0));
        if engine.action_just_started(Action::Jump) && engine.is_grounded(self.player) {
            engine.apply_impulse(self.player, Vec2::new(0.0, 8.0));
        }
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    unison_web::run(MyGame { player: ObjectId::PLACEHOLDER });
}
```

Run with `make dev`.

---

## Game Trait

```rust
pub trait Game {
    type Action: Copy + Eq + Hash + 'static;
    fn init(&mut self, engine: &mut Engine<Self::Action>);    // called once
    fn update(&mut self, engine: &mut Engine<Self::Action>);  // called per fixed timestep (60Hz)
    fn render(&mut self, engine: &mut Engine<Self::Action>) {} // optional, auto-render is default
}
```

- Define your `Action` enum for input mapping
- Physics is stepped automatically after `update()` returns
- All spawned objects are rendered automatically — override `render()` only for custom drawing

## Creating Objects

### Soft Bodies

```rust
let mesh = create_ring_mesh(outer_radius, inner_radius, segments, radial_divs);
let id = engine.spawn_soft_body(SoftBodyDesc {
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
- `Material::JELLO` — soft, jiggly
- `Material::RUBBER` — bouncy (default)
- `Material::WOOD` — stiff
- `Material::METAL` — nearly rigid
- Custom: `Material::new(density, edge_compliance, area_compliance)`

### Rigid Bodies

```rust
let id = engine.spawn_rigid_body(RigidBodyDesc {
    collider: Collider::aabb(half_width, half_height),
    position: Vec2::new(x, y),
    color: Color::from_hex(0x4a3728),
    is_static: true,  // false for dynamic
});
```

**Colliders** (from `unison2d::physics::Collider`):
- `Collider::aabb(half_width, half_height)` — rectangle
- `Collider::circle(radius)` — circle

### Static Rectangles (convenience)

```rust
let id = engine.spawn_static_rect(position, size, color);
```

### Despawn

```rust
engine.despawn(id);
```

## Physics & Movement

```rust
engine.apply_force(id, Vec2::new(10.0, 0.0));   // continuous, call each frame
engine.apply_impulse(id, Vec2::new(0.0, 8.0));   // instantaneous velocity change
engine.set_velocity(id, Vec2::new(5.0, 0.0));    // set velocity directly
let pos = engine.get_position(id);                // get current position
engine.set_position(id, Vec2::new(0.0, 0.0));    // teleport
let vel = engine.get_velocity(id);                // get current velocity
let grounded = engine.is_grounded(id);            // touching ground?
```

### Environment

```rust
engine.set_gravity(Vec2::new(0.0, -9.8));  // gravity direction + magnitude
engine.set_ground(-5.0);                    // flat ground at y=-5
engine.clear_ground();                      // remove ground
engine.dt();                                // current fixed timestep (1/60)
```

## Input & Actions

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

## Camera

```rust
engine.camera_follow(object_id, 0.08);      // follow with smoothing (0.05-0.2 typical)
engine.camera_unfollow();                    // stop following
engine.camera_mut().set_position(x, y);     // manual position
engine.camera_mut().zoom = 2.0;             // zoom in 2x
engine.camera_mut().width = 30.0;           // viewport width in world units
engine.camera_mut().height = 22.5;          // viewport height
let (min_x, min_y, max_x, max_y) = engine.camera().bounds();
```

## Rendering

### Automatic

All spawned objects are rendered automatically. Soft bodies render as meshes, rigid bodies as rectangles.

```rust
engine.set_background(Color::from_hex(0x1a1a2e));  // background clear color
```

### Custom (override render())

```rust
fn render(&mut self, engine: &mut Engine<Action>) {
    // Engine already rendered all objects. Draw additional stuff:
    let renderer = engine.renderer.as_mut().unwrap();
    renderer.draw(RenderCommand::Line {
        start: [0.0, 0.0],
        end: [5.0, 5.0],
        color: Color::RED,
        width: 0.05,
    });
}
```

### Colors

```rust
Color::from_hex(0xff9f43)           // from hex
Color::from_rgba8(255, 159, 67, 255) // from RGBA bytes
Color::WHITE, Color::BLACK, Color::RED, Color::GREEN, Color::BLUE
```

## Raw Access (Escape Hatches)

When the high-level API isn't enough:

```rust
engine.physics_mut()   // -> &mut PhysicsWorld
engine.physics()       // -> &PhysicsWorld
engine.lighting_mut()  // -> &mut LightingManager
engine.lighting()      // -> &LightingManager
engine.input_state()   // -> &InputState (raw keyboard/mouse/touch)
engine.actions_mut()   // -> &mut ActionMap<A>
engine.camera_mut()    // -> &mut Camera
```

## Common Patterns

### Platformer Movement

```rust
fn update(&mut self, engine: &mut Engine<Action>) {
    let move_x = engine.action_axis(Action::MoveLeft, Action::MoveRight);
    if move_x != 0.0 {
        engine.apply_force(self.player, Vec2::new(move_x * 25.0, 0.0));
    }
    if engine.action_just_started(Action::Jump) && engine.is_grounded(self.player) {
        engine.apply_impulse(self.player, Vec2::new(0.0, 8.0));
    }
}
```

### Spawning Objects on Input

```rust
fn update(&mut self, engine: &mut Engine<Action>) {
    if engine.action_just_started(Action::Spawn) {
        let pos = engine.get_position(self.player) + Vec2::new(2.0, 0.0);
        engine.spawn_soft_body(SoftBodyDesc {
            mesh: create_square_mesh(0.5, 3),
            material: Material::JELLO,
            position: pos,
            color: Color::from_hex(0x6c5ce7),
        });
    }
}
```

### Camera Zoom

```rust
fn update(&mut self, engine: &mut Engine<Action>) {
    if engine.action_active(Action::ZoomIn) {
        engine.camera_mut().zoom *= 1.02;
    }
    if engine.action_active(Action::ZoomOut) {
        engine.camera_mut().zoom *= 0.98;
    }
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
