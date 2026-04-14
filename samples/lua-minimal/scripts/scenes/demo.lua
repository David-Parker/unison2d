-- Demo scene: world with a box, arrow key movement, event subscription.
local scene = {}

local world
local box_id

function scene.on_enter()
    world = unison.World.new()
    world:set_background(0x1a1a2e)
    world:set_gravity(-9.8)
    world:set_ground(-4.5)

    box_id = world:spawn_rigid_body({
        collider = "aabb",
        half_width = 0.5,
        half_height = 0.5,
        position = {0, 2},
        color = 0xFF6600,
    })

    world:camera_follow("main", box_id, 0.1)

    unison.events.on("test_event", function(data)
        unison.debug.log("received test_event")
    end)
end

function scene.update(dt)
    if unison.input.is_key_pressed("ArrowLeft") or unison.input.is_key_pressed("A") then
        world:apply_force(box_id, -5, 0)
    end
    if unison.input.is_key_pressed("ArrowRight") or unison.input.is_key_pressed("D") then
        world:apply_force(box_id, 5, 0)
    end
    if unison.input.is_key_just_pressed("Space") and world:is_grounded(box_id) then
        world:apply_impulse(box_id, 0, 5)
    end

    world:step(dt)
end

function scene.render()
    world:render()
end

function scene.on_exit()
    unison.events.clear()
    world = nil
    box_id = nil
end

return scene
