# Lua/TypeScript Scripting Support for Unison 2D

## Context

Game code is currently written in Rust, implementing the `Game` trait directly. This limits game development to Rust developers and prevents hot-reload. The goal is to add a Lua 5.4 scripting layer so game code can be written in Lua (or TypeScript via [TypeScriptToLua](https://github.com/TypeScriptToLua/TypescriptToLua)), while the engine remains in Rust. All three platforms (Web/iOS/Android) must continue to work.

A new crate `unison-scripting` will implement the `Game` trait internally and forward lifecycle calls into an embedded Lua VM. **The Rust game-code path will be deprecated** once scripting is validated — scripting becomes the only way to write games. The engine itself stays in Rust.

**Branch:** All work on `feat/lua-scripting`.

---

## Engine Simplifications to Document

These Rust abstractions were designed for Rust ergonomics and don't map to Lua. The scripting layer replaces them with simpler equivalents:

| Rust Abstraction | Complexity | Lua Replacement |
|---|---|---|
| **EventBus\<C\>** (476 lines) — type-erased pub/sub with `TypeId`-keyed channels, `Box<dyn Any>` downcasting, deferred dispatch, re-entrance safety, `EventSink` handles | Very high | String-keyed event channels: `events.emit("name", data)` / `events.on("name", fn)` |
| **Level\<S\>** (109 lines) — generic trait over shared state `S`, with auto-orchestrated `run_update`/`run_render` | Medium | Scene tables with `on_enter`/`update`/`render`/`on_exit` functions |
| **Ctx\<S\>** (243 lines) — unified context with lifetime refs, generic over shared state | Medium | Individual globals: `input`, `events`, `dt` passed as args |
| **Engine\<A\>** — generic over Action enum `A: Copy+Eq+Hash` for input mapping | Medium | Scripted games use `NoAction` unit type; Lua queries `input` directly |
| **Prefab trait** — spawn abstraction with `World` + `Vec2` | Low | Plain Lua factory functions |
| **SharedState generics** — `S` threaded through `Engine<A>`, `Ctx<S>`, `Level<S>` | Medium | Global Lua state table |

---

## Phase 1: Foundation

**Goal:** Validate mlua + WASM compatibility, create `unison-scripting` crate, prove Lua runs on all platforms.

### Phase 1a: WASM Spike (de-risk first)

Before building the full crate, validate that `mlua` with vendored Lua 5.4 compiles and executes on `wasm32-unknown-unknown`:

1. Create a minimal test crate (outside the engine) that depends on `mlua = { features = ["lua54", "vendored"] }`
2. Compile to WASM: `cargo build --target wasm32-unknown-unknown`
3. Run a trivial Lua script (`print("hello")`) in a browser via wasm-bindgen
4. If this fails, evaluate fallbacks: `cc` crate to compile Lua 5.4 C source directly, or `wasmoon` (Lua 5.4 for WASM)

**Validation:** Lua VM initializes and executes a script in a WASM context in the browser.

### Phase 1b: Create the Crate

1. **Create branch** `feat/lua-scripting`

2. **Create crate** `unison2d/crates/unison-scripting/`
   - `Cargo.toml` — depends on `unison2d`, `mlua` (version 0.10, features: `lua54`, `vendored`, `send`)
   - `src/lib.rs` — `ScriptedGame` struct implementing `Game` trait with `type Action = NoAction`
   - `src/bridge.rs` — minimal Lua bindings (set background color, draw rect, get screen size)
   - `vendored` feature compiles Lua 5.4 from C source (critical for WASM, iOS static linking, Android NDK)

3. **Add workspace member** in `unison2d/Cargo.toml`

4. **Add dependency** in root `Cargo.toml`:
   ```toml
   unison-scripting = { path = "unison2d/crates/unison-scripting" }
   ```

5. **Replace game entry points** in `project/lib.rs`:
   - Instantiate `ScriptedGame` instead of `DonutGame`
   - Scripts loaded from assets: `project/assets/scripts/main.lua`

6. **Create hello-world script** `project/assets/scripts/main.lua`:
   ```lua
   local game = {}
   function game.init()
       engine.set_background(0.1, 0.1, 0.12)
   end
   function game.update(dt) end
   function game.render()
       engine.draw_rect(0, 0, 2, 2, 1, 0.2, 0.2)
   end
   return game
   ```

### Key files
- Create: `unison2d/crates/unison-scripting/Cargo.toml`
- Create: `unison2d/crates/unison-scripting/src/lib.rs`
- Create: `unison2d/crates/unison-scripting/src/bridge.rs`
- Modify: `unison2d/Cargo.toml` (workspace members)
- Modify: `Cargo.toml` (add unison-scripting dependency, remove old DonutGame deps eventually)
- Modify: `project/lib.rs` (replace `DonutGame` with `ScriptedGame`)
- Create: `project/assets/scripts/main.lua`

### E2E Tests (`unison-tests`)

Add a new test file `scripting_foundation.rs` covering:
- Lua VM initializes successfully (create `Lua`, execute trivial script)
- `ScriptedGame` implements `Game` trait — can be constructed and `init`/`update`/`render` called in sequence
- Script lifecycle: returned table has `init`/`update`/`render` functions, all are called
- Script error handling: malformed script produces a recoverable error, not a panic
- Script missing lifecycle function: graceful fallback (e.g., missing `render` is a no-op)

### Doc Updates

- Create `unison2d/docs/api/scripting.md` — initial doc for the `unison-scripting` crate (purpose, ScriptedGame struct, Lua lifecycle)
- Update `unison2d/CLAUDE.md` crate table — add `unison-scripting` entry

### Verification
- `cargo build --features web` — compiles for WASM with Lua VM embedded
- `cargo build --features ios` — compiles for iOS (aarch64-apple-ios)
- `cargo build --features android` — compiles for Android
- `cd platform/web && make dev` — shows colored background drawn by Lua
- `cargo test -p unison-tests --test scripting_foundation` — all tests pass
- Lua `init`/`update`/`render` functions called each frame (verified via `print()`)

---

## Phase 2: Core API Bindings

**Goal:** Expose World, ObjectSystem, input, cameras, and textures to Lua. Port a minimal playable game (spawn donut, move with input, physics).

### API style: method syntax (`world:method()`)

### Work

1. **World bindings** (`bindings/world.rs`)
   - `World.new()` → creates world, returns userdata wrapping `Rc<RefCell<World>>`
   - `world:set_background()`, `world:set_gravity()`, `world:set_ground()`, `world:set_ground_restitution()`
   - `world:step(dt)`, `world:auto_render()`

2. **Object bindings** (`bindings/objects.rs`)
   - `world:spawn_soft_body(desc_table)` — mesh presets resolved in Rust (`"ring"` → `create_ring_mesh(...)`)
   - `world:spawn_rigid_body(desc_table)`, `world:spawn_static_rect()`, `world:spawn_sprite()`, `world:despawn()`
   - Physics: `world:apply_force()`, `world:apply_impulse()`, `world:apply_torque()`
   - Queries: `world:get_position()`, `world:get_velocity()`, `world:is_grounded()`, `world:is_touching()`
   - Display: `world:set_z_order()`, `world:set_casts_shadow()`, `world:set_position()`

3. **Input bindings** (`bindings/input.rs`)
   - Global `input` table refreshed each frame
   - `input:is_key_pressed("Space")`, `input:is_key_just_pressed("W")`
   - `input:axis()` → returns x, y; `input:touches_just_began()` → array of touch tables
   - KeyCode strings mapped to `unison_input::KeyCode` variants in Rust

4. **Camera bindings** (`bindings/camera.rs`)
   - `world:camera_follow("main", id, damping)`, `world:camera_follow_with_offset(...)`
   - `world:camera_add("name", w, h)`, `world:camera_get_position("name")`

5. **Texture loading** (`bindings/engine.rs`)
   - `engine.load_texture("textures/donut-pink.png")` → integer handle
   - `engine:screen_size()` → width, height
   - `engine:set_anti_aliasing("msaa8x")`

6. **Ownership model** — `Rc<RefCell<World>>` shared between Lua userdata and `ScriptedGame`. Renderer access via thread-local during render phase.

### Key files
- Create: `unison-scripting/src/bindings/mod.rs`
- Create: `unison-scripting/src/bindings/world.rs`
- Create: `unison-scripting/src/bindings/objects.rs`
- Create: `unison-scripting/src/bindings/input.rs`
- Create: `unison-scripting/src/bindings/camera.rs`
- Create: `unison-scripting/src/bindings/engine.rs`
- Modify: `unison-scripting/src/lib.rs` (wire bindings)
- Create: `project/assets/scripts/main.lua` (minimal donut platformer)

### E2E Tests (`unison-tests`)

Add `scripting_core_api.rs` covering:
- **World bindings**: create World from Lua, set gravity/ground, verify physics step advances positions
- **Object spawning**: spawn soft body, rigid body, static rect, sprite from Lua descriptor tables; verify ObjectIds returned and objects exist in world
- **Physics interaction**: apply force/impulse/torque from Lua, verify velocity/position changes after step
- **Queries**: `get_position`, `get_velocity`, `is_grounded`, `is_touching` return correct values after simulation
- **Despawn**: despawn object from Lua, verify it's removed from world
- **Input bindings**: inject key presses into `InputState`, verify Lua `input:is_key_pressed()` / `input:is_key_just_pressed()` return correct booleans
- **Camera**: set camera follow from Lua, step world, verify camera position tracks target with damping
- **Texture loading**: `engine.load_texture()` returns a valid handle (using mock renderer)
- **Ownership safety**: multiple Lua references to same World don't cause panics; World outlives individual object references

### Doc Updates

- Update `unison2d/docs/api/scripting.md` — add World, Object, Input, Camera binding API reference
- Update `unison2d/docs/API.md` — add scripting quick-reference section

### Verification
- Lua script spawns soft-body donut, ground, moves with keyboard/joystick, jumps
- Camera follows donut with damping
- Touch input works on iOS/Android
- `cargo test -p unison-tests --test scripting_core_api` — all tests pass
- Runs at 60fps (Lua overhead negligible)

### Example Lua
```lua
local game = {}
local world, donut, donut_tex

function game.init()
    donut_tex = engine.load_texture("textures/donut-pink.png")
    world = World.new()
    world:set_background(0x1a1a2e)
    world:set_gravity(-9.8)
    world:set_ground(-4.5)

    donut = world:spawn_soft_body({
        mesh = "ring", mesh_params = {1.0, 0.25, 24, 8},
        material = {density = 900, edge_compliance = 5e-6, area_compliance = 2e-5},
        position = {0, 3.5}, color = 0xFFFFFF, texture = donut_tex,
    })
    world:spawn_static_rect({0, -7}, {30, 5}, 0x2d5016)
    world:camera_follow_with_offset("main", donut, 0.08, 0, 3.5)
end

function game.update(dt)
    local move_x = input:axis_x()
    if math.abs(move_x) < 0.01 then
        if input:is_key_pressed("ArrowLeft") then move_x = -1
        elseif input:is_key_pressed("ArrowRight") then move_x = 1
        else move_x = 0 end
    end
    if move_x ~= 0 then
        world:apply_force(donut, move_x * 80, 0)
        if world:is_grounded(donut) then world:apply_torque(donut, -move_x * 20, dt) end
    end
    if input:is_key_just_pressed("Space") and world:is_grounded(donut) then
        world:apply_impulse(donut, 0, 10)
    end
    world:step(dt)
end

function game.render()
    world:auto_render()
end

return game
```

---

## Phase 3: Full API Bindings

**Goal:** Expose lighting, events/collisions, UI, render layers/targets, and scene management. Port the full 4-level donut game to Lua.

### Work

1. **Lighting bindings** (`bindings/lighting.rs`)
   - `world:lighting_set_enabled()`, `world:lighting_set_ambient()`, `world:lighting_set_ground_shadow()`
   - `world:add_point_light(desc)`, `world:add_directional_light(desc)` → handle
   - `world:set_light_intensity()`, `world:set_directional_light_direction()`, etc.
   - `world:light_follow()`, `world:light_follow_with_offset()`, `world:light_unfollow()`

2. **Event system** (`bindings/events.rs`) — simplified string-keyed events
   - `events:emit("name", data_table)` / `events:on("name", callback)`
   - `events:on_collision(fn)`, `events:on_collision_for(id, fn)`, `events:on_collision_between(a, b, fn)`
   - Implemented as `HashMap<String, Vec<RegistryKey>>` in Rust; collision events auto-translated

3. **Scene management** (`bindings/scene.rs`) — replaces Level trait
   - `engine:set_scene(scene_table)`, `engine:switch_scene(scene_table)`
   - Scene table: `{ on_enter, update, render, on_exit }` functions
   - Rust bridge auto-calls `world:step(dt)` after `scene.update(dt)` and `world:auto_render()` after `scene.render()` (unless scene called them manually)

4. **Render layers** (`bindings/render_layers.rs`)
   - `world:create_render_layer("sky", {lit=false, clear_color=0x020206})`
   - `world:create_render_layer_before(...)`, `world:set_layer_clear_color()`
   - `world:draw_to(layer, "circle", params, z)`, `world:draw_to(layer, "gradient_circle", params, z)`

5. **Render targets & compositing** (`bindings/render_targets.rs`)
   - `engine:create_render_target(w, h)` → target, texture
   - `world:render_to_targets({{"main", SCREEN}, {"overview", target}})`
   - `engine:draw_overlay()`, `engine:draw_overlay_bordered()`

6. **UI bindings** (`bindings/ui.rs`)
   - `engine:create_ui("fonts/DejaVuSans-Bold.ttf")` → UI handle
   - `ui:frame(tree_table, world)` — tree built from nested Lua tables
   - Button callbacks as Lua functions (routed through event system internally)

7. **Math utilities** (`bindings/math.rs`)
   - `Color.hex()`, `Color.rgba()`, `Color:lerp()`
   - `Rng.new(seed)`, `rng:range(min, max)`, `rng:range_int(min, max)`
   - `math.lerp()`, `math.smoothstep()`, `math.clamp()`

### Key files
- Create: `unison-scripting/src/bindings/lighting.rs`
- Create: `unison-scripting/src/bindings/events.rs`
- Create: `unison-scripting/src/bindings/scene.rs`
- Create: `unison-scripting/src/bindings/render_layers.rs`
- Create: `unison-scripting/src/bindings/render_targets.rs`
- Create: `unison-scripting/src/bindings/ui.rs`
- Create: `unison-scripting/src/bindings/math.rs`
- Create: `project/assets/scripts/scenes/menu.lua`
- Create: `project/assets/scripts/scenes/main_level.lua`
- Create: `project/assets/scripts/scenes/lighting.lua`
- Create: `project/assets/scripts/scenes/random_spawns.lua`
- Create: `project/assets/scripts/scenes/shared.lua` (utilities: `drive_donut`, `new_world`)
- Create: `project/assets/scripts/scenes/day_night_cycle.lua`

### E2E Tests (`unison-tests`)

Add `scripting_full_api.rs` covering:
- **Lighting**: create point/directional lights from Lua, set intensity/direction, verify LightingSystem state; light_follow updates position after step
- **Events (string-keyed)**: emit event from Lua, register Lua callback via `events:on()`, flush, verify callback was invoked with correct data table
- **Collision events**: spawn two overlapping bodies, step, verify `events:on_collision()` callback fires with correct object IDs
- **Collision filtering**: `on_collision_for(id)` only fires for the specified object; `on_collision_between(a, b)` only fires for that pair
- **Scene management**: `engine:set_scene()` calls `on_enter`; `engine:switch_scene()` calls `on_exit` on old, `on_enter` on new; verify scene's `update`/`render` are called each frame
- **Render layers**: create lit/unlit layers from Lua, draw commands via `world:draw_to()`, verify render commands appear in correct layer (using mock renderer)
- **Render targets**: create render target, call `world:render_to_targets()` with multiple cameras, verify mock renderer receives correct target bindings
- **UI**: build UI tree from Lua tables, simulate click via injected input, verify button callback fires and event reaches event system
- **Math utilities**: `Color.hex()`, `Color:lerp()`, `Rng` determinism (same seed → same sequence), `math.lerp`/`math.smoothstep`/`math.clamp` correctness

### Doc Updates

- Update `unison2d/docs/api/scripting.md` — add Lighting, Events, Scene, Render Layer/Target, UI, Math binding API reference
- Add `unison2d/docs/guide/scripting-scenes.md` — patterns for multi-scene games in Lua (replaces `levels.md` for scripted games)
- Document engine simplifications table in `unison2d/docs/api/scripting.md` — explain what each Rust abstraction was, why it existed, and what replaces it in Lua

### Verification
- All 4 levels ported to Lua are visually identical to the Rust version
- Menu UI buttons work, scene transitions work
- Day/night cycle with directional lights + shadows
- PiP camera in RandomSpawns level
- Collision detection (trigger box in Main level)
- `cargo test -p unison-tests --test scripting_full_api` — all tests pass

---

## Phase 4: TypeScript Support

**Goal:** TypeScript-to-Lua pipeline with `.d.ts` type definitions for the engine API.

### Work

1. **Type definitions** (`unison2d/types/unison2d.d.ts`)
   - Complete `.d.ts` describing all Lua globals: `engine`, `input`, `events`, `World`, `Color`, `Rng`, etc.
   - Uses TSTL's `LuaMultiReturn` for multi-return functions

2. **Build pipeline**
   - `project/package.json` — `typescript-to-lua`, `@typescript-to-lua/language-extensions`
   - `project/tsconfig.json` — TSTL config targeting Lua 5.4, `rootDir: scripts-ts/`, `outDir: assets/scripts/`
   - Makefile target: `make ts` runs `npx tstl`

3. **Port donut game to TypeScript** in `project/scripts-ts/`
   - Demonstrates the full workflow
   - Compiled output goes to `project/assets/scripts/` (same location Lua reads from)

### Key files
- Create: `unison2d/types/unison2d.d.ts`
- Create: `project/package.json`
- Create: `project/tsconfig.json`
- Create: `project/scripts-ts/main.ts`
- Create: `project/scripts-ts/scenes/*.ts`
- Modify: `platform/web/Makefile` (add `ts` target)

### E2E Tests (`unison-tests`)

Add `scripting_typescript.rs` covering:
- **TSTL output compatibility**: load a pre-compiled TSTL Lua output file into the Lua VM, verify it executes correctly (TSTL emits specific Lua patterns like `__TS__` helpers — ensure they work with our Lua 5.4 VM)
- **Multi-file require**: TSTL compiles multiple `.ts` files into multiple `.lua` files with `require()` — verify Lua's `require` resolution works within the embedded asset system
- **Type definition completeness**: a TypeScript file that exercises every engine API compiles without type errors (validated by `tsc --noEmit` in CI)

### Doc Updates

- Create `unison2d/docs/scripting/typescript.md` — TSTL setup, `tsconfig.json` config, build pipeline, gotchas (LuaMultiReturn, `@noSelf`, class method syntax)
- Update `unison2d/docs/guide/getting-started.md` (or create `getting-started-ts.md`) — TypeScript quickstart

### Verification
- `npx tstl` compiles TypeScript to Lua with zero errors
- `tsc --noEmit` type-checks with zero errors
- Compiled Lua runs identically to hand-written Lua
- `cargo test -p unison-tests --test scripting_typescript` — all tests pass
- IDE autocomplete and error highlighting work in TypeScript files

---

## Phase 5: Developer Experience & Cleanup

**Goal:** Hot reload, error reporting, debug tools, documentation. Remove deprecated Rust game-code path.

### Work

1. **Error handling** — Lua errors surface as overlay (debug) or log (release), not crashes
2. **Hot reload** (dev mode only)
   - Web: file watcher detects `.lua` changes, re-executes scripts
   - Native: poll filesystem path in debug builds
   - Level 1: full restart (re-run `init`)
   - Level 2: preserve world state, re-bind `update`/`render` functions
3. **Debug utilities** — `debug.log()`, `debug.draw_point()`, `debug.show_physics()`, `debug.show_fps()`
4. **Documentation** in `unison2d/docs/scripting/`
   - `getting-started.md` — first Lua game, first TypeScript game
   - `api-reference.md` — complete Lua API reference
   - `migration-guide.md` — porting Rust games to Lua
   - `typescript.md` — TSTL workflow and quirks
5. **Deprecation cleanup** — Remove old Rust game code (`DonutGame`, level modules), clean up engine APIs that only existed for Rust game code (Action generics on Engine, Ctx, Level if no longer needed internally)

### Key files
- Create: `unison-scripting/src/hot_reload.rs`
- Create: `unison-scripting/src/error_overlay.rs`
- Create: `unison-scripting/src/debug.rs`
- Create: `unison2d/docs/scripting/*.md`
- Remove: `project/levels/` (Rust game levels)
- Modify: `project/lib.rs` (remove DonutGame code)

### E2E Tests (`unison-tests`)

Add `scripting_dx.rs` covering:
- **Error recovery**: execute a script with a runtime error in `update()`, verify `ScriptedGame` doesn't panic and continues calling `render()` (error overlay mode)
- **Error messages**: Lua errors include file name, line number, and stack trace in the error string
- **Hot reload**: call `ScriptedGame::reload()` with new script source, verify new `update`/`render` functions take effect on next frame
- **Hot reload preserves world**: after reload, World objects and physics state are unchanged
- **Debug utilities**: `debug.log()` output is captured (via mock logger), `debug.draw_point()` produces a render command

### Doc Updates

- Create `unison2d/docs/scripting/getting-started.md` — first Lua game, first TypeScript game
- Create `unison2d/docs/scripting/api-reference.md` — complete Lua API reference (all globals, all methods)
- Create `unison2d/docs/scripting/migration-guide.md` — porting a Rust game to Lua, with before/after examples for each subsystem
- Create `unison2d/docs/scripting/hot-reload.md` — how hot reload works, limitations, debug mode setup
- Update `unison2d/docs/guide/README.md` — link to scripting docs
- Update `unison2d/CLAUDE.md` — add scripting docs to the docs table

### Verification
- Change `.lua` file while web dev server runs → game updates within 1s
- Introduce Lua syntax error → error overlay with file/line/stack trace, game doesn't crash
- Fix error + hot-reload → game resumes
- `debug.log()` output in browser console / Xcode / logcat
- `cargo test -p unison-tests --test scripting_dx` — all tests pass
- Old Rust game code fully removed, engine compiles clean

---

## Dependency Graph

```
Phase 1a (WASM Spike)
    │
    ▼
Phase 1b (Create Crate)
    │
    ▼
Phase 2 (Core API)
    │
    ├──────────────────┐
    ▼                  ▼
Phase 3 (Full API)   Phase 4 (TypeScript) — can start once Phase 2 API is stable
    │                  │
    └────────┬─────────┘
             ▼
      Phase 5 (DX + Cleanup)
```
