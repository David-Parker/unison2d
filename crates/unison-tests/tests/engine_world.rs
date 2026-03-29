//! World-level integration tests
//!
//! Tests that exercise the engine's World composition (ObjectSystem + CameraSystem + physics).

use unison2d::{World, SoftBodyDesc};
use unison_physics::Material;
use unison_physics::mesh::create_ring_mesh;
use unison_core::{Vec2, Color};
use unison_render::TextureId;

#[test]
fn world_step_advances_physics() {
    let mut world = World::new();
    let mesh = create_ring_mesh(1.0, 0.5, 8, 2);

    let id = world.objects.spawn_soft_body(SoftBodyDesc {
        mesh,
        material: Material::RUBBER,
        position: Vec2::new(0.0, 5.0),
        color: Color::WHITE,
        texture: TextureId::NONE,
    });

    let pos_before = world.objects.get_position(id);

    // Step a few times — gravity should pull the object down
    for _ in 0..10 {
        world.step(1.0 / 60.0);
    }

    let pos_after = world.objects.get_position(id);
    assert!(pos_after.y < pos_before.y, "Object should have fallen");
}
