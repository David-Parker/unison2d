-- Minimal Lua game — one scene, input, events.
local game = {}

function game.init()
    local demo = require("scenes/demo")
    engine.set_scene(demo)
end

function game.update(dt) end
function game.render() end

return game
