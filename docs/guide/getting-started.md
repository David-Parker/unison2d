# Getting Started

This guide walks through building a minimal game with Unison 2D — a bouncy ball on a platform.

## Project Structure

```
your-game/
├── project/
│   └── lib.rs          # Your game code
├── unison2d/           # Engine (git submodule)
├── Cargo.toml
├── index.html          # Canvas element with id="canvas"
├── Trunk.toml
└── Makefile
```

**Cargo.toml:**
```toml
[dependencies]
unison2d = { path = "unison2d/crates/unison2d" }
unison-web = { path = "unison2d/crates/unison-web" }
wasm-bindgen = "0.2"
```

## Minimal Game

Every Unison game implements the `Game` trait. Here's the smallest working game:

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
        // Configure the world
        self.world.set_background(Color::from_hex(0x1a1a2e));
        self.world.objects.set_gravity(Vec2::new(0.0, -9.8));
        self.world.objects.set_ground(-5.0);

        // Spawn a bouncy ring
        self.player = self.world.spawn_soft_body(SoftBodyDesc {
            mesh: create_ring_mesh(1.0, 0.4, 16, 6),
            material: Material::RUBBER,
            position: Vec2::new(0.0, 3.0),
            color: Color::from_hex(0xd4943a),
        });

        // Ground platform
        self.world.spawn_static_rect(
            Vec2::new(0.0, -5.5), Vec2::new(30.0, 2.0), Color::from_hex(0x2d5016),
        );

        // Camera follows the player
        self.world.cameras.follow("main", self.player, 0.08);

        // Bind controls
        engine.bind_key(KeyCode::ArrowLeft, Action::MoveLeft);
        engine.bind_key(KeyCode::ArrowRight, Action::MoveRight);
        engine.bind_key(KeyCode::Space, Action::Jump);
    }

    fn update(&mut self, engine: &mut Engine<Action>) {
        let move_x = engine.action_axis(Action::MoveLeft, Action::MoveRight);
        if move_x != 0.0 {
            self.world.objects.apply_force(self.player, Vec2::new(move_x * 1200.0, 0.0));
        }
        if engine.action_just_started(Action::Jump)
            && self.world.objects.is_grounded(self.player)
        {
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

## Key Concepts

### World

`World` is the central simulation container. It owns:
- **ObjectSystem** (`world.objects`) — physics bodies, sprites, lights
- **CameraSystem** (`world.cameras`) — named cameras with follow targets
- **LightingSystem** (`world.lighting`) — dynamic lights and shadows
- **Environment** (`world.environment`) — rendering config like background color

You create objects through `world.spawn_*()` methods and advance the simulation with `world.step(dt)`.

### Engine

`Engine<A>` is a thin shell the platform gives you. It provides:
- Input binding and action queries
- Renderer access (`engine.render_context()`)
- Fixed timestep (`engine.dt()`)

Engine does NOT own your game state — that's your job.

### Game Loop

The engine runs a fixed-timestep loop:
1. `init()` — called once at startup
2. `update()` — called at 60Hz (use `engine.dt()` for the timestep)
3. `render()` — called once per frame

You call `world.step(dt)` inside `update()` to advance physics. You call `world.auto_render(renderer)` inside `render()` to draw.

## Next Steps

- **Multiple scenes?** Read [Levels](levels.md)
- **Reusing object definitions?** Read [Prefabs & Shared Code](prefabs.md)
- **Specific gameplay patterns?** Read [Patterns](patterns.md)
