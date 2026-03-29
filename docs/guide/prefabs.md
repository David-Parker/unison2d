# Prefabs & Shared Code

When multiple levels share the same objects or setup logic, extract them into reusable pieces. Unison provides the `Prefab` trait for spawn templates and Rust's module system handles the rest.

## The Prefab Trait

```rust
pub trait Prefab {
    fn spawn(&self, world: &mut World, position: Vec2) -> ObjectId;
}
```

A prefab defines *how* to create an object. Each call to `spawn()` creates an independent instance — there's no shared state between instances.

### Defining a Prefab

```rust
use unison2d::*;
use unison2d::core::{Color, Vec2};
use unison2d::physics::{Material, mesh::create_ring_mesh};

pub struct DonutPrefab;

impl Prefab for DonutPrefab {
    fn spawn(&self, world: &mut World, position: Vec2) -> ObjectId {
        world.spawn_soft_body(SoftBodyDesc {
            mesh: create_ring_mesh(1.0, 0.4, 16, 6),
            material: Material::METAL,
            position,
            color: Color::from_hex(0xd4943a),
            texture: TextureId::NONE,
        })
    }
}
```

### Using a Prefab

```rust
let player = DonutPrefab.spawn(&mut world, Vec2::new(0.0, 3.0));
let npc    = DonutPrefab.spawn(&mut world, Vec2::new(5.0, 3.0));
```

Same definition, independent instances, different positions.

### Prefabs with Parameters

Prefabs can carry configuration. Use struct fields:

```rust
pub struct CratePrefab {
    pub size: f32,
    pub material: Material,
}

impl Prefab for CratePrefab {
    fn spawn(&self, world: &mut World, position: Vec2) -> ObjectId {
        world.spawn_soft_body(SoftBodyDesc {
            mesh: create_square_mesh(self.size, 3),
            material: self.material,
            position,
            color: Color::from_hex(0x8b6914),
            texture: TextureId::NONE,
        })
    }
}

// Small wooden crate
let small = CratePrefab { size: 0.5, material: Material::WOOD };
small.spawn(&mut world, Vec2::new(2.0, 5.0));

// Big metal crate
let big = CratePrefab { size: 1.5, material: Material::METAL };
big.spawn(&mut world, Vec2::new(-3.0, 5.0));
```

### When to Use Prefabs vs. Plain Functions

**Use a Prefab when:**
- The same object type appears across multiple levels
- You want a consistent spawn interface (`spawn(world, position)`)
- The object definition is complex enough to name

**Use a plain function when:**
- The spawn logic is simple and only used in one place
- You need to return multiple ObjectIds or do post-spawn setup

## Shared Helper Functions

Beyond prefabs, extract any repeated setup into plain functions. A common pattern is a world-setup helper:

### World Setup

```rust
// levels/shared.rs

/// Create a World with standard game configuration.
pub fn new_world() -> World {
    let mut world = World::new();
    world.set_background(Color::from_hex(0x1a1a2e));
    world.objects.set_gravity(-9.8);
    world.objects.set_ground(-4.5);
    world.objects.set_ground_restitution(0.0);
    world
}
```

Every level starts with `let mut world = new_world();` — one line instead of repeating five.

### Input Handling

If multiple levels share the same player controls, extract them:

```rust
// levels/shared.rs

const MOVE_FORCE: f32 = 80.0;
const ROLL_TORQUE: f32 = 20.0;
const JUMP_IMPULSE: f32 = 10.0;

pub fn drive_donut(world: &mut World, donut: ObjectId, input: &InputState, dt: f32) {
    let left = input.is_key_pressed(KeyCode::ArrowLeft)
        || input.is_key_pressed(KeyCode::A);
    let right = input.is_key_pressed(KeyCode::ArrowRight)
        || input.is_key_pressed(KeyCode::D);
    let jump = input.is_key_just_pressed(KeyCode::Space)
        || input.is_key_just_pressed(KeyCode::W);

    let move_x = if left && !right { -1.0 }
        else if right && !left { 1.0 }
        else { 0.0 };

    if move_x != 0.0 {
        world.objects.apply_force(donut, Vec2::new(move_x * MOVE_FORCE, 0.0));
        if world.objects.is_grounded(donut) {
            world.objects.apply_torque(donut, -move_x * ROLL_TORQUE, dt);
        }
    }

    if jump && world.objects.is_grounded(donut) {
        world.objects.apply_impulse(donut, Vec2::new(0.0, JUMP_IMPULSE));
    }
}
```

Then each level's update is just:

```rust
fn update(&mut self, ctx: &mut Ctx<SharedState>) {
    drive_donut(&mut self.world, self.donut, ctx.input, ctx.dt);
    // Level-specific logic...
    self.world.step(ctx.dt);
}
```

## Recommended File Layout

```
project/
├── lib.rs                # Game struct, SharedState, event handling
└── levels/
    ├── mod.rs            # pub mod shared; pub mod gameplay; ...
    ├── shared.rs         # Prefabs, new_world(), drive_player(), constants
    ├── main_level.rs     # First level
    └── boss_level.rs     # Another level
```

Keep `shared.rs` as the single home for cross-level code. If it grows too large, split into `shared/prefabs.rs`, `shared/input.rs`, etc.

## Next Steps

- [Levels](levels.md) — level trait, shared state, transitions
- [Patterns](patterns.md) — gameplay recipes
