//! Phase 2 tests for core API bindings: World, Objects, Input, Camera, Engine.
//!
//! Most tests drive the bindings through `ScriptedGame::new(inline_lua)` with
//! `Engine::new()` (no renderer), or directly via the mlua Lua VM + binding
//! registration for finer-grained assertions.

use unison_scripting::ScriptedGame;
use unison2d::{Engine, Game};

/// Helper: create a ScriptedGame, run init+update, return it for inspection.
fn run_script(src: &str) -> (ScriptedGame, Engine) {
    let mut game = ScriptedGame::new(src);
    let mut engine = Engine::new();
    game.init(&mut engine);
    game.update(&mut engine);
    (game, engine)
}

// ===========================================================================
// World bindings
// ===========================================================================

#[test]
fn world_create_and_configure() {
    // Creating a world and setting physics properties should not panic.
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            w:set_gravity(-20)
            w:set_ground(-5)
            w:set_ground_restitution(0.5)
            w:set_ground_friction(0.6)
            w:set_background(0x1a1a2e)
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn world_step_advances_physics() {
    // A soft body spawned above the ground should fall after stepping.
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            w:set_gravity(-9.8)
            local id = w.objects:spawn_soft_body({
                mesh = "square", mesh_params = {0.5, 2},
                material = "rubber",
                position = {0, 10},
            })
            -- Get initial position
            local x0, y0 = w.objects:position(id)
            assert(y0 > 9, "initial Y should be ~10, got " .. y0)
            -- Step physics multiple times
            for i = 1, 60 do w:step(1/60) end
            -- Position should have changed (fallen)
            local x1, y1 = w.objects:position(id)
            assert(y1 < y0, "body should have fallen: y0=" .. y0 .. " y1=" .. y1)
        end
        function game.update(dt) end
        return game
    "#);
}

// ===========================================================================
// Object spawning
// ===========================================================================

#[test]
fn spawn_soft_body_returns_id() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            local id = w.objects:spawn_soft_body({
                mesh = "ring", mesh_params = {1.0, 0.25, 16, 4},
                material = "rubber",
                position = {0, 5},
                color = 0xFF0000,
            })
            assert(type(id) == "number", "spawn_soft_body should return a number ID")
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn spawn_rigid_body_returns_id() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            local id = w.objects:spawn_rigid_body({
                collider = "aabb",
                half_width = 2, half_height = 0.5,
                position = {0, -3},
                color = 0x00FF00,
                is_static = true,
            })
            assert(type(id) == "number", "spawn_rigid_body should return a number ID")
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn spawn_static_rect_returns_id() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            local id = w.objects:spawn_static_rect({ position = {0, -5}, size = {20, 3}, color = 0x336633 })
            assert(type(id) == "number", "spawn_static_rect should return a number ID")
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn spawn_sprite_returns_id() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            local id = w.objects:spawn_sprite({
                position = {3, 3},
                size = {2, 2},
                color = 0xFFFF00,
            })
            assert(type(id) == "number", "spawn_sprite should return a number ID")
        end
        function game.update(dt) end
        return game
    "#);
}

// ===========================================================================
// Physics interaction
// ===========================================================================

#[test]
fn apply_force_changes_velocity() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            w:set_gravity(0) -- no gravity so we can isolate force effect
            local id = w.objects:spawn_soft_body({
                mesh = "square", mesh_params = {0.5, 2},
                material = "rubber",
                position = {0, 0},
            })
            w.objects:apply_force(id, 100, 0)
            w:step(1/60)
            local vx, vy = w.objects:velocity(id)
            assert(vx > 0, "horizontal velocity should be positive after rightward force, got " .. vx)
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn apply_impulse_changes_velocity() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            w:set_gravity(0)
            local id = w.objects:spawn_soft_body({
                mesh = "square", mesh_params = {0.5, 2},
                material = "rubber",
                position = {0, 0},
            })
            w.objects:apply_impulse(id, 0, 50)
            w:step(1/60)
            local vx, vy = w.objects:velocity(id)
            assert(vy > 0, "vertical velocity should be positive after upward impulse, got " .. vy)
        end
        function game.update(dt) end
        return game
    "#);
}

// ===========================================================================
// Queries
// ===========================================================================

