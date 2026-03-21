# Engine, World, Level & Game Trait

The core architecture of Unison 2D. Games compose `World`s and `Level`s, while `Engine` is a thin bridge for input and rendering.

## Architecture Overview

```
Game (your struct, implements Game trait)
├── Engine<A>        — input/actions, renderer access, assets, compositing
├── World            — self-contained simulation
│   ├── ObjectSystem   — physics + object registry
│   ├── CameraSystem   — named cameras + follow targets
│   └── LightingSystem — point lights, directional lights, shadows
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

### Assets

| Method | Description |
|--------|-------------|
| `assets() -> &AssetStore` | Read-only access to the asset store |
| `assets_mut() -> &mut AssetStore` | Mutable access (for loading) |
| `load_texture(path) -> Result<TextureId>` | Decode + upload a texture from the asset store in one step |

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

Self-contained simulation. Composes subsystems for physics/objects and cameras.

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
| `spawn_sprite(desc) -> ObjectId` | Create sprite (no physics) |
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
| `is_touching(a, b) -> bool` | Two objects in contact? |
| `get_contact(id) -> Option<ObjectId>` | First object in contact with this one |
| `get_sprite_position(id) -> Option<Vec2>` | Get sprite position |
| `set_sprite_position(id, pos)` | Set sprite position |
| `set_sprite_rotation(id, rot)` | Set sprite rotation |

#### Physics Configuration

| Method | Description |
|--------|-------------|
| `set_gravity(Vec2)` | Set gravity |
| `set_ground(y)` | Set flat ground at y |
| `clear_ground()` | Remove ground |
| `set_ground_friction(f32)` | Ground friction (0=ice, 1=sticky). Default: 0.8 |
| `set_ground_restitution(f32)` | Ground bounciness (0=none, 1=perfect). Default: 0.3 |

#### Rendering

| Method | Description |
|--------|-------------|
| `set_z_order(id, i32)` | Set draw order — higher values draw later (on top). Default 0 |
| `z_order(id) -> i32` | Get draw order |
| `set_casts_shadow(id, bool)` | Set whether object casts shadows. Default: true for physics objects, false for sprites |
| `casts_shadow(id) -> bool` | Check whether object casts shadows |

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

2D lighting with lightmap compositing and shadow casting. See [lighting.md](lighting.md) for the full deep dive.

| Method | Description |
|--------|-------------|
| `set_enabled(bool)` | Enable/disable the lighting system |
| `set_ambient(Color)` | Set ambient color (unlit areas) |
| `add_light(PointLight) -> LightId` | Add a point light |
| `add_directional_light(DirectionalLight) -> LightId` | Add a directional light |
| `remove_light(id)` | Remove a point light |
| `remove_directional_light(id)` | Remove a directional light |
| `get_light(id) -> Option<&PointLight>` | Get point light |
| `get_light_mut(id) -> Option<&mut PointLight>` | Mutate point light |
| `get_directional_light(id) -> Option<&DirectionalLight>` | Get directional light |
| `get_directional_light_mut(id) -> Option<&mut DirectionalLight>` | Mutate directional light |
| `light_count() -> usize` | Number of point lights |
| `clear_lights()` | Remove all point lights |
| `directional_light_count() -> usize` | Number of directional lights |
| `clear_directional_lights()` | Remove all directional lights |
| `has_lights() -> bool` | Any lights (point or directional) exist? |
| `ambient() -> Color` | Get current ambient color |
| `is_enabled() -> bool` | Check if lighting is enabled |
| `set_ground_shadow(Option<f32>)` | Clip shadows at ground Y |

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
    material: Material,   // JELLO, RUBBER, WOOD, METAL, or custom
    position: Vec2,       // initial world position
    color: Color,         // render color (tint when textured)
    texture: TextureId,   // TextureId::NONE for solid color
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

### SpriteDesc

```rust
SpriteDesc {
    texture: TextureId,   // texture or TextureId::NONE for solid color
    position: Vec2,       // world position
    size: Vec2,           // size in world units
    rotation: f32,        // rotation in radians
    color: Color,         // render color (tint when textured)
}
```

### ObjectId

Returned by `spawn_*()`. Use `ObjectId::PLACEHOLDER` for struct initialization before `init()`.
