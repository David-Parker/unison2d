# Implementation Tasks: Lua/TypeScript Scripting Support for Unison 2D

> **Master Plan:** [scripting_system.md](scripting_system.md)
> **Generated:** 2026-03-30
> **Status:** In Progress

---

## Instructions for Implementation Agents

**IMPORTANT: Read this section before starting any work.**

1. **Read the master plan first.** Open [scripting_system.md](scripting_system.md) and read it end-to-end. It contains the full context, motivation, design decisions, and examples you need.
2. **Find your starting point.** Scan the task list below for the first unchecked `[ ]` item. That is where you pick up.
3. **Work phase by phase.** Complete all tasks within a phase before moving to the next. If a phase has dependencies noted, verify those phases are complete first.
4. **Check off tasks as you go.** After completing each task, edit this file to change `[ ]` to `[x]`. Do this immediately after each task — not in batches.
5. **If you cannot complete a task**, add a note directly below the task line with the prefix `> BLOCKED:` explaining what went wrong. Leave the checkbox unchecked. The next agent will see this and can address it.
6. **Commit frequently.** Commit after each logical unit of work (individual task or small group of related tasks). Use the project's commit format: `[PREFIX]: Description`.
7. **Verify before marking a phase done.** Each phase ends with a "Phase Verification" section. Run every check listed. Only mark the phase complete if all verifications pass.
8. **STOP after each phase.** Once a phase is complete and all verifications pass, **stop immediately**. Do not proceed to the next phase automatically. Report which phase just completed and wait for explicit instruction to continue.

---

## Progress Summary

| Phase | Description | Status |
|-------|-------------|--------|
| Phase 1a | WASM Spike — validate mlua + WASM compatibility | [x] Complete |
| Phase 1b | Create Crate — `unison-scripting` with ScriptedGame | [x] Complete |
| Phase 2 | Core API Bindings — World, Objects, Input, Camera, Textures | [ ] Not Started |
| Phase 3 | Full API Bindings — Lighting, Events, UI, Render, Scenes, Math | [ ] Not Started |
| Phase 4 | TypeScript Support — TSTL pipeline + type definitions | [ ] Not Started |
| Phase 5 | Developer Experience & Cleanup — Hot reload, errors, debug, docs | [ ] Not Started |

---

## Phase 1a: WASM Spike

**Goal:** Validate that `mlua` with vendored Lua 5.4 compiles and executes on `wasm32-unknown-unknown` before committing to the full implementation.
**Dependencies:** None
**Branch:** `feat/lua-scripting`

### Tasks

- [x] **1a.1 — Create branch `feat/lua-scripting`**
  Create and push the feature branch for all scripting work.

- [x] **1a.2 — Create minimal WASM test crate**
  Create a small standalone crate (outside the engine) to test mlua + WASM.
  - Files: create a temporary test crate directory (e.g., `spike/lua-wasm-test/`)
  - [x] Create `spike/lua-wasm-test/Cargo.toml` with dependency `mlua = { version = "0.10", features = ["lua54", "vendored"] }`
  - [x] Create `spike/lua-wasm-test/src/lib.rs` with code that creates a `Lua` instance and executes `print("hello")`

- [x] **1a.3 — Compile spike to WASM**
  - [x] Run `cargo build --target wasm32-unknown-unknown` in the spike crate
  - [x] If compilation fails, document the error and evaluate fallbacks: `cc` crate to compile Lua 5.4 C source directly, or `wasmoon`
  > **Resolution:** `lua-src` v547 rejects `wasm32-unknown-unknown` in its build script, and Apple Clang lacks the WebAssembly backend. Fixed by: (1) patching `lua-src` to add a wasm32 branch with a minimal C11 build, (2) bundling a minimal libc sysroot (`spike/lua-src-patched/wasm-sysroot/`) with just the headers Lua needs, and (3) using LLVM clang from Homebrew (`brew install llvm`) as `CC_wasm32_unknown_unknown`. See `spike/lua-wasm-test/.cargo/config.toml`. **For `unison-scripting`, the same patched lua-src + LLVM clang approach will be used.**

- [x] **1a.4 — Run Lua in browser**
  - [x] Add `wasm-bindgen` to the spike crate
  - [x] Create a minimal HTML page that loads the WASM module
  - [x] Execute a trivial Lua script (`print("hello")`) in the browser via wasm-bindgen
  - [x] Verify Lua VM initializes and executes successfully

### Phase 1a Verification

- [x] `cargo build --target wasm32-unknown-unknown` succeeds in spike crate
- [x] Lua VM initializes and executes a script in a WASM context in the browser (HTML page at `spike/lua-wasm-test/index.html` verified locally)
- [x] All tasks in Phase 1a are checked off

---

## Phase 1b: Create the Crate

**Goal:** Create the `unison-scripting` crate with `ScriptedGame` implementing the `Game` trait, minimal Lua bindings, and a hello-world Lua script running on all platforms.
**Dependencies:** Phase 1a (WASM compatibility validated)
**Branch:** `feat/lua-scripting`

### Tasks

