//! Engine — batteries-included game engine layer.
//!
//! Owns and orchestrates all subsystems (physics, rendering, input, lighting).
//! Game code interacts with `Engine` for a simple, discoverable API.
//! Raw subsystem access is available via escape hatches like `physics_mut()`.

use std::collections::HashMap;
use std::hash::Hash;

use unison_math::{Color, Vec2};
use unison_physics::{BodyHandle, PhysicsWorld};
use unison_render::{Camera, DrawMesh, RenderCommand, Renderer, TextureId};
use unison_lighting::LightingManager;
use unison_input::{ActionMap, InputState, KeyCode, MouseButton};

use crate::object::{ObjectEntry, ObjectId, ObjectKind, RigidBodyDesc, SoftBodyDesc};

/// The main engine struct. Owns all subsystems and provides a unified API.
///
/// Game code talks to `Engine` through high-level methods like `spawn_soft_body()`,
/// `apply_force()`, and `action_just_started()`. For advanced use cases, raw subsystem
/// access is available via `physics_mut()`, `renderer_mut()`, etc.
pub struct Engine<A: Copy + Eq + Hash> {
    // Subsystems — public so platform crates can access them.
    #[doc(hidden)]
    pub physics: PhysicsWorld,
    #[doc(hidden)]
    pub camera: Camera,
    #[doc(hidden)]
    pub lighting: LightingManager,
    #[doc(hidden)]
    pub input: InputState,
    #[doc(hidden)]
    pub actions: ActionMap<A>,

    // Renderer is set by the platform crate at startup.
    // Public so platform crates can set it.
    #[doc(hidden)]
    pub renderer: Option<Box<dyn Renderer<Error = String>>>,

    // Object registry
    objects: HashMap<ObjectId, ObjectEntry>,
    next_id: u64,

    // Handle → ObjectId reverse mapping for physics queries
    handle_to_object: HashMap<BodyHandle, ObjectId>,

    // Environment
    background_color: Color,

    // Camera follow
    follow_target: Option<ObjectId>,
    follow_smoothing: f32,

    // Fixed timestep delta (set by platform's game loop).
    #[doc(hidden)]
    pub fixed_dt: f32,
}

impl<A: Copy + Eq + Hash> Engine<A> {
    /// Create a new engine with default settings.
    /// The renderer is set later by the platform crate.
    pub fn new() -> Self {
        let mut physics = PhysicsWorld::new();
        physics.set_gravity(-9.8);

        Self {
            physics,
            camera: Camera::new(20.0, 15.0),
            lighting: LightingManager::new(),
            input: InputState::new(),
            actions: ActionMap::new(),
            renderer: None,
            objects: HashMap::new(),
            next_id: 0,
            handle_to_object: HashMap::new(),
            background_color: Color::BLACK,
            follow_target: None,
            follow_smoothing: 0.1,
            fixed_dt: 1.0 / 60.0,
        }
    }

    // ── Object spawning ──

    /// Spawn a soft body object. Returns an ObjectId for future reference.
    /// The object is automatically rendered each frame.
    pub fn spawn_soft_body(&mut self, desc: SoftBodyDesc) -> ObjectId {
        let uvs = desc.mesh.uvs_or_default();
        let config = desc.to_body_config();
        let handle = self.physics.add_body(&desc.mesh, config);
        let color = desc.color;

        let id = self.next_object_id();
        self.handle_to_object.insert(handle, id);
        self.objects.insert(id, ObjectEntry {
            kind: ObjectKind::SoftBody { handle, color, uvs },
        });
        id
    }

