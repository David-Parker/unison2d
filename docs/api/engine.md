# Engine, World, Level & Game Trait

The core architecture of Unison 2D. Games compose `World`s and `Level`s, while `Engine` is a thin bridge for input and rendering.

## Architecture Overview

```
Game (your struct, implements Game trait)
├── Engine<A>        — input/actions, renderer access, compositing
├── World            — self-contained simulation
│   ├── ObjectSystem   — physics + object registry
│   ├── CameraSystem   — named cameras + follow targets
│   └── LightingSystem — dynamic lights & shadows
└── Level (trait)    — optional scene abstraction
```

## Game Trait

```rust
pub trait Game {
    type Action: Copy + Eq + Hash + 'static;
    fn init(&mut self, engine: &mut Engine<Self::Action>);
    fn update(&mut self, engine: &mut Engine<Self::Action>);
    fn render(&mut self, engine: &mut Engine<Self::Action>);  // required
}
```

- `init()` — called once. Bind input, set up world(s).
- `update()` — called per fixed timestep (60Hz). Read input, apply forces, game logic, step world(s).
- `render()` — called once per frame. Game controls all rendering (no auto-render).

## Engine\<A\>

Thin shell for input, actions, renderer access, and compositing. Does NOT own a world.

### Input / Actions

| Method | Description |
|--------|-------------|
| `bind_key(key, action)` | Bind keyboard key to action |
| `bind_mouse_button(btn, action)` | Bind mouse button to action |
| `action_active(action) -> bool` | Action is held |
| `action_just_started(action) -> bool` | Action pressed this frame |
| `action_just_ended(action) -> bool` | Action released this frame |
| `action_axis(neg, pos) -> f32` | -1/0/+1 axis from two actions |

### Renderer Access

| Method | Description |
|--------|-------------|
| `renderer_mut() -> Option<&mut dyn Renderer>` | Mutable renderer access |
| `render_context() -> Option<RenderContext>` | Build a `RenderContext` for passing to levels |
| `level_context(shared) -> LevelContext<S>` | Build a `LevelContext` for passing to levels |
| `dt() -> f32` | Fixed timestep delta |
| `input_state() -> &InputState` | Raw input state |
| `actions_mut() -> &mut ActionMap<A>` | Direct action map access |

### Render Targets

| Method | Description |
|--------|-------------|
| `create_render_target(w, h) -> Result<(RenderTargetId, TextureId)>` | Create offscreen target |
| `destroy_render_target(target)` | Destroy target (keeps texture) |

### Compositing

Used to arrange render target outputs on screen:

```rust
fn render(&mut self, engine: &mut Engine<Action>) {
    // Render world to offscreen target
    if let Some(r) = engine.renderer_mut() {
        self.world.render_to_targets(r, &[("main", self.target)]);
    }

    // Composite onto screen
    engine.begin_composite(Color::BLACK);
    engine.composite_layer(self.texture, Rect::from_position(Vec2::ZERO, Vec2::ONE));
    engine.end_composite();
}
```

| Method | Description |
|--------|-------------|
| `begin_composite(clear_color)` | Bind screen, set up 1×1 camera, clear |
| `composite_layer(texture, screen_rect)` | Draw texture at screen rect (0→1 normalized) |
| `end_composite()` | End composite frame |

## World

Self-contained simulation. Composes subsystems for physics/objects, cameras, and lighting.

```rust
let mut world = World::new();
world.set_background(Color::from_hex(0x1a1a2e));
world.objects.set_gravity(Vec2::new(0.0, -9.8));
world.objects.set_ground(-5.0);
let player = world.objects.spawn_soft_body(desc);
world.cameras.follow("main", player, 0.08);

// Each tick:
world.step(dt);
```

| Method | Description |
|--------|-------------|
| `new() -> World` | Default world (main camera 20×15, standard gravity) |
| `set_background(color)` | Set clear color |
| `background_color() -> Color` | Get clear color |
| `step(dt)` | Advance physics + update camera follows |
| `snapshot_for_render()` | Snapshot for interpolated rendering |
| `auto_render(renderer)` | Render through "main" camera to current target |
| `render_to_targets(renderer, &[(&str, RenderTargetId)])` | Multi-camera rendering to targets |

### ObjectSystem (`world.objects`)

Owns the physics world + object registry.

#### Spawning / Despawning

