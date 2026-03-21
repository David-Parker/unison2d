//! ObjectSystem — manages game objects and their underlying physics simulation.
//!
//! Owns the `PhysicsWorld` and the object registry. Provides a high-level API for
//! spawning, querying, and applying forces to objects. Raw physics access is
//! available via `physics()` / `physics_mut()`.

use std::collections::HashMap;

use unison_math::{Color, Vec2};
use unison_physics::{BodyHandle, PhysicsWorld};
use unison_render::{DrawMesh, DrawSprite, RenderCommand};
use unison_lighting::Occluder;
use crate::object::{ObjectEntry, ObjectId, ObjectKind, RigidBodyDesc, SoftBodyDesc, SpriteDesc};

/// Manages game objects and their physics simulation.
///
/// Each object is a soft body or rigid body backed by a `BodyHandle` in the
/// underlying `PhysicsWorld`. The system tracks the mapping between `ObjectId`
/// (game-facing) and `BodyHandle` (physics-facing).
pub struct ObjectSystem {
    physics: PhysicsWorld,
    entries: HashMap<ObjectId, ObjectEntry>,
    handle_map: HashMap<BodyHandle, ObjectId>,
    next_id: u64,
}

impl ObjectSystem {
    /// Create a new ObjectSystem with default physics settings.
    pub fn new() -> Self {
        let mut physics = PhysicsWorld::new();
        physics.set_gravity(-9.8);

        Self {
            physics,
            entries: HashMap::new(),
            handle_map: HashMap::new(),
            next_id: 0,
        }
    }

    // ── Spawning ──

    /// Spawn a soft body object. Returns an ObjectId for future reference.
    pub fn spawn_soft_body(&mut self, mut desc: SoftBodyDesc) -> ObjectId {
        // Precompute boundary edges for shadow casting
        desc.mesh.ensure_boundary_edges();
        let boundary_edges = desc.mesh.boundary_edges.clone();

        let uvs = desc.mesh.uvs_or_default();
        let config = desc.to_body_config();
        let handle = self.physics.add_body(&desc.mesh, config);
        let color = desc.color;
        let texture = desc.texture;

        let id = self.next_object_id();
        self.handle_map.insert(handle, id);
        self.entries.insert(id, ObjectEntry {
            kind: ObjectKind::SoftBody { handle, color, texture, uvs, boundary_edges },
            casts_shadow: true,
            z_order: 0,
        });
        id
    }

    /// Spawn a rigid body object. Returns an ObjectId for future reference.
    pub fn spawn_rigid_body(&mut self, desc: RigidBodyDesc) -> ObjectId {
        let config = desc.to_rigid_body_config();
        let handle = self.physics.add_rigid_body(config);
        let color = desc.color;

        let id = self.next_object_id();
        self.handle_map.insert(handle, id);
        self.entries.insert(id, ObjectEntry {
            kind: ObjectKind::RigidBody { handle, color },
            casts_shadow: true,
            z_order: 0,
        });
        id
    }

    /// Convenience: spawn a static rectangle (platform, wall, floor).
    pub fn spawn_static_rect(&mut self, position: Vec2, size: Vec2, color: Color) -> ObjectId {
        self.spawn_rigid_body(RigidBodyDesc {
            collider: unison_physics::Collider::aabb(size.x / 2.0, size.y / 2.0),
            position,
            color,
            is_static: true,
        })
    }

    /// Spawn a sprite-only object (no physics). Returns an ObjectId.
    ///
    /// Sprites are purely visual — a textured or colored quad with a transform.
    /// Use the sprite query/set methods to move or rotate them.
    pub fn spawn_sprite(&mut self, desc: SpriteDesc) -> ObjectId {
        let id = self.next_object_id();
        self.entries.insert(id, ObjectEntry {
            kind: ObjectKind::Sprite {
                texture: desc.texture,
                position: desc.position,
                size: desc.size,
                rotation: desc.rotation,
                color: desc.color,
            },
            casts_shadow: false, // Sprites don't cast shadows by default
            z_order: 0,
        });
        id
    }

    /// Remove an object from the world.
    pub fn despawn(&mut self, id: ObjectId) {
        if let Some(entry) = self.entries.remove(&id) {
            if let Some(handle) = entry.physics_handle() {
                self.physics.remove_body(handle);
                self.handle_map.remove(&handle);
            }
        }
    }

    // ── Queries ──

    /// Get the position of an object.
    pub fn get_position(&self, id: ObjectId) -> Vec2 {
        self.with_handle(id, |h| self.physics.get_position(h).unwrap_or(Vec2::ZERO))
            .unwrap_or(Vec2::ZERO)
    }

