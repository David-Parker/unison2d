# Concepts

Language-neutral guide to Unison 2D's core concepts. Each section includes both
Lua and TypeScript examples.

---

## Game Lifecycle

Every game has the same three-phase lifecycle: **init**, **update**, **render**.

The entry script (`main.lua` or `main.ts`) returns a game table:

**Lua:**
```lua
local game = {}

function game.init()
    -- Called once at startup
end

function game.update(dt)
    -- Called every frame; dt is seconds since last frame
end

function game.render()
    -- Called every frame; draw here
end

return game
```

**TypeScript:**
```typescript
const game: Game = {
    init() {
        // Called once at startup
    },
    update(dt: number) {
        // Called every frame; dt is seconds since last frame
    },
    render() {
        // Called every frame; draw here
    },
};

export = game;
```

All three functions are optional. When using scenes (below), you typically only
implement `init` at the game level.

---

## Scenes

A scene is a table with optional lifecycle hooks: `on_enter`, `update`, `render`,
`on_exit`. When a scene is active, its `update` and `render` replace the game-level
ones.

### Setting the First Scene

**Lua:**
```lua
function game.init()
    local menu = require("scenes/menu")
    unison.scenes.set(menu)
end
```

**TypeScript:**
```typescript
import * as menu from "./scenes/menu";

const game: Game = {
    init() {
        unison.scenes.set(menu);
    },
};

export = game;
```

### Scene Lifecycle Hooks

**Lua:**
```lua
local scene = {}
local world

function scene.on_enter()
    world = World.new()
    -- set up world, spawn objects, register event listeners
end

function scene.update(dt)
    world:step(dt)
end

function scene.render()
    world:render()
end

function scene.on_exit()
    unison.events.clear()
    world = nil
end

return scene
```

**TypeScript:**
```typescript
let world: World;

const scene: Scene = {
    on_enter() {
        world = World.new();
        // set up world, spawn objects, register event listeners
    },

    update(dt: number) {
        world.step(dt);
    },

    render() {
        world.render();
    },

    on_exit() {
        unison.events.clear();
        world = undefined!;
    },
};

export = scene;
```

### Switching Scenes

Use `unison.scenes.set()` to transition between scenes. It calls `on_exit` on
the current scene (if any), then `on_enter` on the new one:

**Lua:**
```lua
unison.scenes.set(require("scenes/gameplay"))
```

**TypeScript:**
```typescript
import * as gameplay from "./scenes/gameplay";
unison.scenes.set(gameplay);
```

### Fresh State per Visit

Lua's `require()` caches modules — requiring the same path twice returns the same
table. If a scene stores mutable state, use a factory function:

**Lua:**
```lua
-- scenes/gameplay.lua
return function()
    local scene = {}
    local world
    function scene.on_enter() world = unison.World.new() end
    function scene.update(dt) world:step(dt) end
    function scene.render() world:render() end
    function scene.on_exit() world = nil end
    return scene
end

-- In another file:
local make_gameplay = require("scenes/gameplay")
unison.scenes.set(make_gameplay())  -- fresh instance
```

**TypeScript:**
```typescript
// scenes/gameplay.ts
function makeScene(): Scene {
    let world: World;
    return {
        on_enter() { world = unison.World.new(); },
        update(dt: number) { world.step(dt); },
        render() { world.render(); },
        on_exit() { world = undefined!; },
    };
}

export = makeScene;

// In another file:
import makeGameplay = require("./scenes/gameplay");
unison.scenes.set(makeGameplay());
```

---

## Events

The engine provides a string-keyed pub/sub event bus plus collision callbacks.

### String Events

Register a listener with `unison.events.on()`, fire with `unison.events.emit()`. Callbacks
execute at the end of the frame.

**Lua:**
```lua
unison.events.on("level_complete", function(data)
    unison.debug.log("Score:", data.score)
    unison.scenes.set(require("scenes/menu"))
end)

unison.events.emit("level_complete", { score = 1234 })
```

**TypeScript:**
```typescript
unison.events.on("level_complete", (data) => {
    unison.debug.log("Score:", data.score);
    unison.scenes.set(menu);
});

unison.events.emit("level_complete", { score: 1234 });
```

### Collision Events

Three levels of specificity. Collision callbacks are registered on the World, not the
event bus:

**Lua:**
```lua
-- Every collision pair each frame
world:on_collision(function(a, b, info)
    unison.debug.log("collision between", a, b)
end)

-- When a specific object collides with anything
world:on_collision_with(player_id, function(other, info)
    unison.debug.log("player hit", other)
end)

-- When two specific objects collide
world:on_collision_between(player_id, spike_id, function(info)
    unison.debug.log("ouch! penetration:", info.penetration)
end)
```

**TypeScript:**
```typescript
world.on_collision((a, b, info) => {
    unison.debug.log("collision between", a, b);
});

world.on_collision_with(player_id, (other, info) => {
    unison.debug.log("player hit", other);
});

world.on_collision_between(player_id, spike_id, (info) => {
    unison.debug.log("ouch! penetration:", info.penetration);
});
```

### Cleaning Up

Call `unison.events.clear()` in `on_exit` to remove all string-keyed event handlers and
pending events. Collision handlers registered on the World are not cleared by `unison.events.clear()`.

---

## Worlds

A `World` is a self-contained simulation: physics objects, cameras, lighting, and
rendering. Each scene typically creates its own World.

**Lua:**
```lua
local world = unison.World.new()
world:set_background(0x1a1a2e)
world:set_gravity(-9.8)
world:set_ground(-4.5)

local box_id = world.objects:spawn_rigid_body({
    collider = "aabb",
    half_width = 0.5,
    half_height = 0.5,
    position = {0, 2},
    color = 0xFF6600,
})

world.cameras:follow("main", box_id, { smoothing = 0.1 })
```

**TypeScript:**
```typescript
const world = unison.World.new();
world.set_background(0x1a1a2e);
world.set_gravity(-9.8);
world.set_ground(-4.5);

const box_id = world.objects.spawn_rigid_body({
    collider: "aabb",
    half_width: 0.5,
    half_height: 0.5,
    position: [0, 2],
    color: 0xFF6600,
});

world.cameras.follow("main", box_id, { smoothing: 0.1 });
```

Call `world:step(dt)` / `world.step(dt)` in update and `world:render()` /
`world.render()` in render.

---

## Multi-File Organization

### Lua: require()

All `.lua` files under `project/assets/scripts/` are registered as modules.
Use forward-slash paths relative to the scripts directory:

```lua
local menu = require("scenes/menu")
local shared = require("lib/shared")
```

### TypeScript: import / export

Use standard TypeScript imports with relative paths. TSTL translates them to
`require()` calls in the output Lua:

```typescript
import * as menu from "./scenes/menu";
import * as shared from "./lib/shared";
```

Export a module's value with `export =`:

```typescript
const scene: Scene = { ... };
export = scene;
```

---

## Next Steps

- **[API Reference](api-reference.md)** — Every global with Lua + TypeScript signatures
- **[Hot Reload](hot-reload.md)** — Live code reloading for both languages
