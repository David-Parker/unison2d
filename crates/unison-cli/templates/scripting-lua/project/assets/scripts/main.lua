-- {{PROJECT_NAME}} — entry point.
-- See https://github.com/David-Parker/unison2d/blob/main/docs/scripting/getting-started/lua.md
--
-- Demo: a rubber soft-body circle textured with the Unison 2D logo falls under
-- gravity and bounces on a ground plane. Press and hold anywhere on screen
-- (touch or mouse) to pull the circle toward that point for as long as you
-- hold. Replace with your own game logic.

local game = {}

local world
local logo_id
local ui

function game.init()
    print("{{PROJECT_NAME}} loaded")

    local logo_tex = unison.assets.load_texture("textures/logo.png")
    ui = unison.UI.new("fonts/DejaVuSans-Bold.ttf")

    world = unison.World.new()
    world:set_background(0x1a1a2e)
    world:set_gravity(-9.8)
    world:set_ground(-4.5)
    world:set_ground_restitution(0.7)

    logo_id = world:spawn_soft_body({
        mesh = "ellipse",
        mesh_params = {2.0, 2.0, 24, 3},  -- width, height, segments, rings
        material = "rubber",
        position = {0, 4},
        texture = logo_tex,
    })
end

function game.update(dt)
    local sx, sy = unison.input.pointer_position()
    if sx then
        local tx, ty = world:screen_to_world(sx, sy)
        local px, py = world:get_position(logo_id)
        local dx, dy = tx - px, ty - py
        local mag = math.sqrt(dx * dx + dy * dy)
        if mag > 1e-4 then
            local strength = 500.0
            world:apply_force(logo_id, dx / mag * strength, dy / mag * strength)
        end
    end
    world:step(dt)
end

function game.render()
    world:render()
    ui:frame({
        { type = "column", anchor = "top", padding = 16, children = {
            { type = "label", text = "Unison 2D Game", font_size = 28, font_color = 0xffffff },
        } },
    })
end

return game