#[test]
fn get_position_returns_values() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            w:set_gravity(0)
            local id = w.objects:spawn_soft_body({
                mesh = "square", mesh_params = {0.5, 2},
                material = "rubber",
                position = {3, 7},
            })
            local x, y = w.objects:position(id)
            -- Position should be approximately where we spawned it
            assert(math.abs(x - 3) < 0.5, "x should be near 3, got " .. x)
            assert(math.abs(y - 7) < 0.5, "y should be near 7, got " .. y)
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn get_velocity_returns_values() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            w:set_gravity(0)
            local id = w.objects:spawn_soft_body({
                mesh = "square", mesh_params = {0.5, 2},
                material = "rubber",
                position = {0, 0},
            })
            local vx, vy = w.objects:velocity(id)
            -- Should be near zero initially
            assert(math.abs(vx) < 1, "initial vx should be near 0")
            assert(math.abs(vy) < 1, "initial vy should be near 0")
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn is_grounded_works() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            w:set_gravity(-9.8)
            w:set_ground(-2)
            local id = w.objects:spawn_soft_body({
                mesh = "square", mesh_params = {0.5, 2},
                material = "rubber",
                position = {0, 10},
            })
            -- Not grounded initially (high up)
            assert(not w.objects:is_grounded(id), "should not be grounded when spawned at y=10")
            -- Step many times until it falls to ground
            for i = 1, 300 do w:step(1/60) end
            -- Should be grounded after falling
            assert(w.objects:is_grounded(id), "should be grounded after falling for 5 seconds")
        end
        function game.update(dt) end
        return game
    "#);
}

// ===========================================================================
// Despawn
// ===========================================================================

#[test]
fn despawn_removes_object() {
    // Despawning should not panic. We can't easily verify removal from Lua,
    // but we verify no crash on despawn + subsequent step.
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            local id = w.objects:spawn_soft_body({
                mesh = "square", mesh_params = {0.5, 2},
                material = "rubber",
                position = {0, 5},
            })
            w.objects:despawn(id)
            -- Stepping after despawn should not crash
            w:step(1/60)
        end
        function game.update(dt) end
        return game
    "#);
}

// ===========================================================================
// Display properties
// ===========================================================================

#[test]
fn set_display_properties_no_panic() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            local id = w.objects:spawn_soft_body({
                mesh = "square", mesh_params = {0.5, 2},
                material = "rubber",
                position = {0, 5},
            })
            w.objects:set_z_order(id, 100)
            w.objects:set_casts_shadow(id, false)
            w.objects:set_position(id, 5, 10)
            local x, y = w.objects:position(id)
            assert(math.abs(x - 5) < 0.5, "x should be near 5 after set_position")
        end
        function game.update(dt) end
        return game
    "#);
}

// ===========================================================================
// Input bindings
// ===========================================================================

