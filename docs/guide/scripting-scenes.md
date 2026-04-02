# Scenes (Lua Scripting)

Scenes are the Lua equivalent of the Rust `Level` trait. Each scene is a Lua table with lifecycle functions. The engine's scene system handles dispatching `update` and `render` to the active scene automatically.

> This guide covers the Lua scripting approach. For the Rust `Level` trait, see [levels.md](levels.md).

## When to Use Scenes

- **Single scene?** You don't need scenes — put your logic directly in `game.init()`, `game.update(dt)`, and `game.render()`.
- **Multiple scenes?** Use scenes. Each screen (menu, gameplay, boss fight) becomes a scene module.

## Scene Table Format

A scene is a Lua table with these optional functions:

```lua
local scene = {}

function scene.on_enter()
    -- Called when this scene becomes active.
    -- Set up world, spawn objects, load textures.
end

function scene.update(dt)
    -- Called every frame while this scene is active.
    -- Drive game logic, step physics.
end

function scene.render()
    -- Called every frame after update.
    -- Call world:auto_render() or custom draw commands.
end

function scene.on_exit()
    -- Called when switching away from this scene.
    -- Clean up resources.
end

return scene
```

All functions are optional. If `update` or `render` is missing, it's a no-op for that frame.

## Setting the Initial Scene

In your `main.lua`, use `engine.set_scene()` to activate the first scene:

```lua
local game = {}

function game.init()
    local main_level = require("scenes/main_level")
    engine.set_scene(main_level)
end

function game.update(dt)
    -- Scene system handles dispatch — nothing needed here.
end

function game.render()
    -- Scene system handles dispatch — nothing needed here.
end

return game
```

Once a scene is active, `game.update()` and `game.render()` are **not** called — the engine dispatches directly to the scene's `update(dt)` and `render()`.

## Switching Scenes

Use `engine.switch_scene()` to transition between scenes. This calls `on_exit()` on the old scene, then `on_enter()` on the new scene:

```lua
events.on("level_complete", function()
    local next_level = require("scenes/lighting")
    engine.switch_scene(next_level)
end)
```

Scene switches can be triggered from anywhere — inside `update()`, inside event callbacks, or inside UI button handlers.

## Complete Example: Multi-Level Game

### main.lua

```lua
local game = {}

function game.init()
    local initial_scene = require("scenes/main_level")
    engine.set_scene(initial_scene)

    events.on("level_complete", function()
        local lighting = require("scenes/lighting")
        engine.switch_scene(lighting)
    end)
end

function game.update(dt) end
function game.render() end

return game
```

### scenes/main_level.lua

```lua
local shared = require("scenes/shared")

local scene = {}
local world, donut, trigger_box
local triggered = false

function scene.on_enter()
    triggered = false
    local donut_tex = engine.load_texture("textures/donut-pink.png")

    world = shared.new_world()
    shared.spawn_ground(world)

    donut = shared.spawn_donut(world, 0, 3.5, donut_tex)
    world:set_z_order(donut, 100)

    trigger_box = world:spawn_rigid_body({
        collider = "aabb",
        half_width = 0.5, half_height = 0.5,
        position = {6, -3},
        color = 0xe74c3c, is_static = true,
    })

    world:camera_follow_with_offset("main", donut, 0.08, 0, 3.5)
end

function scene.update(dt)
    shared.drive_donut(world, donut, dt)
    world:step(dt)

    if not triggered and world:is_touching(donut, trigger_box) then
        triggered = true
        events.emit("level_complete")
    end
end

function scene.render()
    world:auto_render()
end

function scene.on_exit()
    -- Cleanup
end

return scene
```

### scenes/shared.lua

Shared utilities keep scene code DRY:

