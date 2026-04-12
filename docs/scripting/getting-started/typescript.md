# Getting Started with TypeScript

Write a game in TypeScript with full type safety, IDE autocomplete, and compile-time
error checking. TypeScript is transpiled to Lua at build time via
[TypeScriptToLua](https://typescripttolua.github.io/) (TSTL) — the engine runtime
is Lua-only.

> See `samples/ts-minimal/` for a complete working example.

---

## Prerequisites

- **Node.js** (18+) and **npm**
- **TypeScriptToLua** (`npm install -D typescript-to-lua`)

---

## Project Setup

A TypeScript game has the same Rust scaffold as a Lua game (see
[Lua Getting Started](lua.md) for `Cargo.toml`, `lib.rs`, and `build.rs`). The
difference is that your source files are `.ts` instead of `.lua`, and TSTL
transpiles them into the `scripts/` directory before the Rust build embeds them.

**Directory layout:**
```
your-game/
  project/
    lib.rs                    # Same scripted_game_entry! macro
    assets/
      scripts/                # TSTL output (generated, gitignored)
      scripts-src/            # Your TypeScript source
        main.ts
        scenes/
          demo.ts
        tsconfig.json
  package.json
  build.rs
```

### package.json

```json
{
  "private": true,
  "devDependencies": {
    "typescript-to-lua": "^1.27.0",
    "@typescript-to-lua/language-extensions": "^1.19.0"
  }
}
```

Run `npm install` after creating this file.

### tsconfig.json

Place this in your `scripts-src/` directory:

```json
{
  "compilerOptions": {
    "target": "ESNext",
    "lib": ["ESNext"],
    "module": "commonjs",
    "moduleResolution": "node",
    "strict": true,
    "baseUrl": ".",
    "rootDir": "./",
    "outDir": "../scripts",
    "types": ["@typescript-to-lua/language-extensions"],
    "paths": {
      "unison2d": ["../../../unison2d/crates/unison-scripting/types"]
    }
  },
  "tstl": {
    "luaTarget": "5.4",
    "luaLibImport": "require",
    "noImplicitSelf": true,
    "sourceMapTraceback": true
  },
  "include": ["**/*.ts", "../../../unison2d/crates/unison-scripting/types/**/*.d.ts"]
}
```

Key settings:
- **`outDir`** points to `../scripts` so TSTL output lands where the asset embedder expects it.
- **`noImplicitSelf: true`** in the `tstl` section matches the engine's `this: void` declarations.
- The **`include`** array pulls in the engine type declarations automatically.

### Type Declarations

The engine's TypeScript type declarations live at `crates/unison-scripting/types/`.
These declare all engine globals (`engine`, `input`, `events`, `World`, `Color`, `Rng`,
`math`, `debug`) with full JSDoc. The `tsconfig.json` include path above makes them
available without any explicit import.

---

## Build Workflow

Transpile TypeScript to Lua, then build the game as usual:

```bash
# One-shot transpile
npx tstl -p project/assets/scripts-src/tsconfig.json

# Watch mode (re-transpiles on save)
npx tstl -p project/assets/scripts-src/tsconfig.json --watch

# Then run the game (from platform/web/)
make dev
```

If your project Makefile has `ts` and `ts-watch` targets, use those instead:
```bash
make ts         # One-shot transpile
make ts-watch   # Watch mode
```

---

## Minimal Example

From `samples/ts-minimal/`:

**scripts-src/main.ts:**
```typescript
import * as demo from "./scenes/demo";

const game: Game = {
    init() {
        engine.set_scene(demo);
    },
    update(dt: number) {},
    render() {},
};

export = game;
```

**scripts-src/scenes/demo.ts:**
```typescript
let world: World;
let box_id: ObjectId;

const scene: Scene = {
    on_enter() {
        world = World.new();
        world.set_background(0x1a1a2e);
        world.set_gravity(-9.8);
        world.set_ground(-4.5);

        box_id = world.spawn_rigid_body({
            collider: "aabb",
            half_width: 0.5,
            half_height: 0.5,
            position: [0, 2],
            color: 0xFF6600,
        });

        world.camera_follow("main", box_id, 0.1);

        events.on("test_event", (data) => {
            debug.log("received test_event");
        });
    },

    update(dt: number) {
        if (input.is_key_pressed("ArrowLeft") || input.is_key_pressed("A")) {
            world.apply_force(box_id, -5, 0);
        }
        if (input.is_key_pressed("ArrowRight") || input.is_key_pressed("D")) {
            world.apply_force(box_id, 5, 0);
        }
        if (input.is_key_just_pressed("Space") && world.is_grounded(box_id)) {
            world.apply_impulse(box_id, 0, 5);
        }
        world.step(dt);
    },

    render() {
        world.auto_render();
    },

    on_exit() {
        events.clear();
        world = undefined!;
        box_id = undefined!;
    },
};

export = scene;
```

---

## Key Differences from Lua

### Module syntax

**Lua:** `local menu = require("scenes/menu")` / `return scene`

**TypeScript:** `import * as menu from "./scenes/menu"` / `export = scene`

TSTL translates `import`/`export` to `require`/`return` in the generated Lua.

### Method calls

In Lua, instance methods use `:` syntax (`world:step(dt)`). In TypeScript, use
regular dot notation (`world.step(dt)`) — TSTL handles the translation. The type
declarations use `this: World` parameters to enforce correct usage.

### Multi-return values

Lua functions that return multiple values use `LuaMultiReturn` in TypeScript:

```typescript
// Lua: local w, h = engine.screen_size()
const [w, h] = engine.screen_size();

// Lua: local x, y = world:get_position(id)
const [x, y] = world.get_position(id);
```

### Clearing references

Lua uses `nil` to clear a variable. TypeScript uses `undefined!` (the `!` asserts
non-null to satisfy strict mode):

```typescript
// Lua: world = nil
world = undefined!;
```

### Tables vs objects

Lua tables become TypeScript object literals. Positions use arrays instead of tables:

```typescript
// Lua: position = {0, 2}
position: [0, 2]
```

---

## Next Steps

- **[Concepts](../concepts.md)** — Lifecycle, scenes, events, worlds (with TypeScript examples)
- **[API Reference](../api-reference.md)** — Every global with Lua + TypeScript signatures side-by-side
- **[Hot Reload](../hot-reload.md)** — TSTL watch mode + engine hot reload
