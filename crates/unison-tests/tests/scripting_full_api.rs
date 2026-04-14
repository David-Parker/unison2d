//! Phase 3 tests for full API bindings: Lighting, Events, Scenes, Render Layers,
//! Math utilities.

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

/// Run init only (for tests that need init but not update).
fn run_init(src: &str) -> (ScriptedGame, Engine) {
    let mut game = ScriptedGame::new(src);
    let mut engine = Engine::new();
    game.init(&mut engine);
    (game, engine)
}

/// Run init + multiple updates.
fn run_script_n(src: &str, n: usize) -> (ScriptedGame, Engine) {
    let mut game = ScriptedGame::new(src);
    let mut engine = Engine::new();
    game.init(&mut engine);
    for _ in 0..n {
        game.update(&mut engine);
    }
    (game, engine)
}

// ===========================================================================
// Lighting
// ===========================================================================

#[test]
fn lighting_enable_and_set_ambient() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            w.lights:set_enabled(true)
            w.lights:set_ambient(0.1, 0.1, 0.15, 1.0)
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn lighting_add_point_light() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            w.lights:set_enabled(true)
            local light = w.lights:add_point({
                position = {0, 5},
                color = 0xFFDD44,
                intensity = 2.0,
                radius = 8.0,
                casts_shadows = true,
                shadow = "soft",
            })
            assert(light ~= nil, "add_point should return a handle")
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn lighting_add_directional_light() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            w.lights:set_enabled(true)
            local dir = w.lights:add_directional({
                direction = {-0.5, -1.0},
                color = 0xFFFFFF,
                intensity = 0.8,
                casts_shadows = true,
                shadow = { filter = "pcf5", strength = 0.8 },
            })
            assert(dir ~= nil, "add_directional should return a handle")
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn lighting_set_intensity_and_direction() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            w.lights:set_enabled(true)
            local light = w.lights:add_point({
                position = {0, 5}, color = 0xFFFFFF,
                intensity = 1.0, radius = 5.0,
            })
            w.lights:set_intensity(light, 3.0)

            local dir = w.lights:add_directional({
                direction = {-1, 0}, color = 0xFFFFFF, intensity = 0.5,
            })
            w.lights:set_direction(dir, -0.7, -1.0)
            w.lights:set_intensity(dir, 1.5)
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn lighting_ground_shadow() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            w.lights:set_enabled(true)
            w.lights:set_ground_shadow(-4.5)
            -- Disable it
            w.lights:set_ground_shadow(nil)
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn light_follow_and_unfollow() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            w:set_gravity(0)
            w.lights:set_enabled(true)

            local donut = w.objects:spawn_soft_body({
                mesh = "ring", mesh_params = {1.0, 0.25, 24, 8},
                material = "rubber", position = {0, 3},
            })
            local light = w.lights:add_point({
                position = {0, 0}, color = 0xFFFFFF,
                intensity = 2.0, radius = 8.0,
            })
            w.lights:follow(light, donut)
            w:step(1/60)

            -- Also test with offset
            w.lights:follow(light, donut, { offset = {0, 2} })
            w:step(1/60)

            w.lights:unfollow(light)
        end
        function game.update(dt) end
        return game
    "#);
}

// ===========================================================================
// Events — string-keyed
// ===========================================================================

