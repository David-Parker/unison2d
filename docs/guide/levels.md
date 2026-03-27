# Levels

Levels are the recommended way to organize a multi-scene game. Each level is a self-contained struct that owns its own `World` and implements the `Level<S>` trait.

## When to Use Levels

- **Single scene?** You don't need levels — just use `World` directly in your `Game` struct (see [Getting Started](getting-started.md)).
- **Multiple scenes?** Use levels. Each scene (menu, gameplay, boss fight) becomes a level.

## The Level Trait

```rust
pub trait Level<S = ()> {
    fn world(&self) -> &World;
    fn world_mut(&mut self) -> &mut World;
    fn update(&mut self, ctx: &mut LevelContext<S>);
    fn render(&mut self, ctx: &mut RenderContext);

    // Lifecycle hooks (default no-op):
    fn on_enter(&mut self) {}
    fn on_exit(&mut self) {}
    fn on_pause(&mut self) {}
    fn on_resume(&mut self) {}
}
```

`S` is your game's shared state type. If you don't need shared state, just use `Level` (which defaults to `Level<()>`).

## A Simple Level

```rust
use unison2d::*;
use unison2d::math::{Color, Vec2};
use unison2d::physics::{Material, mesh::create_ring_mesh};
use unison2d::input::KeyCode;

pub struct GameplayLevel {
    world: World,
    player: ObjectId,
}

impl GameplayLevel {
    pub fn new() -> Self {
        let mut world = World::new();
        world.set_background(Color::from_hex(0x1a1a2e));
        world.objects.set_gravity(-9.8);
        world.objects.set_ground(-5.0);

        let player = world.spawn_soft_body(SoftBodyDesc {
            mesh: create_ring_mesh(1.0, 0.4, 16, 6),
            material: Material::RUBBER,
            position: Vec2::new(0.0, 3.0),
            color: Color::from_hex(0xd4943a),
            texture: TextureId::NONE,
        });

        world.cameras.follow("main", player, 0.08);

        Self { world, player }
    }
}

impl Level for GameplayLevel {
    fn world(&self) -> &World { &self.world }
    fn world_mut(&mut self) -> &mut World { &mut self.world }

    fn update(&mut self, ctx: &mut LevelContext) {
        if ctx.input.is_key_pressed(KeyCode::ArrowRight) {
            self.world.objects.apply_force(self.player, Vec2::new(80.0, 0.0));
        }
        self.world.step(ctx.dt);
    }

    fn render(&mut self, ctx: &mut RenderContext) {
        self.world.auto_render(ctx.renderer);
    }
}
```

Key difference from `Game`: levels use `ctx.input` (raw `InputState`) instead of `engine.action_*()`. This is because input bindings live on the Engine, not on levels.

## Shared State & Events

Levels need a way to tell the Game "something happened" — like the player reaching a goal or dying. Unison uses a **shared state** pattern: you define your own state struct and event types, and levels push events through `ctx.shared`.

### Step 1: Define shared state and events

```rust
pub enum GameEvent {
    LevelComplete,
    PlayerDied,
}

pub struct SharedState {
    pub score: u32,
    pub events: Vec<GameEvent>,
}
```

### Step 2: Levels push events

```rust
impl Level<SharedState> for GameplayLevel {
    fn update(&mut self, ctx: &mut LevelContext<SharedState>) {
        // Gameplay logic...

        if player_reached_goal {
            ctx.shared.events.push(GameEvent::LevelComplete);
        }

        // Levels can also read/write shared state directly:
        ctx.shared.score += 10;

        self.world.step(ctx.dt);
    }
    // ...
}
```

### Step 3: Game drains events and transitions