- [x] **1b.1 — Create `unison-scripting` crate directory and Cargo.toml**
  - Files: `unison2d/crates/unison-scripting/Cargo.toml` (create)
  - [x] Create directory `unison2d/crates/unison-scripting/src/`
  - [x] Create `Cargo.toml` with dependencies: `unison2d` (workspace), `mlua` (version 0.10, features: `lua54`, `vendored`, `send`)
  - [x] Add `vendored` feature that compiles Lua 5.4 from C source (critical for WASM, iOS static linking, Android NDK)

- [x] **1b.2 — Implement `ScriptedGame` struct**
  - Files: `unison2d/crates/unison-scripting/src/lib.rs` (create)
  - [x] Define `ScriptedGame` struct that holds a `Lua` instance and loaded script state
  - [x] Define `type Action = NoAction` (unit-type action enum since scripted games don't use Rust action mapping)
  - [x] Implement `Game` trait for `ScriptedGame` with `init`, `update`, `render` methods
  - [x] In `init`: create Lua VM, load and execute the main script, call the returned table's `init()` function
  - [x] In `update`: call the script's `update(dt)` function each frame
  - [x] In `render`: call the script's `render()` function each frame

- [x] **1b.3 — Create minimal bridge bindings**
  - Files: `unison2d/crates/unison-scripting/src/bridge.rs` (create)
  - [x] Implement `engine.set_background(r, g, b)` — sets renderer clear color
  - [x] Implement `engine.draw_rect(x, y, w, h, r, g, b)` — draws a colored rectangle
  - [x] Implement `engine.screen_size()` — returns screen width, height
  - [x] Register these as Lua globals before script execution
  > **Note:** Render commands are buffered in a thread-local `Vec<RenderCommand>` and flushed after Lua's `render()` returns. This avoids storing raw `&mut Renderer` pointers across the Lua call boundary.

- [x] **1b.4 — Add workspace member**
  - Files: `unison2d/Cargo.toml` (modify)
  - [x] Add `"crates/unison-scripting"` to the `[workspace] members` list
  - [x] Add `unison-scripting = { path = "crates/unison-scripting" }` to `[workspace.dependencies]`
  - [x] Add `[patch.crates-io] lua-src = { path = "vendor/lua-src" }` for WASM support

- [x] **1b.5 — Add dependency in root Cargo.toml and wire entry points**
  - Files: `Cargo.toml` (modify), `project/lib.rs` (modify)
  - [x] Add `unison-scripting = { path = "unison2d/crates/unison-scripting" }` to root `[dependencies]`
  - [x] Modify `project/lib.rs` to instantiate `ScriptedGame` instead of `DonutGame`
  - [x] Scripts loaded from assets: `project/assets/scripts/main.lua`
  - [x] Update web entry point (`main()`) to use `ScriptedGame`
  - [x] Update `new_donut_game()` helper (replaced with `new_scripted_game()`) used by iOS/Android FFI

- [x] **1b.6 — Create hello-world Lua script**
  - Files: `project/assets/scripts/main.lua` (create)
  - [x] Create the script with `game.init()` setting background color via `engine.set_background(0.1, 0.1, 0.12)`
  - [x] `game.update(dt)` as empty function
  - [x] `game.render()` drawing a rect via `engine.draw_rect(0, 0, 2, 2, 1, 0.2, 0.2)`
  - [x] Script returns the `game` table

### Tests

- [x] **1b.T1 — Create `scripting_foundation.rs` test file**
  - File: `unison2d/crates/unison-tests/tests/scripting_foundation.rs`
  - [x] Add `unison-scripting` to `unison-tests/Cargo.toml` dev-dependencies
  - [x] Test: Lua VM initializes successfully — create `Lua` instance, execute trivial script, assert no error
  - [x] Test: `ScriptedGame` implements `Game` trait — can be constructed and `init`/`update`/`render` called in sequence without panic
  - [x] Test: Script lifecycle — returned table has `init`/`update`/`render` functions, verify all are called (e.g., via side effects like a global counter)
  - [x] Test: Script error handling — malformed script (syntax error) produces a recoverable error, not a panic
  - [x] Test: Script missing lifecycle function — missing `render` is a no-op (graceful fallback, no panic)

### Documentation

- [x] **1b.D1 — Create `scripting.md` API doc**
  - File: `unison2d/docs/api/scripting.md` (create)
  - [x] Document the purpose of `unison-scripting` crate
  - [x] Document `ScriptedGame` struct and its `Game` trait implementation
  - [x] Document the Lua lifecycle: script returns a table with `init`/`update`/`render` functions

- [x] **1b.D2 — Update `CLAUDE.md` crate table**
  - File: `unison2d/CLAUDE.md` (modify)
  - [x] Add `unison-scripting` row to the crate table with link to `docs/api/scripting.md`

### Phase 1b Verification

- [x] `cargo build --features web` — compiles for WASM with Lua VM embedded
- [x] `cargo build --features ios` — compiles with iOS feature flag (host build; cross-compile to aarch64-apple-ios requires Xcode SDK)
- [x] `cargo build --features android` — compiles with Android feature flag (host build; cross-compile to aarch64-linux-android requires NDK)
- [x] `cd platform/web && make dev` — shows green background and red rect drawn by Lua in the browser
  > **Note:** Required fixing three runtime issues beyond the initial compilation: (1) UTF-8 panic from loading gzip-compressed Lua script — fixed by adding `ScriptedGame::from_asset()` that loads via `AssetStore` decompression during `init()`. (2) `longjmp` panic — `setjmp`/`longjmp` cannot work in `wasm32-unknown-unknown` (no stack save/restore). Three approaches failed (Rust `catch_unwind`, WASM `+exception-handling`, clang `-mllvm -wasm-enable-sjlj`) due to wasm-bindgen 0.2 incompatibility. Fixed by patching `ldo.c` to call `wasm_lua_throw` (`wasm_bindgen::throw_str`) and `wasm_protected_call` (`js_sys::Function::call3` try/catch) — a JS exception bridge that routes Lua error recovery through the JS→WASM boundary. (3) Stale C build cache — `cc` crate cached old `ldo.c` objects; required full `cargo clean --target wasm32-unknown-unknown`.
- [x] `cargo test -p unison-tests --test scripting_foundation` — all 6 tests pass
- [x] Lua `init`/`update`/`render` functions called each frame (verified via `script_lifecycle_all_called` test)
- [x] All tasks in Phase 1b are checked off
- [x] Code compiles without errors

---

## Phase 2: Core API Bindings

**Goal:** Expose World, ObjectSystem, input, cameras, and textures to Lua. Port a minimal playable game (spawn donut, move with input, physics).
**Dependencies:** Phase 1b
**Branch:** `feat/lua-scripting`

### Tasks

- [ ] **2.1 — Create bindings module structure**
  - Files: `unison-scripting/src/bindings/mod.rs` (create)
  - [ ] Create `unison-scripting/src/bindings/` directory
  - [ ] Create `mod.rs` that re-exports all binding submodules
  - [ ] Wire bindings module into `unison-scripting/src/lib.rs`

- [ ] **2.2 — World bindings**
  - Files: `unison-scripting/src/bindings/world.rs` (create)
  - [ ] Implement `World.new()` — creates a World, returns userdata wrapping `Rc<RefCell<World>>`
  - [ ] Implement `world:set_background(color)` — sets world background color
  - [ ] Implement `world:set_gravity(g)` — sets world gravity
  - [ ] Implement `world:set_ground(y)` — sets ground plane Y position
  - [ ] Implement `world:set_ground_restitution(r)` — sets ground bounce factor
  - [ ] Implement `world:step(dt)` — advances physics simulation
  - [ ] Implement `world:auto_render()` — renders all objects through the camera system
  - [ ] Register `World` as a Lua userdata type with all methods

- [ ] **2.3 — Object bindings**
  - Files: `unison-scripting/src/bindings/objects.rs` (create)
  - [ ] Implement `world:spawn_soft_body(desc_table)` — mesh presets resolved in Rust (`"ring"` → `create_ring_mesh(...)`)
  - [ ] Implement `world:spawn_rigid_body(desc_table)` — spawns a rigid body from descriptor table
  - [ ] Implement `world:spawn_static_rect(pos, size, color)` — spawns a static rectangular body
  - [ ] Implement `world:spawn_sprite(desc_table)` — spawns a sprite object
  - [ ] Implement `world:despawn(id)` — removes an object from the world
  - [ ] Implement physics methods: `world:apply_force(id, fx, fy)`, `world:apply_impulse(id, ix, iy)`, `world:apply_torque(id, torque, dt)`
  - [ ] Implement query methods: `world:get_position(id)`, `world:get_velocity(id)`, `world:is_grounded(id)`, `world:is_touching(id)`
  - [ ] Implement display methods: `world:set_z_order(id, z)`, `world:set_casts_shadow(id, bool)`, `world:set_position(id, x, y)`

- [ ] **2.4 — Input bindings**
  - Files: `unison-scripting/src/bindings/input.rs` (create)
  - [ ] Create global `input` table refreshed each frame from `InputState`
  - [ ] Implement `input:is_key_pressed("Space")` — returns true if key is currently held
  - [ ] Implement `input:is_key_just_pressed("W")` — returns true if key was pressed this frame
  - [ ] Implement `input:axis_x()`, `input:axis_y()` — returns analog axis values (-1..1)
  - [ ] Implement `input:touches_just_began()` — returns array of touch tables with x, y positions
  - [ ] Map KeyCode strings to `unison_input::KeyCode` variants in Rust

- [ ] **2.5 — Camera bindings**
  - Files: `unison-scripting/src/bindings/camera.rs` (create)
  - [ ] Implement `world:camera_follow("name", id, damping)` — makes a named camera follow an object
  - [ ] Implement `world:camera_follow_with_offset("name", id, damping, offset_x, offset_y)` — follow with offset
  - [ ] Implement `world:camera_add("name", w, h)` — adds a new named camera
  - [ ] Implement `world:camera_get_position("name")` — returns camera x, y

- [ ] **2.6 — Engine/Texture bindings**
  - Files: `unison-scripting/src/bindings/engine.rs` (create)
  - [ ] Implement `engine.load_texture("textures/donut-pink.png")` — returns integer TextureId handle
  - [ ] Implement `engine:screen_size()` — returns width, height
  - [ ] Implement `engine:set_anti_aliasing("msaa8x")` — sets AA mode from string
  - [ ] Update the minimal bridge from Phase 1b to use this new engine bindings module

- [ ] **2.7 — Ownership model**
  - Files: `unison-scripting/src/lib.rs` (modify)
  - [ ] Implement `Rc<RefCell<World>>` shared between Lua userdata and `ScriptedGame`
  - [ ] Implement renderer access via thread-local during render phase
  - [ ] Ensure Lua can hold World references safely across frames

- [ ] **2.8 — Port minimal donut platformer to Lua**
  - Files: `project/assets/scripts/main.lua` (modify — replace hello-world with playable game)
  - [ ] Load donut texture, create world with gravity and ground
  - [ ] Spawn soft-body donut with ring mesh, ground platform
  - [ ] Handle keyboard input (arrow keys for movement, Space for jump) and joystick axis
  - [ ] Apply forces for movement, impulse for jump, torque for rolling
  - [ ] Camera follow with offset
  - [ ] World step and auto-render
  - Use the example Lua from the master plan (see Phase 2 "Example Lua" section)

### Tests

- [ ] **2.T1 — Create `scripting_core_api.rs` test file**
  - File: `unison2d/crates/unison-tests/tests/scripting_core_api.rs`
  - [ ] Test: World bindings — create World from Lua, set gravity/ground, verify physics step advances positions
  - [ ] Test: Object spawning — spawn soft body, rigid body, static rect, sprite from Lua descriptor tables; verify ObjectIds returned and objects exist in world
  - [ ] Test: Physics interaction — apply force/impulse/torque from Lua, verify velocity/position changes after step
  - [ ] Test: Queries — `get_position`, `get_velocity`, `is_grounded`, `is_touching` return correct values after simulation
  - [ ] Test: Despawn — despawn object from Lua, verify it's removed from world
  - [ ] Test: Input bindings — inject key presses into `InputState`, verify Lua `input:is_key_pressed()` / `input:is_key_just_pressed()` return correct booleans
  - [ ] Test: Camera — set camera follow from Lua, step world, verify camera position tracks target with damping
  - [ ] Test: Texture loading — `engine.load_texture()` returns a valid handle (using mock renderer)
  - [ ] Test: Ownership safety — multiple Lua references to same World don't cause panics; World outlives individual object references

### Documentation

- [ ] **2.D1 — Update `scripting.md` with Core API reference**
  - File: `unison2d/docs/api/scripting.md` (update)
  - [ ] Add World binding API reference (methods, parameters, return types)
  - [ ] Add Object binding API reference (spawn, physics, queries, display)
  - [ ] Add Input binding API reference (key queries, axis, touches)
  - [ ] Add Camera binding API reference (follow, add, get_position)

- [ ] **2.D2 — Update `API.md` with scripting quick-reference**
  - File: `unison2d/docs/API.md` (update)
  - [ ] Add a scripting quick-reference section linking to the full `scripting.md` doc

### Phase 2 Verification

- [ ] Lua script spawns soft-body donut, ground, moves with keyboard/joystick, jumps
- [ ] Camera follows donut with damping
- [ ] Touch input works on iOS/Android
- [ ] `cargo test -p unison-tests --test scripting_core_api` — all tests pass
- [ ] Runs at 60fps (Lua overhead negligible)
- [ ] All tasks in Phase 2 are checked off
- [ ] All tests pass
- [ ] Code compiles without errors

---

## Phase 3: Full API Bindings

**Goal:** Expose lighting, events/collisions, UI, render layers/targets, and scene management. Port the full 4-level donut game to Lua.
**Dependencies:** Phase 2
**Branch:** `feat/lua-scripting`

### Tasks

- [ ] **3.1 — Lighting bindings**
  - Files: `unison-scripting/src/bindings/lighting.rs` (create)
  - [ ] Implement `world:lighting_set_enabled(bool)` — enable/disable lighting system
  - [ ] Implement `world:lighting_set_ambient(r, g, b, a)` — set ambient light color
  - [ ] Implement `world:lighting_set_ground_shadow(params)` — configure ground shadow
  - [ ] Implement `world:add_point_light(desc)` — create point light from descriptor table, return handle
  - [ ] Implement `world:add_directional_light(desc)` — create directional light, return handle
  - [ ] Implement `world:set_light_intensity(handle, intensity)` — update light intensity
  - [ ] Implement `world:set_directional_light_direction(handle, dx, dy)` — update direction
  - [ ] Implement `world:light_follow(handle, object_id)` — make light track an object
  - [ ] Implement `world:light_follow_with_offset(handle, object_id, ox, oy)` — track with offset
  - [ ] Implement `world:light_unfollow(handle)` — stop tracking

- [ ] **3.2 — Event system bindings**
  - Files: `unison-scripting/src/bindings/events.rs` (create)
  - [ ] Implement string-keyed event system as `HashMap<String, Vec<RegistryKey>>` in Rust
  - [ ] Implement `events:emit("name", data_table)` — emit an event with optional data
  - [ ] Implement `events:on("name", callback)` — register a Lua callback for an event
  - [ ] Implement `events:on_collision(fn)` — register a callback for any collision
  - [ ] Implement `events:on_collision_for(id, fn)` — collision callback for specific object
  - [ ] Implement `events:on_collision_between(a, b, fn)` — collision callback for specific pair
  - [ ] Auto-translate engine collision events into Lua event callbacks
  - [ ] Implement event dispatch/flush per frame (process pending events, call callbacks)

- [ ] **3.3 — Scene management bindings**
  - Files: `unison-scripting/src/bindings/scene.rs` (create)
  - [ ] Implement `engine:set_scene(scene_table)` — set initial scene (calls `on_enter`)
  - [ ] Implement `engine:switch_scene(scene_table)` — transition: call `on_exit` on old, `on_enter` on new
  - [ ] Scene table format: `{ on_enter, update, render, on_exit }` functions
  - [ ] Rust bridge auto-calls `world:step(dt)` after `scene.update(dt)` and `world:auto_render()` after `scene.render()` unless scene called them manually

- [ ] **3.4 — Render layers bindings**
  - Files: `unison-scripting/src/bindings/render_layers.rs` (create)
  - [ ] Implement `world:create_render_layer("name", {lit=false, clear_color=0x020206})` — create a named layer
  - [ ] Implement `world:create_render_layer_before("name", "before_name", desc)` — insert layer at position
  - [ ] Implement `world:set_layer_clear_color("name", color)` — update layer clear color
  - [ ] Implement `world:draw_to(layer, "circle", params, z)` — draw shape to specific layer
  - [ ] Implement `world:draw_to(layer, "gradient_circle", params, z)` — draw gradient circle to layer

- [ ] **3.5 — Render targets & compositing bindings**
  - Files: `unison-scripting/src/bindings/render_targets.rs` (create)
  - [ ] Implement `engine:create_render_target(w, h)` — create off-screen render target, return (target, texture) pair
  - [ ] Implement `world:render_to_targets({{"main", SCREEN}, {"overview", target}})` — render cameras to targets
  - [ ] Implement `engine:draw_overlay(texture, x, y, w, h)` — draw overlay from render target texture
  - [ ] Implement `engine:draw_overlay_bordered(texture, x, y, w, h, border)` — overlay with border

- [ ] **3.6 — UI bindings**
  - Files: `unison-scripting/src/bindings/ui.rs` (create)
  - [ ] Implement `engine:create_ui("fonts/DejaVuSans-Bold.ttf")` — create UI handle from font asset
  - [ ] Implement `ui:frame(tree_table, world)` — build and render UI tree from nested Lua tables
  - [ ] Support button elements with Lua function callbacks
  - [ ] Route button callbacks through the event system internally

- [ ] **3.7 — Math utilities bindings**
  - Files: `unison-scripting/src/bindings/math.rs` (create)
  - [ ] Implement `Color.hex(hex_int)` — create color from hex integer
  - [ ] Implement `Color.rgba(r, g, b, a)` — create color from RGBA components
  - [ ] Implement `Color:lerp(other, t)` — interpolate between colors
  - [ ] Implement `Rng.new(seed)` — create deterministic RNG with seed
  - [ ] Implement `rng:range(min, max)` — random float in range
  - [ ] Implement `rng:range_int(min, max)` — random integer in range
  - [ ] Implement `math.lerp(a, b, t)` — linear interpolation
  - [ ] Implement `math.smoothstep(a, b, t)` — smooth interpolation
  - [ ] Implement `math.clamp(x, min, max)` — clamp value

- [ ] **3.8 — Update bindings module**
  - Files: `unison-scripting/src/bindings/mod.rs` (modify)
  - [ ] Add all new binding modules (lighting, events, scene, render_layers, render_targets, ui, math)
  - [ ] Wire registration functions into `ScriptedGame` initialization

- [ ] **3.9 — Port menu level to Lua**
  - Files: `project/assets/scripts/scenes/menu.lua` (create)
  - [ ] Create UI-based menu with level selection buttons
  - [ ] Emit scene-switch events on button press
  - [ ] Match visual appearance of Rust `MenuLevel`

- [ ] **3.10 — Port main level to Lua**
  - Files: `project/assets/scripts/scenes/main_level.lua` (create)
  - [ ] Port full main level logic: donut spawning, platforms, trigger box, collision detection
  - [ ] Match visual appearance and gameplay of Rust `MainLevel`

- [ ] **3.11 — Port lighting level to Lua**
  - Files: `project/assets/scripts/scenes/lighting.lua` (create), `project/assets/scripts/scenes/day_night_cycle.lua` (create)
  - [ ] Port lighting level with point lights, directional lights, shadows
  - [ ] Implement day/night cycle with directional light rotation
  - [ ] Match visual appearance of Rust `LightingLevel`

- [ ] **3.12 — Port random spawns level to Lua**
  - Files: `project/assets/scripts/scenes/random_spawns.lua` (create)
  - [ ] Port random spawns level with PiP camera (render target + overlay)
  - [ ] Random object spawning logic
  - [ ] Match visual appearance of Rust `RandomSpawnsLevel`

- [ ] **3.13 — Create shared utilities**
  - Files: `project/assets/scripts/scenes/shared.lua` (create)
  - [ ] Implement `drive_donut(world, donut, input, dt)` — shared donut movement logic
  - [ ] Implement `new_world(opts)` — shared world creation with common settings

- [ ] **3.14 — Update main.lua to use scene management**
  - Files: `project/assets/scripts/main.lua` (modify)
  - [ ] Replace single-scene hello world with scene-managed multi-level game
  - [ ] Set initial scene to menu
  - [ ] Handle level-complete events to return to menu

### Tests

- [ ] **3.T1 — Create `scripting_full_api.rs` test file**
  - File: `unison2d/crates/unison-tests/tests/scripting_full_api.rs`
  - [ ] Test: Lighting — create point/directional lights from Lua, set intensity/direction, verify LightingSystem state; light_follow updates position after step
  - [ ] Test: Events (string-keyed) — emit event from Lua, register callback via `events:on()`, flush, verify callback was invoked with correct data table
  - [ ] Test: Collision events — spawn two overlapping bodies, step, verify `events:on_collision()` callback fires with correct object IDs
  - [ ] Test: Collision filtering — `on_collision_for(id)` only fires for specified object; `on_collision_between(a, b)` only fires for that pair
  - [ ] Test: Scene management — `engine:set_scene()` calls `on_enter`; `engine:switch_scene()` calls `on_exit` on old, `on_enter` on new; verify `update`/`render` called each frame
  - [ ] Test: Render layers — create lit/unlit layers from Lua, draw commands via `world:draw_to()`, verify render commands appear in correct layer (mock renderer)
  - [ ] Test: Render targets — create render target, call `world:render_to_targets()` with multiple cameras, verify mock renderer receives correct target bindings
  - [ ] Test: UI — build UI tree from Lua tables, simulate click via injected input, verify button callback fires and event reaches event system
  - [ ] Test: Math utilities — `Color.hex()`, `Color:lerp()`, `Rng` determinism (same seed → same sequence), `math.lerp`/`math.smoothstep`/`math.clamp` correctness

### Documentation

- [ ] **3.D1 — Update `scripting.md` with Full API reference**
  - File: `unison2d/docs/api/scripting.md` (update)
  - [ ] Add Lighting binding API reference
  - [ ] Add Events binding API reference (string-keyed events + collision events)
  - [ ] Add Scene management API reference
  - [ ] Add Render Layer/Target API reference
  - [ ] Add UI binding API reference
  - [ ] Add Math utilities API reference
  - [ ] Document engine simplifications table — what each Rust abstraction was, why it existed, and what replaces it in Lua

- [ ] **3.D2 — Create `scripting-scenes.md` guide**
  - File: `unison2d/docs/guide/scripting-scenes.md` (create)
  - [ ] Document patterns for multi-scene games in Lua
  - [ ] Scene table format and lifecycle (`on_enter`, `update`, `render`, `on_exit`)
  - [ ] Scene switching patterns
  - [ ] This replaces `levels.md` for scripted games

- [ ] **3.D3 — Document engine simplifications in `scripting.md`**
  - File: `unison2d/docs/api/scripting.md` (update)
  - [ ] Document the Rust-to-Lua simplification mapping: EventBus → string-keyed events, Level → scene tables, Ctx → individual globals, Engine\<A\> → NoAction, Prefab → factory functions, SharedState → global state

### Phase 3 Verification

- [ ] All 4 levels ported to Lua are visually identical to the Rust version
- [ ] Menu UI buttons work, scene transitions work
- [ ] Day/night cycle with directional lights + shadows
- [ ] PiP camera in RandomSpawns level
- [ ] Collision detection (trigger box in Main level)
- [ ] `cargo test -p unison-tests --test scripting_full_api` — all tests pass
- [ ] All tasks in Phase 3 are checked off
- [ ] All tests pass
- [ ] Code compiles without errors

---

## Phase 4: TypeScript Support

**Goal:** TypeScript-to-Lua pipeline with `.d.ts` type definitions for the engine API.
**Dependencies:** Phase 2 (can start once Phase 2 API is stable; independent of Phase 3)
**Branch:** `feat/lua-scripting`

### Tasks

- [ ] **4.1 — Create TypeScript type definitions**
  - Files: `unison2d/types/unison2d.d.ts` (create)
  - [ ] Define types for `engine` global (load_texture, screen_size, set_anti_aliasing, create_ui, create_render_target, draw_overlay, draw_overlay_bordered, set_scene, switch_scene)
  - [ ] Define types for `input` global (is_key_pressed, is_key_just_pressed, axis_x, axis_y, touches_just_began)
  - [ ] Define types for `events` global (emit, on, on_collision, on_collision_for, on_collision_between)
  - [ ] Define `World` class type with all methods (set_background, set_gravity, set_ground, spawn_soft_body, spawn_rigid_body, spawn_static_rect, spawn_sprite, despawn, apply_force, apply_impulse, apply_torque, get_position, get_velocity, is_grounded, is_touching, set_z_order, set_casts_shadow, set_position, camera_follow, camera_follow_with_offset, camera_add, camera_get_position, step, auto_render, lighting_*, render layer methods)
  - [ ] Define `Color` class type (hex, rgba, lerp)
  - [ ] Define `Rng` class type (new, range, range_int)
  - [ ] Define `math` extensions (lerp, smoothstep, clamp)
  - [ ] Use TSTL's `LuaMultiReturn` for multi-return functions (e.g., `screen_size`, `get_position`)
  - [ ] Add `@noSelf` annotations where needed for TSTL compatibility

- [ ] **4.2 — Set up TSTL build pipeline**
  - Files: `project/package.json` (create), `project/tsconfig.json` (create)
  - [ ] Create `project/package.json` with dependencies: `typescript-to-lua`, `@typescript-to-lua/language-extensions`
  - [ ] Create `project/tsconfig.json` with TSTL config: target Lua 5.4, `rootDir: "scripts-ts/"`, `outDir: "assets/scripts/"`
  - [ ] Run `npm install` to set up node_modules
  - [ ] Verify `npx tstl` compiles successfully (empty project)

- [ ] **4.3 — Add Makefile target for TypeScript compilation**
  - Files: `platform/web/Makefile` (modify)
  - [ ] Add `ts` target that runs `cd ../../project && npx tstl`
  - [ ] Consider adding `ts` as a prerequisite to `dev` and `build` targets

- [ ] **4.4 — Port donut game to TypeScript**
  - Files: `project/scripts-ts/main.ts` (create), `project/scripts-ts/scenes/*.ts` (create)
  - [ ] Create `project/scripts-ts/` directory structure
  - [ ] Port `main.lua` to `main.ts` using the type definitions
  - [ ] Port scene files (menu, main_level, lighting, random_spawns, shared) to TypeScript
  - [ ] Compile with `npx tstl` — output goes to `project/assets/scripts/`
  - [ ] Verify compiled Lua runs identically to hand-written Lua

### Tests

- [ ] **4.T1 — Create `scripting_typescript.rs` test file**
  - File: `unison2d/crates/unison-tests/tests/scripting_typescript.rs`
  - [ ] Test: TSTL output compatibility — load a pre-compiled TSTL Lua output file into the Lua VM, verify it executes correctly (TSTL emits `__TS__` helpers — ensure they work with Lua 5.4 VM)
  - [ ] Test: Multi-file require — TSTL compiles multiple `.ts` files into multiple `.lua` files with `require()` — verify Lua's `require` resolution works within the embedded asset system
  - [ ] Test: Type definition completeness — a TypeScript file that exercises every engine API compiles without type errors (validate with `tsc --noEmit` in CI)

### Documentation

- [ ] **4.D1 — Create `typescript.md` scripting doc**
  - File: `unison2d/docs/scripting/typescript.md` (create)
  - [ ] Document TSTL setup and `tsconfig.json` configuration
  - [ ] Document build pipeline (`npx tstl`)
  - [ ] Document gotchas: `LuaMultiReturn`, `@noSelf`, class method syntax differences
  - [ ] Include example TypeScript game code

- [ ] **4.D2 — Create/update getting-started guide for TypeScript**
  - File: `unison2d/docs/guide/getting-started-ts.md` (create) or update `getting-started.md`
  - [ ] TypeScript quickstart: install deps, create `.ts` file, compile, run
  - [ ] Link to type definitions and full API reference

### Phase 4 Verification

- [ ] `npx tstl` compiles TypeScript to Lua with zero errors
- [ ] `tsc --noEmit` type-checks with zero errors
- [ ] Compiled Lua runs identically to hand-written Lua
- [ ] `cargo test -p unison-tests --test scripting_typescript` — all tests pass
- [ ] IDE autocomplete and error highlighting work in TypeScript files
- [ ] All tasks in Phase 4 are checked off
- [ ] All tests pass
- [ ] Code compiles without errors

---

## Phase 5: Developer Experience & Cleanup

**Goal:** Hot reload, error reporting, debug tools, documentation. Remove deprecated Rust game-code path.
**Dependencies:** Phase 3 and Phase 4
**Branch:** `feat/lua-scripting`

### Tasks

- [ ] **5.1 — Error handling system**
  - Files: `unison-scripting/src/error_overlay.rs` (create), `unison-scripting/src/lib.rs` (modify)
  - [ ] Implement error overlay for debug builds — Lua errors display as on-screen overlay with file, line, and stack trace
  - [ ] Implement error logging for release builds — Lua errors logged but don't crash
  - [ ] Ensure `ScriptedGame` continues calling `render()` after an `update()` error (doesn't crash/freeze)
  - [ ] Error messages include Lua file name, line number, and stack trace

- [ ] **5.2 — Hot reload system**
  - Files: `unison-scripting/src/hot_reload.rs` (create), `unison-scripting/src/lib.rs` (modify)
  - [ ] Implement Level 1 hot reload: full restart (re-run `init`) on script change
  - [ ] Implement Level 2 hot reload: preserve world state, re-bind `update`/`render` functions only
  - [ ] Web: file watcher detects `.lua` changes, triggers script re-execution
  - [ ] Native: poll filesystem path in debug builds for changes
  - [ ] Only enabled in dev/debug mode — not included in release builds

- [ ] **5.3 — Debug utilities**
  - Files: `unison-scripting/src/debug.rs` (create)
  - [ ] Implement `debug.log(...)` — log output to platform console (browser console / Xcode / logcat)
  - [ ] Implement `debug.draw_point(x, y, color)` — draw a debug point in world space
  - [ ] Implement `debug.show_physics()` — toggle physics debug visualization
  - [ ] Implement `debug.show_fps()` — toggle FPS counter overlay
  - [ ] Register all debug functions as Lua globals

- [ ] **5.4 — Deprecation cleanup — remove Rust game code**
  - Files: `project/levels/` (remove entire directory), `project/lib.rs` (modify)
  - [ ] Remove `project/levels/main_level.rs`
  - [ ] Remove `project/levels/menu_level.rs`
  - [ ] Remove `project/levels/lighting_level.rs`
  - [ ] Remove `project/levels/random_spawns.rs`
  - [ ] Remove `project/levels/day_night_cycle.rs`
  - [ ] Remove `project/levels/shared.rs`
  - [ ] Remove `project/levels/mod.rs`
  - [ ] Remove `DonutGame` struct, `ActiveLevel` enum, `SharedState`, `GameEvent`, `Action` from `project/lib.rs`
  - [ ] Clean up `project/lib.rs` to only contain `ScriptedGame` instantiation and platform entry points
  - [ ] Remove old `DonutGame` dependencies from root `Cargo.toml` if no longer needed

- [ ] **5.5 — Clean up engine APIs (if applicable)**
  - Files: various engine crates (modify as needed)
  - [ ] Evaluate whether Action generics on `Engine<A>`, `Ctx<S>`, `Level<S>` are still needed internally
  - [ ] If no longer needed by any consumer, simplify by removing unused generics
  - [ ] Ensure all engine public API changes are backward-compatible or documented as breaking

### Tests

- [ ] **5.T1 — Create `scripting_dx.rs` test file**
  - File: `unison2d/crates/unison-tests/tests/scripting_dx.rs`
  - [ ] Test: Error recovery — execute a script with a runtime error in `update()`, verify `ScriptedGame` doesn't panic and continues calling `render()` (error overlay mode)
  - [ ] Test: Error messages — Lua errors include file name, line number, and stack trace in the error string
  - [ ] Test: Hot reload — call `ScriptedGame::reload()` with new script source, verify new `update`/`render` functions take effect on next frame
  - [ ] Test: Hot reload preserves world — after reload, World objects and physics state are unchanged
  - [ ] Test: Debug utilities — `debug.log()` output is captured (via mock logger), `debug.draw_point()` produces a render command

### Documentation

- [ ] **5.D1 — Create `getting-started.md` scripting guide**
  - File: `unison2d/docs/scripting/getting-started.md` (create)
  - [ ] First Lua game tutorial — step-by-step from empty project to running game
  - [ ] First TypeScript game tutorial — same but with TSTL

- [ ] **5.D2 — Create `api-reference.md` complete Lua API reference**
  - File: `unison2d/docs/scripting/api-reference.md` (create)
  - [ ] Document all Lua globals: `engine`, `input`, `events`, `World`, `Color`, `Rng`, `math`, `debug`
  - [ ] Document all methods with parameters and return types
  - [ ] Include code examples for common patterns

- [ ] **5.D3 — Create `migration-guide.md`**
  - File: `unison2d/docs/scripting/migration-guide.md` (create)
  - [ ] Document how to port a Rust game to Lua
  - [ ] Before/after examples for each subsystem (World, Input, Events, Levels→Scenes, UI, Lighting)
  - [ ] Common gotchas and differences

- [ ] **5.D4 — Create `hot-reload.md`**
  - File: `unison2d/docs/scripting/hot-reload.md` (create)
  - [ ] Document how hot reload works (Level 1 vs Level 2)
  - [ ] Limitations and caveats
  - [ ] Debug mode setup instructions per platform

- [ ] **5.D5 — Update `guide/README.md`**
  - File: `unison2d/docs/guide/README.md` (update)
  - [ ] Add links to all scripting docs (`getting-started.md`, `api-reference.md`, `migration-guide.md`, `hot-reload.md`, `typescript.md`)

- [ ] **5.D6 — Update `CLAUDE.md` with scripting docs**
  - File: `unison2d/CLAUDE.md` (update)
  - [ ] Add scripting docs to the docs table/section
  - [ ] Update any references to the Rust game-code path as deprecated/removed

### Phase 5 Verification

- [ ] Change `.lua` file while web dev server runs → game updates within 1s
- [ ] Introduce Lua syntax error → error overlay with file/line/stack trace, game doesn't crash
- [ ] Fix error + hot-reload → game resumes
- [ ] `debug.log()` output appears in browser console / Xcode / logcat
- [ ] `cargo test -p unison-tests --test scripting_dx` — all tests pass
- [ ] Old Rust game code fully removed (`project/levels/` directory gone)
- [ ] Engine compiles clean without old game code
- [ ] All tasks in Phase 5 are checked off
- [ ] All tests pass
- [ ] Code compiles without errors

---

## Final Validation

When ALL phases above are marked `[x] Complete` in the Progress Summary, run the following validation process:

### Validation Steps

1. **Spawn a validation sub-agent** with the following prompt:

   > You are a validation agent. Your job is to verify that an implementation fully satisfies its master plan.
   >
   > 1. Read the master plan: `unison2d/docs/plans/scripting_system.md`
   > 2. Read this implementation task list: `unison2d/docs/plans/scripting_system_tasks.md`
   > 3. For every requirement, verification step, file change, test, and doc update in the master plan, confirm there is a corresponding checked-off task in the implementation list.
   > 4. For every file listed as "create" in the master plan, verify the file exists.
   > 5. For every test listed in the master plan, verify the test exists and passes.
   > 6. For every doc update listed in the master plan, verify the doc was updated.
   > 7. Report your findings below under "Validation Results".
   >
   > If there are gaps, list each one as a missing item. If everything is covered, confirm full compliance.

2. **The validation agent will append results below this line.**

### Validation Results

{Validation agent writes results here — do not edit above this line}