| Method | Description |
|--------|-------------|
| `spawn_soft_body(desc) -> ObjectId` | Create soft body |
| `spawn_rigid_body(desc) -> ObjectId` | Create rigid body |
| `spawn_static_rect(pos, size, color) -> ObjectId` | Convenience for static platforms |
| `despawn(id)` | Remove object |

#### Queries & Forces

| Method | Description |
|--------|-------------|
| `get_position(id) -> Vec2` | Object center position |
| `set_position(id, pos)` | Teleport object |
| `get_velocity(id) -> Vec2` | Object velocity |
| `set_velocity(id, vel)` | Set velocity directly |
| `apply_force(id, force)` | Continuous force (call each frame) |
| `apply_torque(id, torque, dt)` | Continuous rotation |
| `apply_impulse(id, impulse)` | Instantaneous velocity change |
| `is_grounded(id) -> bool` | Touching ground or another body? |

#### Physics Configuration

| Method | Description |
|--------|-------------|
| `set_gravity(Vec2)` | Set gravity |
| `set_ground(y)` | Set flat ground at y |
| `clear_ground()` | Remove ground |
| `set_ground_friction(f32)` | Ground friction (0=ice, 1=sticky). Default: 0.8 |
| `set_ground_restitution(f32)` | Ground bounciness (0=none, 1=perfect). Default: 0.3 |

#### Raw Physics Access

| Method | Description |
|--------|-------------|
| `physics() -> &PhysicsWorld` | Read physics |
| `physics_mut() -> &mut PhysicsWorld` | Modify physics directly |
| `object_count() -> usize` | Number of objects |

### CameraSystem (`world.cameras`)

Named cameras with optional follow targets.

| Method | Description |
|--------|-------------|
| `add(name, camera)` | Add a named camera |
| `remove(name)` | Remove a camera |
| `get(name) -> Option<&Camera>` | Get camera by name |
| `get_mut(name) -> Option<&mut Camera>` | Mutate camera by name |
| `iter() -> impl Iterator` | Iterate all cameras |
| `count() -> usize` | Number of cameras |
| `follow(name, object_id, smoothing)` | Camera follows an object |
| `unfollow(name)` | Stop following |

Default: "main" camera at 20×15.

### LightingSystem (`world.lighting`)

See [lighting.md](lighting.md).

## Level Trait

Optional abstraction for self-contained game scenes, generic over shared state `S`.

```rust
pub trait Level<S = ()> {
    fn world(&self) -> &World;
    fn world_mut(&mut self) -> &mut World;
    fn update(&mut self, ctx: &mut LevelContext<S>);
    fn render(&mut self, ctx: &mut RenderContext);

    fn on_enter(&mut self) {}
    fn on_exit(&mut self) {}
    fn on_pause(&mut self) {}
    fn on_resume(&mut self) {}
}
```

### LevelContext

```rust
pub struct LevelContext<'a, S = ()> {
    pub input: &'a InputState,
    pub dt: f32,
    pub shared: &'a mut S,
}
```

### RenderContext

```rust
pub struct RenderContext<'a> {
    pub renderer: &'a mut dyn Renderer<Error = String>,
}
```

| Method | Description |
|--------|-------------|
| `create_render_target(w, h)` | Create offscreen render target |
| `bind_render_target(id)` | Bind render target for draw calls |
| `destroy_render_target(id)` | Destroy offscreen render target |
| `screen_size()` | Get screen/canvas size in pixels |
| `draw_overlay(texture, position, size)` | Draw render-target texture as screen-space overlay (0..1 NDC) |
| `draw_overlay_bordered(texture, position, size, border_width, border_color)` | Same, with a colored border |
```

## Object Types

### SoftBodyDesc

```rust
SoftBodyDesc {
    mesh: Mesh,           // from create_ring_mesh, etc.
    material: Material,   // JELLO, RUBBER, WOOD, METAL
    position: Vec2,       // initial world position
    color: Color,         // render color
}
```

### RigidBodyDesc

```rust
RigidBodyDesc {
    collider: Collider,   // aabb(hw, hh) or circle(r)
    position: Vec2,       // initial world position
    color: Color,         // render color
    is_static: bool,      // true = not affected by physics
}
```

### ObjectId

Returned by `spawn_*()`. Use `ObjectId::PLACEHOLDER` for struct initialization before `init()`.
