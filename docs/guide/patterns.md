# Patterns

Common gameplay patterns and recipes for Unison 2D games. Each example is self-contained — copy what you need.

## Platformer Movement

Force-based movement with ground check for jumping:

```rust
const MOVE_FORCE: f32 = 80.0;
const JUMP_IMPULSE: f32 = 10.0;

fn update_player(&mut self, ctx: &mut LevelContext<SharedState>) {
    let left = ctx.input.is_key_pressed(KeyCode::ArrowLeft);
    let right = ctx.input.is_key_pressed(KeyCode::ArrowRight);
    let jump = ctx.input.is_key_just_pressed(KeyCode::Space);

    let move_x = match (left, right) {
        (true, false) => -1.0,
        (false, true) =>  1.0,
        _ => 0.0,
    };

    if move_x != 0.0 {
        self.world.objects.apply_force(
            self.player,
            Vec2::new(move_x * MOVE_FORCE, 0.0),
        );
    }

    if jump && self.world.objects.is_grounded(self.player) {
        self.world.objects.apply_impulse(
            self.player,
            Vec2::new(0.0, JUMP_IMPULSE),
        );
    }
}
```

For soft bodies that should roll, add torque:

```rust
if move_x != 0.0 && self.world.objects.is_grounded(self.player) {
    self.world.objects.apply_torque(self.player, -move_x * 20.0, ctx.dt);
}
```

## Spawning Objects

### On a Timer

```rust
struct MyLevel {
    world: World,
    spawn_timer: f32,
    spawned: Vec<ObjectId>,
}

const SPAWN_INTERVAL: f32 = 0.6;
const MAX_OBJECTS: usize = 40;

fn update(&mut self, ctx: &mut LevelContext<SharedState>) {
    self.spawn_timer += ctx.dt;
    if self.spawn_timer >= SPAWN_INTERVAL {
        self.spawn_timer -= SPAWN_INTERVAL;
        self.spawn_object();
    }
    // ...
}

fn spawn_object(&mut self) {
    let id = self.world.spawn_soft_body(SoftBodyDesc {
        mesh: create_square_mesh(0.8, 3),
        material: Material::JELLO,
        position: Vec2::new(0.0, 10.0),
        color: Color::from_hex(0x6c5ce7),
        texture: TextureId::NONE,
    });
    self.spawned.push(id);

    // Cap the count by despawning oldest
    while self.spawned.len() > MAX_OBJECTS {
        let old = self.spawned.remove(0);
        self.world.despawn(old);
    }
}
```

## Trigger Zones

Use a static rigid body as a trigger and check for overlap:

```rust
// Setup:
let trigger = world.spawn_rigid_body(RigidBodyDesc {
    collider: Collider::aabb(1.0, 1.0),
    position: Vec2::new(6.0, -3.0),
    color: Color::from_hex(0xe74c3c),
    is_static: true,
});

// In update:
if !self.triggered && self.world.objects.is_touching(self.player, self.trigger) {
    self.triggered = true;
    ctx.shared.events.push(GameEvent::LevelComplete);
}
```

## Camera Follow

```rust
// Follow the player with smoothing (0.08 = smooth, 1.0 = instant):
world.cameras.follow("main", player_id, 0.08);

// Follow with a fixed offset (e.g. shift view up to show more sky):
world.cameras.follow_with_offset("main", player_id, 0.08, Vec2::new(0.0, 3.0));

// Change offset on an already-following camera:
world.cameras.set_follow_offset("main", Vec2::new(0.0, 5.0));

// Stop following:
world.cameras.unfollow("main");

// Manual position:
world.cameras.get_mut("main").unwrap().set_position(x, y);

// Zoom:
world.cameras.get_mut("main").unwrap().zoom = 2.0;
```

## Picture-in-Picture (PiP)

Render a secondary camera to a texture and composite it as an overlay. Common for minimaps or overview cameras.

### Setup

Add a second camera and lazily create a render target:

```rust
use unison2d::render::{Camera, RenderTargetId, TextureId};

struct PipTarget {
    target: RenderTargetId,
    texture: TextureId,
}

struct MyLevel {
    world: World,
    pip: Option<PipTarget>,
    // ...
}

// In constructor:
world.cameras.add("overview", Camera::new(20.0, 15.0));

// Lazy init (render targets need the renderer, which isn't available in the constructor):
fn ensure_pip(&mut self, ctx: &mut RenderContext) {
    if self.pip.is_some() { return; }

    let (screen_w, screen_h) = ctx.screen_size();
    let pip_w = (screen_w / 4.0) as u32;
    let pip_h = (pip_w as f32 * screen_h / screen_w) as u32;

    let cam_height = 15.0;
    let cam_width = cam_height * (pip_w as f32 / pip_h as f32);
    if let Some(cam) = self.world.cameras.get_mut("overview") {
        cam.width = cam_width;
        cam.height = cam_height;
    }

    let (target, texture) = ctx.create_render_target(pip_w, pip_h)
        .expect("Failed to create PiP render target");
    self.pip = Some(PipTarget { target, texture });
}
```

