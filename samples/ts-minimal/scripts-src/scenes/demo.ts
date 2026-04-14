// Demo scene: world with a box, arrow key movement, event subscription.

let world: World;
let box_id: ObjectId;

const scene: Scene = {
    on_enter() {
        world = World.new();
        world.set_background(0x1a1a2e);
        world.set_gravity(-9.8);
        world.set_ground(-4.5);

        box_id = world.spawn_rigid_body({
            collider: "aabb",
            half_width: 0.5,
            half_height: 0.5,
            position: [0, 2],
            color: 0xFF6600,
        });

        world.cameras.follow("main", box_id, { smoothing: 0.1 });

        events.on("test_event", (data) => {
            debug.log("received test_event");
        });
    },

    update(dt: number) {
        if (input.is_key_pressed("ArrowLeft") || input.is_key_pressed("A")) {
            world.apply_force(box_id, -5, 0);
        }
        if (input.is_key_pressed("ArrowRight") || input.is_key_pressed("D")) {
            world.apply_force(box_id, 5, 0);
        }
        if (input.is_key_just_pressed("Space") && world.is_grounded(box_id)) {
            world.apply_impulse(box_id, 0, 5);
        }

        world.step(dt);
    },

    render() {
        world.render();
    },

    on_exit() {
        events.clear();
        world = undefined!;
        box_id = undefined!;
    },
};

export = scene;