```rust
impl Game for MyGame {
    type Action = Action;

    fn update(&mut self, engine: &mut Engine<Action>) {
        // Build context and update the active level
        let mut ctx = engine.level_context(&mut self.shared);
        self.active_level_mut().update(&mut ctx);

        // Drain events and react
        let events: Vec<_> = self.shared.events.drain(..).collect();
        for event in events {
            match event {
                GameEvent::LevelComplete => {
                    self.active_level_mut().on_exit();
                    self.level = ActiveLevel::NextLevel(NextLevel::new());
                    self.active_level_mut().on_enter();
                }
                GameEvent::PlayerDied => {
                    // Restart current level, show game over, etc.
                }
            }
        }
    }

    fn render(&mut self, engine: &mut Engine<Action>) {
        if let Some(renderer) = engine.renderer_mut() {
            let mut ctx = RenderContext { renderer };
            self.active_level_mut().render(&mut ctx);
        }
    }

    // ...
}
```

**Important:** Collect events into a `Vec` before iterating. This avoids borrow conflicts between `self.shared` and `self.level`.

### Why events instead of a built-in transition system?

- Games define their own event types — no engine enum to extend
- Games control *how* transitions happen — the engine doesn't force a stack or state machine
- Shared state can carry more than just events (score, settings, etc.)
- Simple games that don't need events just use `Level<()>` — zero ceremony

## Level Management

The engine does not dictate how you store or switch levels. Here's the recommended pattern using an enum:

```rust
enum ActiveLevel {
    Menu(MenuLevel),
    Gameplay(GameplayLevel),
    BossFight(BossFightLevel),
}

struct MyGame {
    level: ActiveLevel,
    shared: SharedState,
}

impl MyGame {
    fn active_level_mut(&mut self) -> &mut dyn Level<SharedState> {
        match &mut self.level {
            ActiveLevel::Menu(l) => l,
            ActiveLevel::Gameplay(l) => l,
            ActiveLevel::BossFight(l) => l,
        }
    }
}
```

This gives you:
- Type-safe level variants
- Each level can have its own fields and methods
- The `Game` is the state machine — it decides what transitions are valid

## Lifecycle Hooks

| Hook | When it's called |
|------|-----------------|
| `on_enter()` | After a level becomes the active level |
| `on_exit()` | Before a level is replaced |
| `on_pause()` | When another level is pushed on top (if you implement a stack) |
| `on_resume()` | When the level on top is popped (if you implement a stack) |

All hooks default to no-ops. Use them for setup/teardown that should happen on transitions (starting music, resetting timers, releasing resources).

## RenderContext

Levels receive a `RenderContext` in `render()` instead of raw renderer access. It wraps the renderer and adds compositing helpers:

```rust
fn render(&mut self, ctx: &mut RenderContext) {
    // Simple: render all layers through the main camera
    self.world.auto_render(ctx.renderer);
}
```

Worlds support named render layers for separating lit and unlit content. For example, a sky layer can be created before the default scene layer so sky elements are never darkened by shadows:

```rust
let sky = world.create_render_layer_before(
    "sky",
    RenderLayerConfig { lit: false, clear_color: sky_color },
    world.default_layer(),
);
world.draw_to(sky, sun_disc, 0);  // unlit sky layer
world.draw(tree_mesh, 0);          // default lit scene layer
```

For multi-camera setups, use render targets and overlay helpers:

```rust
fn render(&mut self, ctx: &mut RenderContext) {
    self.world.render_to_targets(ctx.renderer, &[
        ("overview", self.pip_target),
        ("main", RenderTargetId::SCREEN),
    ]);

    // Draw the overview as a PiP overlay with a white border
    ctx.draw_overlay_bordered(self.pip_texture, [0.02, 0.02], [0.25, 0.25], 0.005, Color::WHITE);
}
```

The `Game` builds the `RenderContext` using `engine.render_context()`:

```rust
fn render(&mut self, engine: &mut Engine<Action>) {
    if let Some(mut ctx) = engine.render_context() {
        level.render(&mut ctx);
    }
}
```

See [Patterns — Picture-in-Picture](patterns.md#picture-in-picture-pip) for a complete PiP example.

## Next Steps

- [Prefabs & Shared Code](prefabs.md) — extract shared spawning logic across levels
- [Patterns](patterns.md) — platformer movement, spawning, cameras