```lua
local shared = {}

function shared.new_world()
    local w = World.new()
    w:set_background(0x1a1a2e)
    w:set_gravity(-9.8)
    w:set_ground(-4.5)
    w:lighting_set_enabled(true)
    w:lighting_set_ambient(0.12, 0.12, 0.15, 1.0)
    return w
end

function shared.spawn_donut(world, x, y, texture)
    return world:spawn_soft_body({
        mesh = "ring",
        mesh_params = {1.0, 0.25, 24, 8},
        material = {density = 900, edge_compliance = 5e-6, area_compliance = 2e-5},
        position = {x, y}, color = 0xFFFFFF, texture = texture,
    })
end

function shared.drive_donut(world, donut, dt)
    local move_x = input.axis_x()
    if math.abs(move_x) < 0.01 then
        if input.is_key_pressed("ArrowRight") then move_x = 1
        elseif input.is_key_pressed("ArrowLeft") then move_x = -1
        end
    end
    if math.abs(move_x) > 0.01 then
        world:apply_force(donut, move_x * 80, 0)
        if world:is_grounded(donut) then
            world:apply_torque(donut, -move_x * 20, dt)
        end
    end
    if input.is_key_just_pressed("Space") and world:is_grounded(donut) then
        world:apply_impulse(donut, 0, 10)
    end
end

return shared
```

## Scene Lifecycle

| Hook | When | Typical Use |
|------|------|-------------|
| `on_enter()` | Scene becomes active (via `set_scene` or `switch_scene`) | Create world, spawn objects, load textures |
| `update(dt)` | Every frame while active | Game logic, physics step |
| `render()` | Every frame after update | `world:auto_render()`, custom draw commands |
| `on_exit()` | Another scene replaces this one (via `switch_scene`) | Cleanup, release resources |

## Events for Communication

Scenes communicate through the string-keyed event system rather than returning values:

```lua
-- In a scene's update():
events.emit("player_scored", { points = 100 })

-- In main.lua's init():
events.on("player_scored", function(data)
    print("Scored " .. data.points .. " points!")
end)
```

This decouples scenes from each other — a scene doesn't need to know what happens when it emits an event.

## Menu with UI

Scenes without physics can skip `World` entirely and use the UI system:

```lua
local menu = {}
local ui_handle

function menu.on_enter()
    ui_handle = engine.create_ui("fonts/DejaVuSans-Bold.ttf")

    events.on("start_game", function()
        local level = require("scenes/main_level")
        engine.switch_scene(level)
    end)
end

function menu.update(dt)
    -- No physics
end

function menu.render()
    engine.set_background(0x0e0e1a)
    ui_handle:frame({
        { type = "column", anchor = "center", gap = 12, children = {
            { type = "label", text = "My Game", font_size = 32 },
            { type = "button", text = "Play", on_click = "start_game",
              width = 200, height = 48, font_size = 20 },
        }},
    })
end

function menu.on_exit() end

return menu
```

Button `on_click` values are event names — clicking the button emits that event.

## Module Loading

Scene files are loaded via Lua's `require()`. The engine maps embedded asset paths to module names:

| Asset Path | require() Call |
|------------|---------------|
| `scripts/scenes/main_level.lua` | `require("scenes/main_level")` |
| `scripts/scenes/shared.lua` | `require("scenes/shared")` |
| `scripts/utils/math_helpers.lua` | `require("utils/math_helpers")` |

The `scripts/` prefix and `.lua` suffix are stripped automatically. Modules are cached by Lua's `require` — calling `require("scenes/shared")` multiple times returns the same table.

## Comparison: Rust Levels vs Lua Scenes

| Rust Level | Lua Scene |
|------------|-----------|
| `impl Level<S> for MyLevel` | `local scene = {}` table |
| `fn world(&self) -> &World` | Scene owns `world` as a local variable |
| `fn on_enter(&mut self)` | `function scene.on_enter()` |
| `fn update(&mut self, ctx: &mut Ctx<S>)` | `function scene.update(dt)` |
| `fn render(&mut self, ctx: &mut Ctx<S>)` | `function scene.render()` |
| `ctx.shared.events.push(Event)` | `events.emit("event_name", data)` |
| `enum ActiveLevel { A(A), B(B) }` | `engine.switch_scene(scene_table)` |

The Lua approach trades compile-time type safety for simplicity — no trait implementations, no generic parameters, no enum dispatch boilerplate.

## Next Steps

- [Scripting API Reference](../api/scripting.md) — full API for World, input, events, lighting, UI
- [Patterns](patterns.md) — platformer movement, spawning, cameras