### Rendering

```rust
fn render(&mut self, ctx: &mut RenderContext) {
    self.ensure_pip(ctx);
    let pip = self.pip.as_ref().unwrap();

    // Render both cameras
    self.world.render_to_targets(ctx.renderer, &[
        ("overview", pip.target),
        ("main", RenderTargetId::SCREEN),
    ]);

    // Draw PiP overlay in bottom-left corner with white border
    ctx.draw_overlay_bordered(pip.texture, [0.02, 0.02], [0.25, 0.25], 0.005, Color::WHITE);
}
```

`draw_overlay_bordered` handles all the NDC math, camera setup, and draw calls. For an overlay without a border, use `draw_overlay(texture, position, size)`. Coordinates are in 0..1 normalized screen space — (0,0) is bottom-left, (1,1) is top-right.

## Day/Night Cycle

Use a single directional light with a time-driven `sun_amount` (1 = sun, 0 = moon). Each half of the cycle arcs the light across the sky; the `sun_amount` blend handles color and intensity transitions at the horizons.

```rust
const CYCLE_DURATION: f32 = 24.0;

struct DayNightCycle {
    light: LightId,
    cycle_time: f32,
}

impl DayNightCycle {
    fn new(world: &mut World) -> Self {
        let light = world.lighting.add_directional_light(DirectionalLight {
            direction: Vec2::new(1.0, 0.0),
            color: Color::new(1.0, 0.5, 0.2, 1.0),
            intensity: 0.6,
            casts_shadows: true,
            shadow: ShadowSettings { /* ... */ },
        });
        Self { light, cycle_time: 0.0 }
    }

    fn update(&mut self, world: &mut World, dt: f32) {
        self.cycle_time = (self.cycle_time + dt) % CYCLE_DURATION;
        let t = self.cycle_time / CYCLE_DURATION; // 0..1

        // sun_amount: 1 = sun, 0 = moon, with smooth transitions at boundaries
        let fade = 0.10;
        let sun_amount = /* smoothstep blend at t=0.0 and t=0.5 */;

        // Arc within each half-cycle
        let phase_t = if t < 0.5 { t / 0.5 } else { (t - 0.5) / 0.5 };
        let angle = smoothstep(phase_t) * PI;

        // Direction: sun right→left, moon left→right
        if let Some(light) = world.lighting.get_directional_light_mut(self.light) {
            light.direction = if t < 0.5 {
                Vec2::new(angle.cos(), -angle.sin())
            } else {
                Vec2::new(-angle.cos(), -angle.sin())
            };

            // Lerp color/intensity between sun and moon based on sun_amount
            let mc = MOON_COLOR;
            let sc = sun_color;
            light.color = Color::new(
                mc.r + (sc.r - mc.r) * sun_amount,
                mc.g + (sc.g - mc.g) * sun_amount,
                mc.b + (sc.b - mc.b) * sun_amount,
                1.0,
            );
            light.intensity = lerp(moon_intensity, sun_intensity, sun_amount);
        }
    }
}
```

Key points:
- **One continuous blend variable** (`sun_amount`) avoids flip-flop artifacts at transitions
- **`smoothstep` on `phase_t`** before computing the angle eases the light direction near the horizon, preventing shadow jitter from rapid direction changes
- **Moon intensity stays flat** — only the angle changes during the moon phase, giving a calm nighttime feel
- See `project/levels/day_night_cycle.rs` for the full implementation

## Level Transitions

See [Levels — Shared State & Events](levels.md#shared-state--events) for the full pattern. The short version:

```rust
// In a level:
ctx.shared.events.push(GameEvent::LevelComplete);

// In the Game:
let events: Vec<_> = self.shared.events.drain(..).collect();
for event in events {
    match event {
        GameEvent::LevelComplete => {
            self.active_level_mut().on_exit();
            self.level = ActiveLevel::Next(NextLevel::new());
            self.active_level_mut().on_enter();
        }
    }
}
```