    /// Set the position of an object.
    pub fn set_position(&mut self, id: ObjectId, pos: Vec2) {
        if let Some(handle) = self.get_handle(id) {
            self.physics.set_position(handle, pos.x, pos.y);
        }
    }

    /// Apply a force to an object (continuous, call each frame).
    pub fn apply_force(&mut self, id: ObjectId, force: Vec2) {
        if let Some(handle) = self.get_handle(id) {
            self.physics.apply_force(handle, force.x, force.y);
        }
    }

    /// Apply a torque to an object (continuous rotation, call each frame).
    /// Positive = counter-clockwise, negative = clockwise.
    pub fn apply_torque(&mut self, id: ObjectId, torque: f32, dt: f32) {
        if let Some(handle) = self.get_handle(id) {
            self.physics.apply_torque(handle, torque, dt);
        }
    }

    /// Apply an impulse to an object (instantaneous velocity change).
    pub fn apply_impulse(&mut self, id: ObjectId, impulse: Vec2) {
        if let Some(handle) = self.get_handle(id) {
            self.physics.apply_impulse(handle, impulse.x, impulse.y);
        }
    }

    /// Check if an object is touching the ground, a platform, or another body below it.
    pub fn is_grounded(&self, id: ObjectId) -> bool {
        self.with_handle(id, |h| self.physics.is_grounded(h, 0.5))
            .unwrap_or(false)
    }

    /// Check if two objects are touching (AABB overlap with threshold).
    pub fn is_touching(&self, a: ObjectId, b: ObjectId) -> bool {
        let (ha, hb) = match (self.get_handle(a), self.get_handle(b)) {
            (Some(ha), Some(hb)) => (ha, hb),
            _ => return false,
        };
        self.physics.are_overlapping(ha, hb, 0.3)
    }

    /// Get the first object in contact with the given object, if any.
    pub fn get_contact(&self, id: ObjectId) -> Option<ObjectId> {
        let handle = self.get_handle(id)?;
        let contact_handle = self.physics.get_contact(handle, 0.3)?;
        self.handle_map.get(&contact_handle).copied()
    }

    /// Get the velocity of an object.
    pub fn get_velocity(&self, id: ObjectId) -> Vec2 {
        self.with_handle(id, |h| self.physics.get_velocity(h).unwrap_or(Vec2::ZERO))
            .unwrap_or(Vec2::ZERO)
    }

    /// Set the velocity of an object.
    pub fn set_velocity(&mut self, id: ObjectId, vel: Vec2) {
        if let Some(handle) = self.get_handle(id) {
            self.physics.set_velocity(handle, vel.x, vel.y);
        }
    }

    // ── Sprite queries ──

    /// Get the position of a sprite object.
    pub fn get_sprite_position(&self, id: ObjectId) -> Option<Vec2> {
        match &self.entries.get(&id)?.kind {
            ObjectKind::Sprite { position, .. } => Some(*position),
            _ => None,
        }
    }

    /// Set the position of a sprite object.
    pub fn set_sprite_position(&mut self, id: ObjectId, pos: Vec2) {
        if let Some(entry) = self.entries.get_mut(&id) {
            if let ObjectKind::Sprite { position, .. } = &mut entry.kind {
                *position = pos;
            }
        }
    }

    /// Set the rotation of a sprite object (radians).
    pub fn set_sprite_rotation(&mut self, id: ObjectId, rot: f32) {
        if let Some(entry) = self.entries.get_mut(&id) {
            if let ObjectKind::Sprite { rotation, .. } = &mut entry.kind {
                *rotation = rot;
            }
        }
    }

    // ── Shadow config ──

