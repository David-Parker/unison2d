// {{PROJECT_NAME}} — entry point.
// See https://github.com/David-Parker/unison2d/blob/main/docs/scripting/getting-started/typescript.md
//
// Demo: a rubber soft-body circle textured with the Unison 2D logo falls under
// gravity and bounces on a ground plane. Press and hold anywhere on screen
// (touch or mouse) to pull the circle toward that point for as long as you
// hold. Replace with your own game logic.

let world: World;
let logo_id: ObjectId;
let ui: UI;

const game: Game = {
    init() {
        print("{{PROJECT_NAME}} loaded");

        const logo_tex = unison.assets.load_texture("textures/logo.png");
        ui = unison.UI.new("fonts/DejaVuSans-Bold.ttf");

        world = unison.World.new();
        world.set_background(0x1a1a2e);
        world.set_gravity(-9.8);
        world.set_ground(-4.5);
        world.set_ground_restitution(0.7);

        logo_id = world.spawn_soft_body({
            mesh: "ellipse",
            mesh_params: [2.0, 2.0, 24, 3],
            material: "rubber",
            position: [0, 4],
            texture: logo_tex,
        });
    },

    update(dt: number) {
        const [sx, sy] = unison.input.pointer_position();
        if (sx !== undefined && sy !== undefined) {
            const [tx, ty] = world.screen_to_world(sx, sy);
            const [px, py] = world.get_position(logo_id);
            const dx = tx - px;
            const dy = ty - py;
            const mag = Math.sqrt(dx * dx + dy * dy);
            if (mag > 1e-4) {
                const strength = 500.0;
                world.apply_force(logo_id, (dx / mag) * strength, (dy / mag) * strength);
            }
        }
        world.step(dt);
    },

    render() {
        world.render();
        ui.frame([
            { type: "column", anchor: "top", padding: 16, children: [
                { type: "label", text: "Unison 2D Game", font_size: 28, font_color: 0xffffff },
            ] },
        ]);
    },
};

export = game;
