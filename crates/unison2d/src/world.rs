//! World — a self-contained game world with physics, objects, cameras, and lighting.
//!
//! Each Level (or the Game itself) owns a `World`. Multiple worlds can coexist
//! independently. The `World` struct composes subsystems:
//!
//! - `objects` ([`ObjectSystem`]) — physics simulation + object registry
//! - `cameras` ([`CameraSystem`]) — named cameras with optional follow targets
//! - `lighting` ([`LightingSystem`]) — dynamic lights and shadows

use unison_math::Color;
use unison_render::{Renderer, RenderTargetId};
use unison_lighting::LightingSystem;

use crate::object_system::ObjectSystem;
use crate::camera_system::CameraSystem;

/// A self-contained game world.
///
/// Composes subsystems for physics/objects, cameras, and lighting.
/// Call `step(dt)` each tick to advance the simulation.
///
/// ```ignore
/// let mut world = World::new();
/// world.set_background(Color::from_hex(0x1a1a2e));
/// world.objects.set_gravity(Vec2::new(0.0, -9.8));
/// let player = world.objects.spawn_soft_body(desc);
/// world.cameras.follow("main", player, 0.08);
///
/// // Each tick:
/// world.step(dt);
/// ```
pub struct World {
    /// Object and physics subsystem.
    pub objects: ObjectSystem,
    /// Camera subsystem with named cameras.
    pub cameras: CameraSystem,
    /// Lighting subsystem.
    pub lighting: LightingSystem,
    /// Background clear color.
    background_color: Color,
}

impl World {
    /// Create a new World with default settings.
    ///
    /// Comes with a default "main" camera (20x15 world units) and
    /// standard gravity (-9.8).
    pub fn new() -> Self {
        Self {
            objects: ObjectSystem::new(),
            cameras: CameraSystem::new(),
            lighting: LightingSystem::new(),
            background_color: Color::BLACK,
        }
    }

    /// Set the background clear color.
    pub fn set_background(&mut self, color: Color) {
        self.background_color = color;
    }

    /// Get the background clear color.
    pub fn background_color(&self) -> Color {
        self.background_color
    }

    /// Advance the world by one timestep.
    ///
    /// Steps physics, then updates camera follow targets.
    pub fn step(&mut self, dt: f32) {
        self.objects.step(dt);
        self.cameras.update_follows(&self.objects);
    }

    /// Snapshot physics state for interpolated rendering.
    pub fn snapshot_for_render(&mut self) {
        self.objects.snapshot_for_render();
    }

    /// Render all objects through the "main" camera to the currently-bound target.
    ///
    /// This is a convenience method for simple single-camera setups.
    /// For multi-camera rendering, use `render_to_targets()` instead.
    pub fn auto_render(&self, renderer: &mut dyn Renderer<Error = String>) {
        let camera = match self.cameras.get("main") {
            Some(c) => c,
            None => return,
        };

        renderer.begin_frame(camera);
        renderer.clear(self.background_color);

        for cmd in self.objects.render_commands() {
            renderer.draw(cmd);
        }

        renderer.end_frame();
    }

    /// Render all objects through each named camera into its assigned render target.
    ///
    /// For each `(camera_name, target_id)` pair: binds the target, renders the scene
    /// through that camera, and ends the frame. Use with `Engine::composite_layer()`
    /// to arrange outputs on screen.
    pub fn render_to_targets(
        &self,
        renderer: &mut dyn Renderer<Error = String>,
        camera_targets: &[(&str, RenderTargetId)],
    ) {
        let commands = self.objects.render_commands();

        for &(cam_name, target_id) in camera_targets {
            let camera = match self.cameras.get(cam_name) {
                Some(c) => c,
                None => continue,
            };

            renderer.bind_render_target(target_id);
            renderer.begin_frame(camera);
            renderer.clear(self.background_color);

            for cmd in &commands {
                renderer.draw(cmd.clone());
            }

            renderer.end_frame();
        }
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unison_math::Vec2;
    use unison_physics::mesh::create_ring_mesh;
    use crate::object::{SoftBodyDesc, RigidBodyDesc};

    #[test]
    fn world_new_has_defaults() {
        let world = World::new();
        assert!(world.cameras.get("main").is_some());
        assert_eq!(world.background_color(), Color::BLACK);
        assert_eq!(world.objects.object_count(), 0);
    }

    #[test]
    fn world_step_advances_physics() {
        let mut world = World::new();
        let mesh = create_ring_mesh(1.0, 0.5, 8, 2);

        let id = world.objects.spawn_soft_body(SoftBodyDesc {
            mesh,
            material: unison_physics::Material::RUBBER,
            position: Vec2::new(0.0, 5.0),
            color: Color::WHITE,
        });

        let pos_before = world.objects.get_position(id);

        // Step a few times — gravity should pull the object down
        for _ in 0..10 {
            world.step(1.0 / 60.0);
        }

        let pos_after = world.objects.get_position(id);
        assert!(pos_after.y < pos_before.y, "Object should have fallen");
    }

    #[test]
    fn world_camera_follow_updates() {
        let mut world = World::new();
        let mesh = create_ring_mesh(1.0, 0.5, 8, 2);

        let id = world.objects.spawn_soft_body(SoftBodyDesc {
            mesh,
            material: unison_physics::Material::RUBBER,
            position: Vec2::new(10.0, 5.0),
            color: Color::WHITE,
        });

        world.cameras.follow("main", id, 1.0); // instant snap
        world.step(1.0 / 60.0);

        let cam = world.cameras.get("main").unwrap();
        // Camera should have moved toward the object
        assert!(cam.x.abs() > 0.0 || cam.y.abs() > 0.0);
    }

    #[test]
    fn set_background() {
        let mut world = World::new();
        let color = Color::from_hex(0x1a1a2e);
        world.set_background(color);
        assert_eq!(world.background_color().r, color.r);
    }
}