    /// Spawn a rigid body object. Returns an ObjectId for future reference.
    /// The object is automatically rendered each frame.
    pub fn spawn_rigid_body(&mut self, desc: RigidBodyDesc) -> ObjectId {
        let config = desc.to_rigid_body_config();
        let handle = self.physics.add_rigid_body(config);
        let color = desc.color;

        let id = self.next_object_id();
        self.handle_to_object.insert(handle, id);
        self.objects.insert(id, ObjectEntry {
            kind: ObjectKind::RigidBody { handle, color },
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

    /// Remove an object from the world.
    pub fn despawn(&mut self, id: ObjectId) {
        if let Some(entry) = self.objects.remove(&id) {
            let handle = match &entry.kind {
                ObjectKind::SoftBody { handle, .. } => *handle,
                ObjectKind::RigidBody { handle, .. } => *handle,
            };
            self.physics.remove_body(handle);
            self.handle_to_object.remove(&handle);
            if self.follow_target == Some(id) {
                self.follow_target = None;
            }
        }
    }

    // ── Object queries ──

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
    pub fn apply_torque(&mut self, id: ObjectId, torque: f32) {
        if let Some(handle) = self.get_handle(id) {
            self.physics.apply_torque(handle, torque, self.fixed_dt);
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

    // ── Camera ──

    /// Get a mutable reference to the camera for direct manipulation.
    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    /// Get an immutable reference to the camera.
    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    /// Make the camera follow an object with the given smoothing factor.
    /// Smoothing: 0.0 = no movement, 1.0 = instant snap. Typical: 0.05-0.2.
    pub fn camera_follow(&mut self, target: ObjectId, smoothing: f32) {
        self.follow_target = Some(target);
        self.follow_smoothing = smoothing;
    }

    /// Stop the camera from following any object.
    pub fn camera_unfollow(&mut self) {
        self.follow_target = None;
    }

    // ── Input / Actions ──

    /// Bind a keyboard key to an action.
    pub fn bind_key(&mut self, key: KeyCode, action: A) {
        self.actions.bind_key(key, action);
    }

    /// Bind a mouse button to an action.
    pub fn bind_mouse_button(&mut self, button: MouseButton, action: A) {
        self.actions.bind_mouse_button(button, action);
    }

    /// Is the action currently active? (any bound input is held)
    pub fn action_active(&self, action: A) -> bool {
        self.actions.is_action_active(action)
    }

    /// Did the action just start this frame?
    pub fn action_just_started(&self, action: A) -> bool {
        self.actions.is_action_just_started(action)
    }

    /// Did the action just end this frame?
    pub fn action_just_ended(&self, action: A) -> bool {
        self.actions.is_action_just_ended(action)
    }

    /// Get an axis value from two opposing actions (-1, 0, or +1).
    pub fn action_axis(&self, negative: A, positive: A) -> f32 {
        self.actions.axis_value(negative, positive)
    }

    // ── Environment ──

    /// Set the gravity vector (negative = downward).
    pub fn set_gravity(&mut self, gravity: Vec2) {
        self.physics.set_gravity(gravity.y);
    }

    /// Set the background clear color.
    pub fn set_background(&mut self, color: Color) {
        self.background_color = color;
    }

    /// Set a flat ground plane at the given Y position.
    pub fn set_ground(&mut self, y: f32) {
        self.physics.set_ground(Some(y));
    }

    /// Remove the ground plane.
    pub fn clear_ground(&mut self) {
        self.physics.set_ground(None);
    }

    /// Get the current fixed timestep delta (typically 1/60).
    pub fn dt(&self) -> f32 {
        self.fixed_dt
    }

    // ── Raw access (escape hatches) ──

    /// Direct access to the physics world for advanced operations.
    pub fn physics_mut(&mut self) -> &mut PhysicsWorld {
        &mut self.physics
    }

    /// Direct read access to the physics world.
    pub fn physics(&self) -> &PhysicsWorld {
        &self.physics
    }

    /// Direct access to the lighting manager.
    pub fn lighting_mut(&mut self) -> &mut LightingManager {
        &mut self.lighting
    }

    /// Direct read access to the lighting manager.
    pub fn lighting(&self) -> &LightingManager {
        &self.lighting
    }

    /// Direct access to raw input state.
    pub fn input_state(&self) -> &InputState {
        &self.input
    }

    /// Direct access to the action map for custom bindings.
    pub fn actions_mut(&mut self) -> &mut ActionMap<A> {
        &mut self.actions
    }

    // ── Internal: called by platform game loop ──

    /// Update the engine for one fixed timestep tick.
    /// Called by the platform's game loop before `Game::update()`.
    #[doc(hidden)]
    pub fn pre_update(&mut self) {
        self.actions.update(&self.input);
    }

    /// Step physics and update camera. Called after `Game::update()`.
    #[doc(hidden)]
    pub fn post_update(&mut self) {
        let dt = self.fixed_dt;

        // Step physics
        self.physics.step(dt);

        // Update camera follow
        if let Some(target) = self.follow_target {
            if let Some(pos) = self.with_handle(target, |h| {
                self.physics.get_position(h).unwrap_or(Vec2::ZERO)
            }) {
                self.camera.move_toward(pos.x, pos.y, self.follow_smoothing);
            }
        }
    }

    /// Render all objects. Called by the platform's game loop.
    #[doc(hidden)]
    pub fn auto_render(&mut self) {
        let renderer = match self.renderer.as_mut() {
            Some(r) => r,
            None => return,
        };

        renderer.begin_frame(&self.camera);
        renderer.clear(self.background_color);

        // Collect render commands from all objects
        let mut commands: Vec<RenderCommand> = Vec::new();

        for entry in self.objects.values() {
            match &entry.kind {
                ObjectKind::SoftBody { handle, color, uvs } => {
                    if let Some((positions, indices)) = self.physics.get_body_render_data(*handle) {
                        commands.push(RenderCommand::Mesh(DrawMesh {
                            positions: positions.to_vec(),
                            uvs: uvs.clone(),
                            indices: indices.to_vec(),
                            texture: TextureId::NONE,
                            color: *color,
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
            }
        }

        for cmd in commands {
            renderer.draw(cmd);
        }

        renderer.end_frame();
    }

    /// Snapshot physics state for interpolated rendering.
    #[doc(hidden)]
    pub fn snapshot_for_render(&mut self) {
        self.physics.snapshot_for_render();
    }

    // ── Helpers ──

    fn next_object_id(&mut self) -> ObjectId {
        let id = ObjectId(self.next_id);
        self.next_id += 1;
        id
    }

    fn get_handle(&self, id: ObjectId) -> Option<BodyHandle> {
        self.objects.get(&id).map(|entry| match &entry.kind {
            ObjectKind::SoftBody { handle, .. } => *handle,
            ObjectKind::RigidBody { handle, .. } => *handle,
        })
    }

    fn with_handle<T, F: FnOnce(BodyHandle) -> T>(&self, id: ObjectId, f: F) -> Option<T> {
        self.get_handle(id).map(f)
    }
}

impl<A: Copy + Eq + Hash> Default for Engine<A> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unison_physics::mesh::create_ring_mesh;

    #[derive(Copy, Clone, Eq, PartialEq, Hash)]
    enum TestAction {
        Jump,
        Left,
        Right,
    }

    #[test]
    fn spawn_and_query_soft_body() {
        let mut engine = Engine::<TestAction>::new();
        let mesh = create_ring_mesh(1.0, 0.5, 8, 2);

        let id = engine.spawn_soft_body(SoftBodyDesc {
            mesh,
            material: unison_physics::Material::RUBBER,
            position: Vec2::new(0.0, 5.0),
            color: Color::WHITE,
        });

        let pos = engine.get_position(id);
        assert!((pos.x).abs() < 0.5);
        assert!((pos.y - 5.0).abs() < 0.5);
    }

    #[test]
    fn spawn_and_query_rigid_body() {
        let mut engine = Engine::<TestAction>::new();

        let id = engine.spawn_rigid_body(RigidBodyDesc {
            collider: unison_physics::Collider::aabb(5.0, 0.5),
            position: Vec2::new(0.0, -3.0),
            color: Color::from_hex(0x2d5016),
            is_static: true,
        });

        let pos = engine.get_position(id);
        assert!((pos.x).abs() < 0.01);
        assert!((pos.y + 3.0).abs() < 0.01);
    }

    #[test]
    fn spawn_static_rect() {
        let mut engine = Engine::<TestAction>::new();

        let id = engine.spawn_static_rect(
            Vec2::new(0.0, -5.0),
            Vec2::new(100.0, 2.0),
            Color::from_hex(0x2d5016),
        );

        let pos = engine.get_position(id);
        assert!((pos.y + 5.0).abs() < 0.01);
    }

    #[test]
    fn despawn_removes_object() {
        let mut engine = Engine::<TestAction>::new();
        let mesh = create_ring_mesh(1.0, 0.5, 8, 2);

        let id = engine.spawn_soft_body(SoftBodyDesc {
            mesh,
            material: unison_physics::Material::RUBBER,
            position: Vec2::ZERO,
            color: Color::WHITE,
        });

        engine.despawn(id);

        // Position returns ZERO (default) for despawned objects
        let pos = engine.get_position(id);
        assert_eq!(pos, Vec2::ZERO);
    }

    #[test]
    fn action_binding_and_query() {
        let mut engine = Engine::<TestAction>::new();
        engine.bind_key(KeyCode::Space, TestAction::Jump);
        engine.bind_key(KeyCode::ArrowLeft, TestAction::Left);
        engine.bind_key(KeyCode::ArrowRight, TestAction::Right);

        // Simulate a key press
        engine.input.key_pressed(KeyCode::Space);
        engine.pre_update();

        assert!(engine.action_active(TestAction::Jump));
        assert!(engine.action_just_started(TestAction::Jump));
        assert!(!engine.action_active(TestAction::Left));
    }

    #[test]
    fn action_axis_works() {
        let mut engine = Engine::<TestAction>::new();
        engine.bind_key(KeyCode::ArrowLeft, TestAction::Left);
        engine.bind_key(KeyCode::ArrowRight, TestAction::Right);

        engine.input.key_pressed(KeyCode::ArrowRight);
        engine.pre_update();

        assert_eq!(engine.action_axis(TestAction::Left, TestAction::Right), 1.0);
    }

    #[test]
    fn camera_follow() {
        let mut engine = Engine::<TestAction>::new();
        let mesh = create_ring_mesh(1.0, 0.5, 8, 2);

        let id = engine.spawn_soft_body(SoftBodyDesc {
            mesh,
            material: unison_physics::Material::RUBBER,
            position: Vec2::new(10.0, 5.0),
            color: Color::WHITE,
        });

        engine.camera_follow(id, 1.0); // instant snap
        engine.post_update();

        // Camera should have moved toward the body
        assert!(engine.camera.x.abs() > 0.0 || engine.camera.y.abs() > 0.0);
    }

    #[test]
    fn set_gravity_and_ground() {
        let mut engine = Engine::<TestAction>::new();
        engine.set_gravity(Vec2::new(0.0, -20.0));
        engine.set_ground(-5.0);

        assert_eq!(engine.physics.gravity(), -20.0);
        assert_eq!(engine.physics.ground(), Some(-5.0));
    }
}
