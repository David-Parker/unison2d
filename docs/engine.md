# Engine & Game Trait

The batteries-included layer that sits on top of the subsystem crates.

## Game Trait

```rust
pub trait Game {
    type Action: Copy + Eq + Hash + 'static;
    fn init(&mut self, engine: &mut Engine<Self::Action>);
    fn update(&mut self, engine: &mut Engine<Self::Action>);
    fn render(&mut self, engine: &mut Engine<Self::Action>) {} // optional override
}
```

- `init()` — called once. Spawn objects, bind input, set up world.
- `update()` — called per fixed timestep (60Hz). Read input, apply forces, game logic.
- `render()` — called once per frame. Engine auto-renders all objects first. Override only for custom drawing (UI, debug, particles).

## Engine<A>

Single struct that owns and orchestrates all subsystems.

### Object Management

| Method | Description |
|--------|-------------|
| `spawn_soft_body(desc) -> ObjectId` | Create a soft body + auto-render |
| `spawn_rigid_body(desc) -> ObjectId` | Create a rigid body + auto-render |
| `spawn_static_rect(pos, size, color) -> ObjectId` | Convenience for static platforms |
| `despawn(id)` | Remove object from world |

### Physics Queries

| Method | Description |
|--------|-------------|
| `get_position(id) -> Vec2` | Object center position |
| `set_position(id, pos)` | Teleport object |
| `get_velocity(id) -> Vec2` | Object velocity |
| `set_velocity(id, vel)` | Set velocity directly |
| `apply_force(id, force)` | Continuous force (call each frame) |
| `apply_torque(id, torque)` | Continuous rotation (negative = clockwise) |
| `apply_impulse(id, impulse)` | Instantaneous velocity change |
| `is_grounded(id) -> bool` | Is touching ground, platform, or another body? |

### Camera

| Method | Description |
|--------|-------------|
| `camera_follow(id, smoothing)` | Follow an object |
| `camera_unfollow()` | Stop following |
| `camera_mut() -> &mut Camera` | Direct camera access |
| `camera() -> &Camera` | Read camera state |

### Input / Actions

| Method | Description |
|--------|-------------|
| `bind_key(key, action)` | Bind keyboard key to action |
| `bind_mouse_button(btn, action)` | Bind mouse button to action |
| `action_active(action) -> bool` | Action is held |
| `action_just_started(action) -> bool` | Action pressed this frame |
| `action_just_ended(action) -> bool` | Action released this frame |
| `action_axis(neg, pos) -> f32` | -1/0/+1 axis from two actions |

### Environment

| Method | Description |
|--------|-------------|
| `set_gravity(Vec2)` | Set gravity |
| `set_background(Color)` | Set clear color |
| `set_ground(y)` | Set flat ground at y |
| `clear_ground()` | Remove ground |
| `dt() -> f32` | Fixed timestep delta |

### Raw Access (Escape Hatches)

| Method | Returns |
|--------|---------|
| `physics_mut()` | `&mut PhysicsWorld` |
| `physics()` | `&PhysicsWorld` |
| `lighting_mut()` | `&mut LightingManager` |
| `lighting()` | `&LightingManager` |
| `input_state()` | `&InputState` |
| `actions_mut()` | `&mut ActionMap<A>` |

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
