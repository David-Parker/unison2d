# Migration Guide: Rust → Lua

This guide is for developers with an existing Rust `Game` implementation who want to
port to Lua scripting. The concepts map cleanly — the Lua layer simplifies most of them.

## Before You Start

Lua scripting replaces your Rust `lib.rs` game code. Your Cargo.toml swap:

```toml
# Before
[dependencies]
unison2d = { path = "unison2d/crates/unison2d" }

# After
[dependencies]
unison2d         = { path = "unison2d/crates/unison2d" }
unison-scripting = { path = "unison2d/crates/unison-scripting" }
```

Your new `lib.rs` is a three-liner:

```rust
// Before: hundreds of lines of Rust game code
// After:
mod assets { include!(concat!(env!("OUT_DIR"), "/assets.rs")); }
#[wasm_bindgen(start)]
pub fn main() {
    unison_web::run(ScriptedGame::from_asset("scripts/main.lua", assets::ASSETS));
}
```

---

## World Setup

**Rust:**
```rust
fn init(&mut self, engine: &mut Engine<Action>) {
    self.world.set_background(Color::from_hex(0x1a1a2e));
    self.world.objects.set_gravity(-9.8);
    self.world.objects.set_ground(-4.5);
    self.world.objects.set_ground_restitution(0.2);
}
```

**Lua:**
```lua
function game.init()
    world = World.new()
    world:set_background(0x1a1a2e)
    world:set_gravity(-9.8)
    world:set_ground(-4.5)
    world:set_ground_restitution(0.2)
end
```

Key difference: in Lua, `World` is a value you hold in a local variable. It is not owned
by the `Game` struct — you keep it in a module-level local.

---

## Spawning Objects

**Rust:**
```rust
let player = self.world.spawn_soft_body(SoftBodyDesc {
    mesh: create_ring_mesh(1.0, 0.4, 16, 6),
    material: Material::RUBBER,
    position: Vec2::new(0.0, 3.0),
    color: Color::from_hex(0xd4943a),
    texture: tex_id,
});
```

**Lua:**
```lua
local player = world:spawn_soft_body({
    mesh = "ring",
    mesh_params = {1.0, 0.4, 16, 6},
    material = "rubber",
    position = {0, 3},
    color = 0xd4943a,
    texture = tex_id,
})
```

Mesh creation is inlined into the descriptor — no separate mesh factory call.

---

## Input

**Rust:**
```rust
// Separate action enum + binding step:
engine.bind_key(KeyCode::ArrowRight, Action::MoveRight);
// ...
fn update(&mut self, engine: &mut Engine<Action>) {
    if engine.action_is_active(Action::MoveRight) {
        self.world.objects.apply_force(self.player, Vec2::new(80.0, 0.0));
    }
}
```

**Lua:**
```lua
-- No action enum, no binding step:
function game.update(dt)
    if input.is_key_pressed("ArrowRight") then
        world:apply_force(player, 80, 0)
    end
end
```

Lua reads input directly — no `Action` enum, no `bind_key` call, no `action_axis`.
For an analog axis equivalent, combine two keys:

```lua
local move_x = 0
if input.is_key_pressed("ArrowRight") then move_x = move_x + 1 end
if input.is_key_pressed("ArrowLeft")  then move_x = move_x - 1 end
world:apply_force(player, move_x * 80, 0)
```

---

## Events

**Rust:**
```rust
// Define a typed event enum + emit via SharedState:
enum Event { LevelComplete, Died }
shared.events.push(Event::LevelComplete);

// Drain and match in update:
for event in shared.events.drain() {
    match event {
        Event::LevelComplete => self.switch_to_menu(),
        Event::Died => { ... }
    }
}
```

**Lua:**
```lua
-- Emit anywhere:
events.emit("level_complete")

-- Listen anywhere (usually in init or on_enter):
events.on("level_complete", function()
    engine.switch_scene(require("scenes/menu"))
end)
```

No enum, no drain loop. Callbacks fire automatically at end of frame.

Event data is passed as an optional table:
```lua
events.emit("player_died", { score = 1234 })
events.on("player_died", function(data)
    print("Score:", data.score)
end)
```

---

## Levels → Scenes

The Rust `Level<S>` trait maps directly to a Lua scene table.

**Rust:**
```rust
struct GameplayLevel { world: World, player: ObjectId }

impl Level<SharedState> for GameplayLevel {
    fn on_enter(&mut self, _ctx: &mut Ctx<SharedState>) {
        self.world = World::new();
        self.player = /* spawn */;
    }
    fn update(&mut self, ctx: &mut Ctx<SharedState>) { ... }
    fn render(&mut self, ctx: &mut Ctx<SharedState>) { ... }
    fn on_exit(&mut self, _ctx: &mut Ctx<SharedState>) { }
}
```

