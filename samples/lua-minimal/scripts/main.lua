-- Minimal Lua game — one scene, input, events.
local game = {}

function game.init()
    local demo = require("scenes/demo")
    unison.scenes.set(demo)
end

function game.update(dt) end
function game.render() end

return game