    /// Set whether an object casts shadows.
    ///
    /// Default is `true` for rigid bodies and soft bodies, `false` for sprites.
    pub fn set_casts_shadow(&mut self, id: ObjectId, casts_shadow: bool) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.casts_shadow = casts_shadow;
        }
    }

    /// Check whether an object casts shadows.
    pub fn casts_shadow(&self, id: ObjectId) -> bool {
        self.entries.get(&id).map_or(false, |e| e.casts_shadow)
    }

    // ── Draw order ──

    /// Set the draw order for an object. Higher values draw later (on top).
    /// Default is 0. Objects with the same z-order have no guaranteed ordering.
    pub fn set_z_order(&mut self, id: ObjectId, z_order: i32) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.z_order = z_order;
        }
    }

    /// Get the draw order for an object.
    pub fn z_order(&self, id: ObjectId) -> i32 {
        self.entries.get(&id).map_or(0, |e| e.z_order)
    }

    /// Collect occluder geometry from all shadow-casting objects.
    ///
    /// Returns one [`Occluder`] per shadow-casting rigid body or soft body.
    /// The ground plane is handled separately by the lighting system via
    /// [`LightingSystem::set_ground_shadow`](unison_lighting::LightingSystem::set_ground_shadow).
    pub fn collect_occluders(&self) -> Vec<Occluder> {
        let mut occluders = Vec::new();

        for entry in self.entries.values() {
            if !entry.casts_shadow {
                continue;
            }

            match &entry.kind {
                ObjectKind::RigidBody { handle, .. } => {
                    if let Some(body) = self.physics.get_rigid_body(*handle) {
                        let he = body.collider.half_extents();
                        occluders.push(Occluder::from_aabb(
                            body.position.x,
                            body.position.y,
                            he.x,
                            he.y,
                        ));
                    }
                }
                ObjectKind::SoftBody { handle, boundary_edges, .. } => {
                    if let Some(boundary) = boundary_edges {
                        if let Some((positions, _)) = self.physics.get_body_render_data(*handle) {
                            occluders.push(Occluder::from_boundary_edges(&positions, boundary));
                        }
                    }
                }
                ObjectKind::Sprite { .. } => {
                    // Sprites don't have geometry to occlude
                }
            }
        }

        occluders
    }

    // ── Physics config ──

    /// Set the gravity vector (negative = downward).
    pub fn set_gravity(&mut self, gravity: Vec2) {
        self.physics.set_gravity(gravity.y);
    }

    /// Set a flat ground plane at the given Y position.
    pub fn set_ground(&mut self, y: f32) {
        self.physics.set_ground(Some(y));
    }

    /// Remove the ground plane.
    pub fn clear_ground(&mut self) {
        self.physics.set_ground(None);
    }

    /// Set ground friction (0.0 = ice, 1.0 = very sticky). Default: 0.8
    pub fn set_ground_friction(&mut self, friction: f32) {
        self.physics.set_ground_friction(friction);
    }

    /// Set ground bounciness (0.0 = no bounce, 1.0 = perfect bounce). Default: 0.3
    pub fn set_ground_restitution(&mut self, restitution: f32) {
        self.physics.set_ground_restitution(restitution);
    }

    // ── Raw access ──

    /// Direct access to the physics world for advanced operations.
    pub fn physics_mut(&mut self) -> &mut PhysicsWorld {
        &mut self.physics
    }

    /// Direct read access to the physics world.
    pub fn physics(&self) -> &PhysicsWorld {
        &self.physics
    }

    // ── Simulation ──

    /// Step the physics simulation by `dt` seconds.
    pub fn step(&mut self, dt: f32) {
        self.physics.step(dt);
    }

    /// Snapshot physics state for interpolated rendering.
    pub fn snapshot_for_render(&mut self) {
        self.physics.snapshot_for_render();
    }

    // ── Rendering ──

    /// Collect render commands for all objects, sorted by z-order.
    ///
    /// Higher z-order draws later (on top). Objects at the same z-order
    /// have no guaranteed relative ordering.
    pub fn render_commands(&self) -> Vec<RenderCommand> {
        let mut sorted: Vec<_> = self.entries.values().collect();
        sorted.sort_by_key(|e| e.z_order);

        let mut commands = Vec::new();
        for entry in sorted {
            match &entry.kind {
                ObjectKind::SoftBody { handle, color, texture, uvs, .. } => {
                    if let Some((positions, indices)) = self.physics.get_body_render_data(*handle) {
                        commands.push(RenderCommand::Mesh(DrawMesh {
                            positions,
                            uvs: uvs.clone(),
                            indices: indices.to_vec(),
                            texture: *texture,
                            color: *color,
                            vertex_colors: None,
                        }));
                    }
                }
                ObjectKind::RigidBody { handle, color } => {
                    if let Some(body) = self.physics.get_rigid_body(*handle) {
                        let he = body.collider.half_extents();
                        commands.push(RenderCommand::Rect {
                            position: [body.position.x - he.x, body.position.y - he.y],
                            size: [he.x * 2.0, he.y * 2.0],
                            color: *color,
                        });
                    }
                }
                ObjectKind::Sprite { texture, position, size, rotation, color } => {
                    commands.push(RenderCommand::Sprite(DrawSprite {
                        texture: *texture,
                        position: [position.x, position.y],
                        size: [size.x, size.y],
                        rotation: *rotation,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        color: *color,
                    }));
                }
            }
        }

        commands
    }

    // ── Info ──

    /// Number of objects currently alive.
    pub fn object_count(&self) -> usize {
        self.entries.len()
    }

    // ── Helpers ──

    fn next_object_id(&mut self) -> ObjectId {
        let id = ObjectId(self.next_id);
        self.next_id += 1;
        id
    }

    fn get_handle(&self, id: ObjectId) -> Option<BodyHandle> {
        self.entries.get(&id).and_then(|entry| entry.physics_handle())
    }

    fn with_handle<T, F: FnOnce(BodyHandle) -> T>(&self, id: ObjectId, f: F) -> Option<T> {
        self.get_handle(id).map(f)
    }
}

