// Minimal TypeScript game — one scene, input, events.
import * as demo from "./scenes/demo";

const game: Game = {
    init() {
        engine.set_scene(demo);
    },
    update(dt: number) {},
    render() {},
};

export = game;