**Lua:**
```lua
local scene = {}
local world, player

function scene.on_enter()
    world = World.new()
    player = world:spawn_soft_body({ ... })
end

function scene.update(dt)
    world:step(dt)
end

function scene.render()
    world:auto_render()
end

function scene.on_exit() end

return scene
```

The `Ctx<S>` parameter disappears entirely — `world`, `input`, `events`, and `engine`
are all globals.

**Switching scenes:**
```lua
-- Rust: set active level enum variant + re-init
-- Lua:
engine.switch_scene(require("scenes/menu"))
```

### Shared State Across Scenes

Rust used a `SharedState` struct passed as `S`. In Lua, use module-level locals in a
shared module:

**shared.lua:**
```lua
local shared = {}
shared.score = 0
shared.level_index = 1
return shared
```

**In any scene:**
```lua
local shared = require("scenes/shared")
shared.score = shared.score + 100
```

---

## Lighting

**Rust:**
```rust
self.world.lighting.set_enabled(true);
self.world.lighting.set_ambient(Color::new(0.05, 0.05, 0.1, 1.0));
let light = self.world.lighting.add_point_light(PointLightDesc {
    position: Vec2::new(0.0, 3.5),
    color: Color::from_hex(0xFFE6B3),
    intensity: 1.5,
    radius: 8.0,
    ..Default::default()
});
self.world.lighting.set_follow(light, self.player);
```

**Lua:**
```lua
world:lighting_set_enabled(true)
world:lighting_set_ambient(0.05, 0.05, 0.1, 1.0)
local light = world:add_point_light({
    position = {0, 3.5},
    color = 0xFFE6B3,
    intensity = 1.5,
    radius = 8.0,
})
world:light_follow(light, player)
```

---

## UI

**Rust:**
```rust
let mut ui = Ui::<MyEvent>::new(font_bytes, renderer, sink);
ui.frame(UiTree::new(vec![
    UiNode::column()
        .with_children(vec![
            UiNode::label("Play").with_text_style(TextStyle::default().font_size(24.0)),
            UiNode::button("Start").with_on_click(MyEvent::Start),
        ])
]), input, screen_size, dt, &mut world, renderer);
```

**Lua:**
```lua
local ui = engine.create_ui("fonts/DejaVuSans-Bold.ttf")

-- In render:
ui:frame({
    { type = "column", children = {
        { type = "label", text = "Play", font_size = 24 },
        { type = "button", text = "Start", on_click = "start_game" },
    }},
})
events.on("start_game", function()
    engine.switch_scene(require("scenes/gameplay"))
end)
```

---

## Common Gotchas

### Method call syntax: `:` vs `.`

Lua distinguishes between method calls (`:`) and function calls (`.`).

```lua
-- CORRECT: world methods use ':'
world:step(dt)
world:spawn_soft_body({ ... })
world:camera_follow("main", id, 0.1)

-- WRONG: this passes the table as an explicit first arg and shifts all others
world.step(dt)        -- error or wrong behavior
```

Use `:` for anything that looks like `obj:method()` in the API reference. Use `.` only
for module-level functions like `World.new()`, `Color.hex()`, `input.is_key_pressed()`.

### Multi-return values

Several world methods return two values. Capture both:

```lua
local x, y = world:get_position(player)
local vx, vy = world:get_velocity(player)
local w, h = engine.screen_size()

-- If you only capture one, you get the first and discard the rest:
local x = world:get_position(player)  -- y is silently dropped
```

### Lua 1-based indexing

Lua tables use 1-based indexing. This matters when working with the `mesh_params` array
or any table you iterate with `ipairs`:

```lua
local params = {1.0, 0.25, 24, 8}
-- params[1] == 1.0   (outer_radius)
-- params[4] == 8     (radial_divisions)
```

### Nil vs false

In Lua, only `nil` and `false` are falsy. `0` and `""` are truthy — unlike C/Rust:

```lua
local id = 0   -- object ID 0 is falsy in C, but TRUTHY in Lua
if id then
    -- this branch IS taken even though id == 0
end
```

Don't use integer IDs as booleans. Use a separate boolean flag or check `id ~= nil`.

### require() caches modules

`require("scenes/menu")` returns the *same table* every call. If your scene stores
mutable state at module level, re-requiring it will give you the existing dirty state.

Use a factory function for scenes that need fresh state each time:

```lua
-- scenes/gameplay.lua
return function()  -- factory
    local scene = {}
    local world    -- created fresh in on_enter
    ...
    return scene
end

-- In main.lua or another scene:
local make_gameplay = require("scenes/gameplay")
engine.switch_scene(make_gameplay())  -- fresh instance
```