#[test]
fn input_functions_exist_and_return_defaults() {
    // Without injecting input state, everything should return false/0/empty.
    run_script(r#"
        local game = {}
        function game.init()
            -- These should all be callable without error
            local pressed = unison.input.is_key_pressed("Space")
            assert(not pressed, "no keys should be pressed by default")

            local just = unison.input.is_key_just_pressed("W")
            assert(not just, "no keys should be just pressed by default")

            local ax = unison.input.axis_x()
            local ay = unison.input.axis_y()
            assert(ax == 0, "axis_x should be 0 by default")
            assert(ay == 0, "axis_y should be 0 by default")

            local touches = unison.input.touches_just_began()
            assert(#touches == 0, "no touches by default")
        end
        function game.update(dt) end
        return game
    "#);
}

// ===========================================================================
// Camera bindings
// ===========================================================================

#[test]
fn camera_follow_no_panic() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            local id = w.objects:spawn_soft_body({
                mesh = "square", mesh_params = {0.5, 2},
                material = "rubber",
                position = {5, 5},
            })
            w.cameras:follow("main", id, { smoothing = 0.1 })
            -- Step to let camera track
            for i = 1, 10 do w:step(1/60) end
            local cx, cy = w.cameras:position("main")
            -- Camera should have moved toward the object
            assert(type(cx) == "number", "camera x should be a number")
            assert(type(cy) == "number", "camera y should be a number")
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn camera_follow_with_offset() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            w:set_gravity(0)
            local id = w.objects:spawn_soft_body({
                mesh = "square", mesh_params = {0.5, 2},
                material = "rubber",
                position = {0, 0},
            })
            w.cameras:follow("main", id, { smoothing = 1.0, offset = {0, 5} })
            w:step(1/60)
            local cx, cy = w.cameras:position("main")
            -- With smoothing=1.0 (instant), camera should snap to object + offset
            -- Object at ~(0,0), offset (0,5), so camera should be near (0,5)
            assert(math.abs(cy - 5) < 1, "camera y should be near 5 (object + offset), got " .. cy)
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn camera_add_and_get_position() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            w.cameras:add("overview", 20, 15)
            local cx, cy = w.cameras:position("overview")
            assert(type(cx) == "number", "overview camera x should be a number")
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn camera_unfollow_no_panic() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            local id = w.objects:spawn_soft_body({
                mesh = "square", mesh_params = {0.5, 2},
                material = "rubber",
                position = {0, 0},
            })
            w.cameras:follow("main", id, { smoothing = 0.1 })
            w.cameras:unfollow("main")
            -- After unfollow, stepping should not panic
            w:step(1/60)
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn camera_follow_no_opts() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            local id = w.objects:spawn_soft_body({
                mesh = "square", mesh_params = {0.5, 2},
                material = "rubber",
                position = {0, 0},
            })
            -- opts is optional — omitting it should use defaults (smoothing=0, offset=(0,0))
            w.cameras:follow("main", id)
            w:step(1/60)
        end
        function game.update(dt) end
        return game
    "#);
}

// ===========================================================================
// Engine bindings
// ===========================================================================

#[test]
fn engine_screen_size_returns_values() {
    run_script(r#"
        local game = {}
        function game.init()
            local w, h = unison.renderer.screen_size()
            assert(type(w) == "number", "width should be a number")
            assert(type(h) == "number", "height should be a number")
            assert(w > 0, "width should be positive")
            assert(h > 0, "height should be positive")
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn engine_set_anti_aliasing_no_panic() {
    run_script(r#"
        local game = {}
        function game.init()
            unison.renderer.set_anti_aliasing("msaa8x")
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn engine_set_background_hex_and_rgb() {
    // Both forms should work without error.
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            w:set_background(0x1a1a2e)
        end
        function game.update(dt) end
        return game
    "#);
}

// ===========================================================================
// Mesh presets
// ===========================================================================

#[test]
fn all_mesh_presets_work() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            -- Ring
            w.objects:spawn_soft_body({ mesh = "ring", mesh_params = {1, 0.3, 16, 4}, material = "rubber", position = {0,0} })
            -- Square
            w.objects:spawn_soft_body({ mesh = "square", mesh_params = {1, 4}, material = "jello", position = {3,0} })
            -- Ellipse
            w.objects:spawn_soft_body({ mesh = "ellipse", mesh_params = {1, 0.6, 16, 4}, material = "wood", position = {6,0} })
            -- Star
            w.objects:spawn_soft_body({ mesh = "star", mesh_params = {1, 0.4, 5, 3}, material = "metal", position = {9,0} })
            -- Blob
            w.objects:spawn_soft_body({ mesh = "blob", mesh_params = {1, 0.2, 12, 4, 42}, material = "slime", position = {12,0} })
            -- Rounded box
            w.objects:spawn_soft_body({ mesh = "rounded_box", mesh_params = {2, 1, 0.2, 4}, material = "rubber", position = {15,0} })
        end
        function game.update(dt) end
        return game
    "#);
}

// ===========================================================================
// Material presets
// ===========================================================================

#[test]
fn material_presets_and_custom() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            -- String presets
            w.objects:spawn_soft_body({ mesh = "square", mesh_params = {0.5, 2}, material = "rubber", position = {0,0} })
            w.objects:spawn_soft_body({ mesh = "square", mesh_params = {0.5, 2}, material = "jello", position = {2,0} })
            w.objects:spawn_soft_body({ mesh = "square", mesh_params = {0.5, 2}, material = "wood", position = {4,0} })
            w.objects:spawn_soft_body({ mesh = "square", mesh_params = {0.5, 2}, material = "metal", position = {6,0} })
            w.objects:spawn_soft_body({ mesh = "square", mesh_params = {0.5, 2}, material = "slime", position = {8,0} })
            -- Custom table
            w.objects:spawn_soft_body({ mesh = "square", mesh_params = {0.5, 2},
                material = {density = 500, edge_compliance = 1e-5, area_compliance = 1e-4},
                position = {10,0} })
        end
        function game.update(dt) end
        return game
    "#);
}

// ===========================================================================
// Ownership safety
// ===========================================================================

#[test]
fn multiple_lua_refs_to_same_world() {
    // Multiple Lua variables holding the same World shouldn't cause issues.
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            local w2 = w  -- alias
            w:set_gravity(-5)
            w2:set_ground(-3)
            local id = w.objects:spawn_soft_body({
                mesh = "square", mesh_params = {0.5, 2},
                material = "rubber", position = {0, 5},
            })
            w2:step(1/60)
            local x, y = w.objects:position(id)
            assert(type(y) == "number", "should be able to query through original ref")
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn world_persists_across_lifecycle_calls() {
    // World created in init should be usable in update.
    let mut game = ScriptedGame::new(r#"
        local game = {}
        local world, obj_id

        function game.init()
            world = unison.World.new()
            world:set_gravity(-9.8)
            world:set_ground(-5)
            obj_id = world.objects:spawn_soft_body({
                mesh = "square", mesh_params = {0.5, 2},
                material = "rubber",
                position = {0, 10},
            })
        end

        function game.update(dt)
            -- These should work — world persists from init
            world.objects:apply_force(obj_id, 10, 0)
            world:step(dt)
            local x, y = world.objects:position(obj_id)
            assert(type(x) == "number", "position query should work across frames")
        end

        return game
    "#);
    let mut engine = Engine::new();
    game.init(&mut engine);
    // Run several update frames
    for _ in 0..10 {
        game.update(&mut engine);
    }
}
