# Getting Started with Lua Scripting

Build a game entirely in Lua — no Rust game code required. The `unison-scripting` crate
embeds a Lua 5.4 VM and implements the `Game` trait for you.

## Setup

**Cargo.toml:**
```toml
[dependencies]
unison2d          = { path = "unison2d/crates/unison2d" }
unison-scripting  = { path = "unison2d/crates/unison-scripting" }
unison-web        = { path = "unison2d/crates/unison-web" }
unison-assets     = { path = "unison2d/crates/unison-assets" }
wasm-bindgen      = "0.2"
```

**lib.rs (your entire Rust file):**
```rust
use wasm_bindgen::prelude::*;
use unison_scripting::ScriptedGame;

mod assets { include!(concat!(env!("OUT_DIR"), "/assets.rs")); }

#[wasm_bindgen(start)]
pub fn main() {
    let game = ScriptedGame::from_asset("scripts/main.lua", assets::ASSETS);
    unison_web::run(game);
}
```

**build.rs:**
```rust
fn main() {
    unison_assets::embed("project/assets");
}
```

Place all your Lua scripts under `project/assets/scripts/`. The entry point is
`scripts/main.lua`.

## Script Lifecycle

Your `main.lua` must return a table with three functions:

```lua
local game = {}

function game.init()
    -- Called once at startup. Set up your world, load textures, wire up events.
end

function game.update(dt)
    -- Called every frame (~60 Hz). dt is the timestep in seconds.
    -- Step physics and handle input here.
end

function game.render()
    -- Called every frame. Call world:auto_render() to draw the world.
end

return game
```

Missing functions are silently ignored — a stub `update` or `render` that does nothing
is fine.

## Minimal Working Example

A colored rectangle that moves left/right with arrow keys:

```lua
local game = {}
local world, box_id

function game.init()
    world = World.new()
    world:set_background(0x1a1a2e)
    world:set_gravity(-9.8)
    world:set_ground(-4.5)

    -- Spawn a rigid body
    box_id = world:spawn_rigid_body({
        collider = "aabb",
        half_width = 0.6,
        half_height = 0.6,
        position = {0, 2},
        color = 0xe74c3c,
    })

    -- Camera tracks the box
    world:camera_follow("main", box_id, 0.1)
end

function game.update(dt)
    if input.is_key_pressed("ArrowRight") then
        world:apply_force(box_id, 60, 0)
    end
    if input.is_key_pressed("ArrowLeft") then
        world:apply_force(box_id, -60, 0)
    end
    if input.is_key_just_pressed("Space") and world:is_grounded(box_id) then
        world:apply_impulse(box_id, 0, 8)
    end
    world:step(dt)
end

function game.render()
    world:auto_render()
end

return game
```

Run with `make dev` from `platform/web/`.

## Multi-File Scripts with require()

Split your game into modules using `require()`. All `.lua` files under
`project/assets/scripts/` are automatically registered as modules.

**File layout:**
```
project/assets/scripts/
├── main.lua
└── scenes/
    ├── menu.lua
    └── gameplay.lua
```

**Loading a module:**
```lua
-- In main.lua — loads scripts/scenes/menu.lua
local menu = require("scenes/menu")
engine.set_scene(menu)
```

**Module format** — each module file returns a table:
```lua
-- scenes/menu.lua
local menu = {}

function menu.on_enter() ... end
function menu.update(dt) ... end
function menu.render() ... end
function menu.on_exit() ... end

return menu
```

Modules are cached after the first `require()` — requiring the same path twice returns
the same table. To force a fresh table (e.g. when re-entering a scene), create a factory
function instead:

```lua
-- scenes/gameplay.lua
local function make_scene()
    local scene = {}
    local world  -- fresh local per call
    function scene.on_enter() world = World.new() ... end
    ...
    return scene
end
return make_scene  -- return the factory, not the scene
```

## Scene System

For multi-scene games, use `engine.set_scene()` instead of returning update/render from
the top-level game table:

```lua
-- main.lua
local game = {}

function game.init()
    local menu = require("scenes/menu")
    engine.set_scene(menu)
end

-- update/render are optional when using scenes
return game
```

See [api-reference.md](api-reference.md) for the full scene API, and
[migration-guide.md](migration-guide.md) if you are porting an existing Rust game.