impl Default for ObjectSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unison_physics::mesh::create_ring_mesh;
    use unison_render::TextureId;

    #[test]
    fn spawn_and_query_soft_body() {
        let mut objects = ObjectSystem::new();
        let mesh = create_ring_mesh(1.0, 0.5, 8, 2);

        let id = objects.spawn_soft_body(SoftBodyDesc {
            mesh,
            material: unison_physics::Material::RUBBER,
            position: Vec2::new(0.0, 5.0),
            color: Color::WHITE,
            texture: TextureId::NONE,
        });

        let pos = objects.get_position(id);
        assert!((pos.x).abs() < 0.5);
        assert!((pos.y - 5.0).abs() < 0.5);
    }

    #[test]
    fn spawn_and_query_rigid_body() {
        let mut objects = ObjectSystem::new();

        let id = objects.spawn_rigid_body(RigidBodyDesc {
            collider: unison_physics::Collider::aabb(5.0, 0.5),
            position: Vec2::new(0.0, -3.0),
            color: Color::from_hex(0x2d5016),
            is_static: true,
        });

        let pos = objects.get_position(id);
        assert!((pos.x).abs() < 0.01);
        assert!((pos.y + 3.0).abs() < 0.01);
    }

    #[test]
    fn spawn_static_rect() {
        let mut objects = ObjectSystem::new();

        let id = objects.spawn_static_rect(
            Vec2::new(0.0, -5.0),
            Vec2::new(100.0, 2.0),
            Color::from_hex(0x2d5016),
        );

        let pos = objects.get_position(id);
        assert!((pos.y + 5.0).abs() < 0.01);
    }

    #[test]
    fn despawn_removes_object() {
        let mut objects = ObjectSystem::new();
        let mesh = create_ring_mesh(1.0, 0.5, 8, 2);

        let id = objects.spawn_soft_body(SoftBodyDesc {
            mesh,
            material: unison_physics::Material::RUBBER,
            position: Vec2::ZERO,
            color: Color::WHITE,
            texture: TextureId::NONE,
        });

        objects.despawn(id);
        let pos = objects.get_position(id);
        assert_eq!(pos, Vec2::ZERO);
    }

    #[test]
    fn object_count() {
        let mut objects = ObjectSystem::new();
        assert_eq!(objects.object_count(), 0);

        let mesh = create_ring_mesh(1.0, 0.5, 8, 2);
        let id = objects.spawn_soft_body(SoftBodyDesc {
            mesh,
            material: unison_physics::Material::RUBBER,
            position: Vec2::ZERO,
            color: Color::WHITE,
            texture: TextureId::NONE,
        });

        assert_eq!(objects.object_count(), 1);
        objects.despawn(id);
        assert_eq!(objects.object_count(), 0);
    }

    #[test]
    fn set_gravity_and_ground() {
        let mut objects = ObjectSystem::new();
        objects.set_gravity(Vec2::new(0.0, -20.0));
        objects.set_ground(-5.0);

        assert_eq!(objects.physics.gravity(), -20.0);
        assert_eq!(objects.physics.ground(), Some(-5.0));
    }

    #[test]
    fn render_commands_collected() {
        let mut objects = ObjectSystem::new();
        let mesh = create_ring_mesh(1.0, 0.5, 8, 2);

        objects.spawn_soft_body(SoftBodyDesc {
            mesh,
            material: unison_physics::Material::RUBBER,
            position: Vec2::ZERO,
            color: Color::WHITE,
            texture: TextureId::NONE,
        });

        objects.spawn_rigid_body(RigidBodyDesc {
            collider: unison_physics::Collider::aabb(1.0, 1.0),
            position: Vec2::new(5.0, 0.0),
            color: Color::RED,
            is_static: true,
        });

        let cmds = objects.render_commands();
        assert_eq!(cmds.len(), 2);
    }
}