#[test]
fn events_on_and_emit() {
    run_script(r#"
        local game = {}
        local received = false
        local received_data = nil

        function game.init()
            unison.events.on("score", function(data)
                received = true
                received_data = data
            end)
        end

        function game.update(dt)
            unison.events.emit("score", { points = 10 })
            -- Events flush at end of update, so check on next frame
        end

        return game
    "#);
}

#[test]
fn events_emit_without_data() {
    run_script(r#"
        local game = {}
        local fired = false

        function game.init()
            unison.events.on("game_over", function()
                fired = true
            end)
        end

        function game.update(dt)
            unison.events.emit("game_over")
        end

        return game
    "#);
}

#[test]
fn events_multiple_handlers() {
    run_script(r#"
        local game = {}
        local count = 0

        function game.init()
            unison.events.on("tick", function() count = count + 1 end)
            unison.events.on("tick", function() count = count + 10 end)
        end

        function game.update(dt)
            unison.events.emit("tick")
        end

        return game
    "#);
}

// ===========================================================================
// Events — collision (now on world:on_collision*)
// ===========================================================================

#[test]
fn world_on_collision_registers() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            w:on_collision(function(a, b, info)
                -- Just registering should not panic
            end)
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn world_on_collision_with_registers() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            local obj = w.objects:spawn_soft_body({
                mesh = "ring", mesh_params = {1.0, 0.25, 24, 8},
                material = "rubber", position = {0, 3},
            })
            w:on_collision_with(obj, function(other, info) end)
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn world_on_collision_between_registers() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            local a = w.objects:spawn_soft_body({
                mesh = "ring", mesh_params = {1.0, 0.25, 24, 8},
                material = "rubber", position = {0, 3},
            })
            local b = w.objects:spawn_soft_body({
                mesh = "ring", mesh_params = {1.0, 0.25, 24, 8},
                material = "rubber", position = {2, 3},
            })
            w:on_collision_between(a, b, function(info) end)
        end
        function game.update(dt) end
        return game
    "#);
}

// ===========================================================================
// Scene management
// ===========================================================================

#[test]
fn scene_set_scene_calls_on_enter() {
    run_script(r#"
        local game = {}
        local entered = false

        function game.init()
            unison.scenes.set({
                on_enter = function() entered = true end,
                update = function(dt) end,
                render = function() end,
            })
            assert(entered, "on_enter should be called by set_scene")
        end

        function game.update(dt) end
        return game
    "#);
}

#[test]
fn scene_switch_calls_exit_and_enter() {
    run_script(r#"
        local game = {}
        local log = {}

        function game.init()
            local scene_a = {
                on_enter = function() table.insert(log, "a_enter") end,
                on_exit = function() table.insert(log, "a_exit") end,
                update = function(dt) end,
            }
            local scene_b = {
                on_enter = function() table.insert(log, "b_enter") end,
                update = function(dt) end,
            }
            unison.scenes.set(scene_a)
            unison.scenes.set(scene_b)
            assert(log[1] == "a_enter", "first: a_enter")
            assert(log[2] == "a_exit", "second: a_exit")
            assert(log[3] == "b_enter", "third: b_enter")
        end

        function game.update(dt) end
        return game
    "#);
}

#[test]
fn scene_update_dispatches_to_active_scene() {
    // When scenes are active, the scene's update() is called instead of game.update()
    let mut game = ScriptedGame::new(r#"
        local game = {}
        local scene_updated = false

        function game.init()
            unison.scenes.set({
                update = function(dt)
                    scene_updated = true
                end,
            })
        end

        function game.update(dt)
            -- This should NOT be called when scene is active
            error("game.update should not be called when scene is active")
        end

        return game
    "#);
    let mut engine = Engine::new();
    game.init(&mut engine);
    // If scene dispatch works, this won't panic (game.update errors)
    game.update(&mut engine);
}

// ===========================================================================
// Render layers
// ===========================================================================

#[test]
fn render_layer_create() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            local sky = w:create_render_layer("sky", {
                lit = false,
                clear_color = 0x020206,
            })
            assert(sky ~= nil, "create_render_layer should return a handle")
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn render_layer_draw_to() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            local sky = w:create_render_layer("sky", { lit = false })
            w:draw_to(sky, "rect", {
                x = 0, y = 0, width = 10, height = 5, color = 0x1a1a2e,
            }, 0)
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn render_layer_draw_and_draw_unlit() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            -- Draw to default layer
            w:draw("rect", { x = 0, y = 0, width = 1, height = 1, color = 0xFF0000 }, 5)
            -- Draw unlit
            w:draw_unlit("line", { x1 = 0, y1 = 0, x2 = 1, y2 = 1, color = 0x00FF00 }, 10)
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn render_layer_set_clear_color() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            local layer = w:create_render_layer("bg", { lit = false, clear_color = 0x000000 })
            w:set_layer_clear_color(layer, 0xFF0000)
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn render_layer_default_layer() {
    run_script(r#"
        local game = {}
        function game.init()
            local w = unison.World.new()
            local default = w:default_layer()
            assert(default ~= nil, "default_layer should return a handle")
        end
        function game.update(dt) end
        return game
    "#);
}

// ===========================================================================
// Math utilities
// ===========================================================================

#[test]
fn math_color_hex() {
    run_script(r#"
        local game = {}
        function game.init()
            local c = unison.Color.hex(0xFF0000)
            assert(c.r > 0.9, "red channel should be ~1.0, got " .. c.r)
            assert(c.g < 0.1, "green channel should be ~0.0")
            assert(c.b < 0.1, "blue channel should be ~0.0")
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn math_color_rgba() {
    run_script(r#"
        local game = {}
        function game.init()
            local c = unison.Color.rgba(0.5, 0.6, 0.7, 0.8)
            assert(math.abs(c.r - 0.5) < 0.01)
            assert(math.abs(c.g - 0.6) < 0.01)
            assert(math.abs(c.b - 0.7) < 0.01)
            assert(math.abs(c.a - 0.8) < 0.01)
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn math_color_lerp() {
    run_script(r#"
        local game = {}
        function game.init()
            local a = unison.Color.rgba(0, 0, 0, 1)
            local b = unison.Color.rgba(1, 1, 1, 1)
            local mid = a:lerp(b, 0.5)
            assert(math.abs(mid.r - 0.5) < 0.01, "lerp r should be 0.5")
            assert(math.abs(mid.g - 0.5) < 0.01, "lerp g should be 0.5")
            assert(math.abs(mid.b - 0.5) < 0.01, "lerp b should be 0.5")
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn math_rng_deterministic() {
    run_script(r#"
        local game = {}
        function game.init()
            local rng1 = unison.Rng.new(42)
            local rng2 = unison.Rng.new(42)
            local a = rng1:range(0, 100)
            local b = rng2:range(0, 100)
            assert(a == b, "same seed should produce same values: " .. a .. " vs " .. b)
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn math_rng_range_int() {
    run_script(r#"
        local game = {}
        function game.init()
            local rng = unison.Rng.new(123)
            for i = 1, 100 do
                local v = rng:range_int(1, 6)
                assert(v >= 1 and v <= 6, "range_int out of bounds: " .. v)
            end
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn math_lerp() {
    run_script(r#"
        local game = {}
        function game.init()
            assert(unison.math.lerp(0, 100, 0.5) == 50, "lerp(0,100,0.5) should be 50")
            assert(unison.math.lerp(10, 20, 0) == 10, "lerp at 0")
            assert(unison.math.lerp(10, 20, 1) == 20, "lerp at 1")
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn math_smoothstep() {
    run_script(r#"
        local game = {}
        function game.init()
            local v = unison.math.smoothstep(0, 1, 0.5)
            assert(math.abs(v - 0.5) < 0.01, "smoothstep(0,1,0.5) should be ~0.5, got " .. v)
            assert(unison.math.smoothstep(0, 1, 0) == 0, "smoothstep at 0")
            assert(unison.math.smoothstep(0, 1, 1) == 1, "smoothstep at 1")
        end
        function game.update(dt) end
        return game
    "#);
}

#[test]
fn math_clamp() {
    run_script(r#"
        local game = {}
        function game.init()
            assert(unison.math.clamp(5, 0, 10) == 5, "clamp in range")
            assert(unison.math.clamp(-5, 0, 10) == 0, "clamp below")
            assert(unison.math.clamp(15, 0, 10) == 10, "clamp above")
        end
        function game.update(dt) end
        return game
    "#);
}

// ===========================================================================
// UI
// ===========================================================================

#[test]
fn ui_create_and_frame_no_panic() {
    run_script(r#"
        local game = {}
        local ui

        function game.init()
            ui = unison.UI.new("fonts/test.ttf")
        end

        function game.update(dt) end

        function game.render()
            ui:frame({
                { type = "column", anchor = "center", gap = 10, children = {
                    { type = "label", text = "Hello" },
                    { type = "button", text = "Click Me", on_click = "test_click" },
                }},
            })
        end

        return game
    "#);
}

// ===========================================================================
// Integration: scene with lighting
// ===========================================================================

#[test]
fn scene_with_lighting_integration() {
    run_script(r#"
        local game = {}

        function game.init()
            unison.scenes.set({
                on_enter = function()
                    local w = unison.World.new()
                    w.lights:set_enabled(true)
                    w.lights:set_ambient(0.05, 0.05, 0.1, 1.0)
                    w.lights:add_point({
                        position = {0, 5}, color = 0xFFDD44,
                        intensity = 2.0, radius = 8.0,
                    })
                end,
                update = function(dt) end,
            })
        end

        function game.update(dt) end
        return game
    "#);
}
