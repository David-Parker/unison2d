//! Physics World - High-level API for soft and rigid body simulation
//!
//! Provides a clean interface for managing physics bodies, collision groups,
//! and simulation stepping. Supports both soft bodies (XPBD) and true rigid bodies.

use crate::mesh::Mesh;
use crate::rigid::{RigidBody, RigidBodyConfig};
use crate::xpbd::{XPBDSoftBody, CollisionSystem};
use unison_math::Vec2;
use unison_profiler::profile_scope;

/// Unique identifier for a physics body
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BodyHandle(pub(crate) usize);

impl BodyHandle {
    /// Get the raw index (for advanced usage)
    pub fn index(&self) -> usize {
        self.0
    }
}

/// Collision group flags - bodies only collide if their groups overlap
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollisionGroups {
    /// Groups this body belongs to
    pub membership: u32,
    /// Groups this body can collide with
    pub filter: u32,
}

impl Default for CollisionGroups {
    fn default() -> Self {
        Self {
            membership: 0xFFFF_FFFF,
            filter: 0xFFFF_FFFF,
        }
    }
}

impl CollisionGroups {
    /// No collisions with any body
    pub const NONE: Self = Self { membership: 0, filter: 0 };

    /// Collide with everything (default)
    pub const ALL: Self = Self { membership: 0xFFFF_FFFF, filter: 0xFFFF_FFFF };

    /// Create collision groups with specific membership and filter
    pub fn new(membership: u32, filter: u32) -> Self {
        Self { membership, filter }
    }

    /// Check if two collision groups can interact
    pub fn can_collide(&self, other: &Self) -> bool {
        (self.membership & other.filter) != 0 && (other.membership & self.filter) != 0
    }
}

/// Material properties for creating bodies
#[derive(Clone, Copy, Debug)]
pub struct Material {
    /// Density in kg/m² (affects mass)
    pub density: f32,
    /// Edge compliance (0 = rigid edges, higher = softer)
    pub edge_compliance: f32,
    /// Area compliance (0 = incompressible, higher = compressible)
    pub area_compliance: f32,
}

impl Default for Material {
    fn default() -> Self {
        Self::RUBBER
    }
}

impl Material {
    /// Ultra-soft, blobby material
    pub const SLIME: Self = Self {
        density: 800.0,
        edge_compliance: 1e-5,
        area_compliance: 1e-4,
    };

    /// Soft, jiggly material
    pub const JELLO: Self = Self {
        density: 1000.0,
        edge_compliance: 0.0,
        area_compliance: 1e-6,
    };

    /// Bouncy rubber
    pub const RUBBER: Self = Self {
        density: 1100.0,
        edge_compliance: 0.0,
        area_compliance: 1e-7,
    };

    /// Stiff material
    pub const WOOD: Self = Self {
        density: 600.0,
        edge_compliance: 0.0,
        area_compliance: 1e-8,
    };

    /// Nearly rigid material
    pub const METAL: Self = Self {
        density: 2000.0,
        edge_compliance: 0.0,
        area_compliance: 0.0,
    };

    /// Create custom material
    pub fn new(density: f32, edge_compliance: f32, area_compliance: f32) -> Self {
        Self { density, edge_compliance, area_compliance }
    }
}

/// Body configuration options for soft bodies
#[derive(Clone, Debug)]
pub struct BodyConfig {
    /// Material properties
    pub material: Material,
    /// Collision groups
    pub collision_groups: CollisionGroups,
    /// Initial position offset
    pub position: Vec2,
    /// Initial velocity
    pub velocity: Vec2,
}

impl Default for BodyConfig {
    fn default() -> Self {
        Self {
            material: Material::default(),
            collision_groups: CollisionGroups::default(),
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
        }
    }
}

impl BodyConfig {
    /// Create a new body config with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the material
    pub fn with_material(mut self, material: Material) -> Self {
        self.material = material;
        self
    }

    /// Set collision groups
    pub fn with_collision_groups(mut self, groups: CollisionGroups) -> Self {
        self.collision_groups = groups;
        self
    }

    /// Disable collisions for this body
    pub fn without_collisions(mut self) -> Self {
        self.collision_groups = CollisionGroups::NONE;
        self
    }

    /// Set initial position
    pub fn at_position(mut self, x: f32, y: f32) -> Self {
        self.position = Vec2::new(x, y);
        self
    }

    /// Set initial velocity
    pub fn with_velocity(mut self, vx: f32, vy: f32) -> Self {
        self.velocity = Vec2::new(vx, vy);
        self
    }
}

/// Internal body data for soft bodies
struct SoftBodyData {
    collision_groups: CollisionGroups,
    /// Original edge rest lengths (for compression effects)
    original_edge_lengths: Vec<f32>,
    /// Original area rest values (for compression effects)
    original_areas: Vec<f32>,
}

/// Internal body data for rigid bodies
struct RigidBodyData {
    collision_groups: CollisionGroups,
}

/// Enum to distinguish between soft and rigid bodies in unified storage
#[derive(Clone, Copy)]
enum BodyType {
    Soft(usize),  // Index into soft_bodies
    Rigid(usize), // Index into rigid_bodies
}

/// Physics world containing all bodies and simulation state
pub struct PhysicsWorld {
    // Soft body storage
    soft_bodies: Vec<XPBDSoftBody>,
    soft_body_data: Vec<SoftBodyData>,
    triangles: Vec<Vec<u32>>,
    prev_render_positions: Vec<Vec<f32>>,

    // Rigid body storage
    rigid_bodies: Vec<RigidBody>,
    rigid_body_data: Vec<RigidBodyData>,

    // Handle mapping: maps BodyHandle -> body type and index
    body_types: Vec<Option<BodyType>>,

    // Collision system (for soft-soft collisions)
    collision_system: CollisionSystem,

    // Simulation parameters
    gravity: f32,
    ground_y: Option<f32>,
    ground_friction: f32,
    ground_restitution: f32,
    substeps: u32,
    /// Constraint solver iterations before collision
    pre_collision_iters: u32,
    /// Constraint solver iterations after collision
    post_collision_iters: u32,

    // Handle tracking
    next_handle: usize,
}

impl PhysicsWorld {
    /// Create a new empty physics world
    pub fn new() -> Self {
        Self {
            soft_bodies: Vec::new(),
            soft_body_data: Vec::new(),
            triangles: Vec::new(),
            prev_render_positions: Vec::new(),
            rigid_bodies: Vec::new(),
            rigid_body_data: Vec::new(),
            body_types: Vec::new(),
            collision_system: CollisionSystem::new(0.15),
            gravity: -9.8,
            ground_y: None,
            ground_friction: 0.8,
            ground_restitution: 0.3,
            substeps: 4,
            pre_collision_iters: 3,
            post_collision_iters: 2,
            next_handle: 0,
        }
    }

    /// Set gravity (default: -9.8)
    pub fn set_gravity(&mut self, gravity: f32) {
        self.gravity = gravity;
    }

    /// Get current gravity
    pub fn gravity(&self) -> f32 {
        self.gravity
    }

    /// Set ground plane Y coordinate (None to disable)
    pub fn set_ground(&mut self, y: Option<f32>) {
        self.ground_y = y;
    }

    /// Get ground Y coordinate
    pub fn ground(&self) -> Option<f32> {
        self.ground_y
    }

    /// Set ground friction coefficient (default: 0.8)
    /// 0.0 = frictionless ice, 1.0 = very sticky
    pub fn set_ground_friction(&mut self, friction: f32) {
        self.ground_friction = friction.clamp(0.0, 1.0);
    }

    /// Get ground friction coefficient
    pub fn ground_friction(&self) -> f32 {
        self.ground_friction
    }

    /// Set ground restitution/bounciness (default: 0.3)
    /// 0.0 = no bounce, 1.0 = perfect bounce
    pub fn set_ground_restitution(&mut self, restitution: f32) {
        self.ground_restitution = restitution.clamp(0.0, 1.0);
    }

    /// Get ground restitution
    pub fn ground_restitution(&self) -> f32 {
        self.ground_restitution
    }

    /// Set number of substeps per step (default: 4)
    pub fn set_substeps(&mut self, substeps: u32) {
        self.substeps = substeps.max(1);
    }

    /// Set constraint solver iterations (default: pre=3, post=2)
    ///
    /// Pre-collision iterations stabilize shape before collision detection.
    /// Post-collision iterations restore shape after collision resolution.
    /// Total iterations per substep = pre + post per body.
    /// With 4 substeps and 3+2 iterations, that's 20 total per frame.
    pub fn set_solver_iterations(&mut self, pre_collision: u32, post_collision: u32) {
        self.pre_collision_iters = pre_collision.max(1);
        self.post_collision_iters = post_collision.max(1);
    }

    /// Add a soft body from a mesh with configuration
    pub fn add_body(&mut self, mesh: &Mesh, config: BodyConfig) -> BodyHandle {
        let mut vertices = mesh.vertices.clone();

        // Apply position offset
        for i in 0..vertices.len() / 2 {
            vertices[i * 2] += config.position.x;
            vertices[i * 2 + 1] += config.position.y;
        }

        let mut body = XPBDSoftBody::new(
            &vertices,
            &mesh.triangles,
            config.material.density,
            config.material.edge_compliance,
            config.material.area_compliance,
        );

        // Set initial velocity
        for i in 0..body.num_verts {
            body.vel[i * 2] = config.velocity.x;
            body.vel[i * 2 + 1] = config.velocity.y;
        }

        // Initialize prev_pos for correct first-frame velocity
        body.prev_pos = body.pos.clone();

        // Store original rest lengths/areas for later manipulation
        let original_edge_lengths: Vec<f32> = body.edge_constraints
            .iter()
            .map(|c| c.rest_length)
            .collect();
        let original_areas: Vec<f32> = body.area_constraints
            .iter()
            .map(|c| c.rest_area)
            .collect();

        let body_data = SoftBodyData {
            collision_groups: config.collision_groups,
            original_edge_lengths,
            original_areas,
        };

        // Allocate handle
        let handle = BodyHandle(self.next_handle);
        self.next_handle += 1;

        let soft_index = self.soft_bodies.len();

        // Extend body_types if needed
        while self.body_types.len() <= handle.0 {
            self.body_types.push(None);
        }
        self.body_types[handle.0] = Some(BodyType::Soft(soft_index));

        self.soft_bodies.push(body);
        self.soft_body_data.push(body_data);
        self.triangles.push(mesh.triangles.clone());
        self.prev_render_positions.push(vertices.clone());

        handle
    }

    /// Add a rigid body with configuration
    pub fn add_rigid_body(&mut self, config: RigidBodyConfig) -> BodyHandle {
        let body = RigidBody::new(&config);

        let body_data = RigidBodyData {
            collision_groups: CollisionGroups::default(),
        };

        // Allocate handle
        let handle = BodyHandle(self.next_handle);
        self.next_handle += 1;

        let rigid_index = self.rigid_bodies.len();

        // Extend body_types if needed
        while self.body_types.len() <= handle.0 {
            self.body_types.push(None);
        }
        self.body_types[handle.0] = Some(BodyType::Rigid(rigid_index));

        self.rigid_bodies.push(body);
        self.rigid_body_data.push(body_data);

        handle
    }

    /// Add a soft body with default configuration
    pub fn add_body_simple(&mut self, mesh: &Mesh, x: f32, y: f32) -> BodyHandle {
        self.add_body(mesh, BodyConfig::new().at_position(x, y))
    }

    /// Check if a body handle refers to a rigid body
    pub fn is_rigid(&self, handle: BodyHandle) -> bool {
        matches!(
            self.body_types.get(handle.0),
            Some(Some(BodyType::Rigid(_)))
        )
    }

    /// Remove a body from the world
    pub fn remove_body(&mut self, handle: BodyHandle) -> bool {
        let Some(body_type) = self.body_types.get(handle.0).cloned().flatten() else {
            return false;
        };

        match body_type {
            BodyType::Soft(index) => {
                // Remove from soft body vectors
                self.soft_bodies.remove(index);
                self.soft_body_data.remove(index);
                self.triangles.remove(index);
                self.prev_render_positions.remove(index);

                // Invalidate the handle
                self.body_types[handle.0] = None;

                // Update indices for all soft body handles after the removed one
                for bt in self.body_types.iter_mut().flatten() {
                    if let BodyType::Soft(ref mut idx) = bt {
                        if *idx > index {
                            *idx -= 1;
                        }
                    }
                }

                true
            }
            BodyType::Rigid(index) => {
                // Remove from rigid body vectors
                self.rigid_bodies.remove(index);
                self.rigid_body_data.remove(index);

                // Invalidate the handle
                self.body_types[handle.0] = None;

                // Update indices for all rigid body handles after the removed one
                for bt in self.body_types.iter_mut().flatten() {
                    if let BodyType::Rigid(ref mut idx) = bt {
                        if *idx > index {
                            *idx -= 1;
                        }
                    }
                }

                true
            }
        }
    }

    /// Check if a body handle is valid
    pub fn contains(&self, handle: BodyHandle) -> bool {
        self.body_types.get(handle.0).map(|t| t.is_some()).unwrap_or(false)
    }

    /// Get the number of bodies in the world (soft + rigid)
    pub fn body_count(&self) -> usize {
        self.soft_bodies.len() + self.rigid_bodies.len()
    }

    /// Get a soft body by handle (immutable)
    pub fn get_body(&self, handle: BodyHandle) -> Option<&XPBDSoftBody> {
        match self.body_types.get(handle.0)?.as_ref()? {
            BodyType::Soft(index) => self.soft_bodies.get(*index),
            BodyType::Rigid(_) => None,
        }
    }

    /// Get a soft body by handle (mutable)
    pub fn get_body_mut(&mut self, handle: BodyHandle) -> Option<&mut XPBDSoftBody> {
        match self.body_types.get(handle.0)?.as_ref()? {
            BodyType::Soft(index) => self.soft_bodies.get_mut(*index),
            BodyType::Rigid(_) => None,
        }
    }

    /// Get a rigid body by handle (immutable)
    pub fn get_rigid_body(&self, handle: BodyHandle) -> Option<&RigidBody> {
        match self.body_types.get(handle.0)?.as_ref()? {
            BodyType::Rigid(index) => self.rigid_bodies.get(*index),
            BodyType::Soft(_) => None,
        }
    }

    /// Get a rigid body by handle (mutable)
    pub fn get_rigid_body_mut(&mut self, handle: BodyHandle) -> Option<&mut RigidBody> {
        match self.body_types.get(handle.0)?.as_ref()? {
            BodyType::Rigid(index) => self.rigid_bodies.get_mut(*index),
            BodyType::Soft(_) => None,
        }
    }

    /// Get triangles for a soft body (for rendering)
    pub fn get_triangles(&self, handle: BodyHandle) -> Option<&[u32]> {
        match self.body_types.get(handle.0)?.as_ref()? {
            BodyType::Soft(index) => self.triangles.get(*index).map(|v| v.as_slice()),
            BodyType::Rigid(_) => None,
        }
    }

    /// Get collision groups for a body
    pub fn get_collision_groups(&self, handle: BodyHandle) -> Option<CollisionGroups> {
        match self.body_types.get(handle.0)?.as_ref()? {
            BodyType::Soft(index) => self.soft_body_data.get(*index).map(|d| d.collision_groups),
            BodyType::Rigid(index) => self.rigid_body_data.get(*index).map(|d| d.collision_groups),
        }
    }

    /// Set collision groups for a body
    pub fn set_collision_groups(&mut self, handle: BodyHandle, groups: CollisionGroups) -> bool {
        let Some(body_type) = self.body_types.get(handle.0).cloned().flatten() else {
            return false;
        };
        match body_type {
            BodyType::Soft(index) => {
                if let Some(data) = self.soft_body_data.get_mut(index) {
                    data.collision_groups = groups;
                    true
                } else {
                    false
                }
            }
            BodyType::Rigid(index) => {
                if let Some(data) = self.rigid_body_data.get_mut(index) {
                    data.collision_groups = groups;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Iterate over all valid body handles
    pub fn handles(&self) -> impl Iterator<Item = BodyHandle> + '_ {
        self.body_types
            .iter()
            .enumerate()
            .filter_map(|(i, bt)| bt.as_ref().map(|_| BodyHandle(i)))
    }

    /// Iterate over all soft bodies with their handles
    pub fn iter(&self) -> impl Iterator<Item = (BodyHandle, &XPBDSoftBody)> {
        self.body_types
            .iter()
            .enumerate()
            .filter_map(|(i, bt)| {
                if let Some(BodyType::Soft(index)) = bt {
                    Some((BodyHandle(i), &self.soft_bodies[*index]))
                } else {
                    None
                }
            })
    }

    /// Iterate over all soft bodies mutably with their handles
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (BodyHandle, &mut XPBDSoftBody)> {
        // Build a vec of (handle_id, soft_index) pairs first
        let pairs: Vec<(usize, usize)> = self.body_types
            .iter()
            .enumerate()
            .filter_map(|(i, bt)| {
                if let Some(BodyType::Soft(index)) = bt {
                    Some((i, *index))
                } else {
                    None
                }
            })
            .collect();

        // Create iterator over mutable references
        let soft_bodies = &mut self.soft_bodies;
        pairs.into_iter().map(move |(handle_id, soft_index)| {
            // SAFETY: We know each soft_index is unique because of how body_types works
            let body = unsafe { &mut *(&mut soft_bodies[soft_index] as *mut XPBDSoftBody) };
            (BodyHandle(handle_id), body)
        })
    }

    /// Iterate over all rigid bodies with their handles
    pub fn iter_rigid(&self) -> impl Iterator<Item = (BodyHandle, &RigidBody)> {
        self.body_types
            .iter()
            .enumerate()
            .filter_map(|(i, bt)| {
                if let Some(BodyType::Rigid(index)) = bt {
                    Some((BodyHandle(i), &self.rigid_bodies[*index]))
                } else {
                    None
                }
            })
    }

    // === Force/Impulse Application ===

    /// Apply a force to a body (in Newtons). Accumulated and applied during the next physics step.
    /// For soft bodies, applies uniformly to all vertices.
    /// For rigid bodies, applies at center of mass.
    pub fn apply_force(&mut self, handle: BodyHandle, fx: f32, fy: f32) {
        let Some(body_type) = self.body_types.get(handle.0).cloned().flatten() else { return };

        match body_type {
            BodyType::Soft(index) => {
                let body = &mut self.soft_bodies[index];
                for i in 0..body.num_verts {
                    if body.inv_mass[i] > 0.0 {
                        body.force_accum[i * 2] += fx;
                        body.force_accum[i * 2 + 1] += fy;
                    }
                }
            }
            BodyType::Rigid(index) => {
                let body = &mut self.rigid_bodies[index];
                body.apply_impulse(fx, fy);
            }
        }
    }

    /// Apply an impulse to a body (instantaneous velocity change)
    pub fn apply_impulse(&mut self, handle: BodyHandle, vx: f32, vy: f32) {
        let Some(body_type) = self.body_types.get(handle.0).cloned().flatten() else { return };

        match body_type {
            BodyType::Soft(index) => {
                let body = &mut self.soft_bodies[index];
                for i in 0..body.num_verts {
                    if body.inv_mass[i] > 0.0 {
                        body.vel[i * 2] += vx;
                        body.vel[i * 2 + 1] += vy;
                    }
                }
            }
            BodyType::Rigid(index) => {
                let body = &mut self.rigid_bodies[index];
                if body.inv_mass > 0.0 {
                    body.linear_velocity.x += vx;
                    body.linear_velocity.y += vy;
                }
            }
        }
    }

    /// Apply a force at the body's center of mass
    pub fn apply_central_force(&mut self, handle: BodyHandle, fx: f32, fy: f32) {
        self.apply_force(handle, fx, fy);
    }

    /// Apply an impulse at the body's center of mass
    pub fn apply_central_impulse(&mut self, handle: BodyHandle, vx: f32, vy: f32) {
        self.apply_impulse(handle, vx, vy);
    }

    /// Apply acceleration to a body (like gravity, doesn't depend on mass)
    pub fn apply_acceleration(&mut self, handle: BodyHandle, ax: f32, ay: f32, dt: f32) {
        let Some(body_type) = self.body_types.get(handle.0).cloned().flatten() else { return };

        match body_type {
            BodyType::Soft(index) => {
                let body = &mut self.soft_bodies[index];
                for i in 0..body.num_verts {
                    if body.inv_mass[i] > 0.0 {
                        body.vel[i * 2] += ax * dt;
                        body.vel[i * 2 + 1] += ay * dt;
                    }
                }
            }
            BodyType::Rigid(index) => {
                let body = &mut self.rigid_bodies[index];
                if body.inv_mass > 0.0 {
                    body.linear_velocity.x += ax * dt;
                    body.linear_velocity.y += ay * dt;
                }
            }
        }
    }

    /// Set velocity of a body
    pub fn set_velocity(&mut self, handle: BodyHandle, vx: f32, vy: f32) {
        let Some(body_type) = self.body_types.get(handle.0).cloned().flatten() else { return };

        match body_type {
            BodyType::Soft(index) => {
                let body = &mut self.soft_bodies[index];
                for i in 0..body.num_verts {
                    body.vel[i * 2] = vx;
                    body.vel[i * 2 + 1] = vy;
                }
            }
            BodyType::Rigid(index) => {
                let body = &mut self.rigid_bodies[index];
                body.linear_velocity.x = vx;
                body.linear_velocity.y = vy;
            }
        }
    }

    /// Get average velocity of body
    pub fn get_velocity(&self, handle: BodyHandle) -> Option<Vec2> {
        let body_type = self.body_types.get(handle.0)?.as_ref()?;

        match body_type {
            BodyType::Soft(index) => {
                let body = &self.soft_bodies[*index];
                let mut vx = 0.0;
                let mut vy = 0.0;
                let mut count = 0;

                for i in 0..body.num_verts {
                    if body.inv_mass[i] > 0.0 {
                        vx += body.vel[i * 2];
                        vy += body.vel[i * 2 + 1];
                        count += 1;
                    }
                }

                if count > 0 {
                    Some(Vec2::new(vx / count as f32, vy / count as f32))
                } else {
                    Some(Vec2::ZERO)
                }
            }
            BodyType::Rigid(index) => {
                let body = &self.rigid_bodies[*index];
                Some(body.linear_velocity)
            }
        }
    }

    /// Get angular velocity of the body
    /// Returns radians per second, positive = counter-clockwise
    pub fn get_angular_velocity(&self, handle: BodyHandle) -> Option<f32> {
        let body_type = self.body_types.get(handle.0)?.as_ref()?;

        match body_type {
            BodyType::Rigid(index) => {
                Some(self.rigid_bodies[*index].angular_velocity)
            }
            BodyType::Soft(index) => {
                let body = &self.soft_bodies[*index];

                // Get center of mass
                let mut cx = 0.0;
                let mut cy = 0.0;
                for i in 0..body.num_verts {
                    cx += body.pos[i * 2];
                    cy += body.pos[i * 2 + 1];
                }
                cx /= body.num_verts as f32;
                cy /= body.num_verts as f32;

                // Get average linear velocity
                let mut avg_vx = 0.0;
                let mut avg_vy = 0.0;
                let mut count = 0;
                for i in 0..body.num_verts {
                    if body.inv_mass[i] > 0.0 {
                        avg_vx += body.vel[i * 2];
                        avg_vy += body.vel[i * 2 + 1];
                        count += 1;
                    }
                }
                if count == 0 { return Some(0.0); }
                avg_vx /= count as f32;
                avg_vy /= count as f32;

                // Calculate angular velocity from tangential components
                let mut omega_sum = 0.0;
                let mut weight_sum = 0.0;
                for i in 0..body.num_verts {
                    if body.inv_mass[i] == 0.0 { continue; }

                    let rx = body.pos[i * 2] - cx;
                    let ry = body.pos[i * 2 + 1] - cy;
                    let r_sq = rx * rx + ry * ry;

                    if r_sq < 1e-10 { continue; }

                    let rel_vx = body.vel[i * 2] - avg_vx;
                    let rel_vy = body.vel[i * 2 + 1] - avg_vy;

                    let omega_i = (rx * rel_vy - ry * rel_vx) / r_sq;
                    omega_sum += omega_i;
                    weight_sum += 1.0;
                }

                if weight_sum > 0.0 {
                    Some(omega_sum / weight_sum)
                } else {
                    Some(0.0)
                }
            }
        }
    }

    /// Set linear velocity while preserving angular velocity
    pub fn set_linear_velocity(&mut self, handle: BodyHandle, vx: f32, vy: f32) {
        let Some(body_type) = self.body_types.get(handle.0).cloned().flatten() else { return };

        match body_type {
            BodyType::Soft(index) => {
                let body = &mut self.soft_bodies[index];

                // Get current average velocity
                let mut avg_vx = 0.0;
                let mut avg_vy = 0.0;
                let mut count = 0;
                for i in 0..body.num_verts {
                    if body.inv_mass[i] > 0.0 {
                        avg_vx += body.vel[i * 2];
                        avg_vy += body.vel[i * 2 + 1];
                        count += 1;
                    }
                }
                if count == 0 { return; }
                avg_vx /= count as f32;
                avg_vy /= count as f32;

                let dvx = vx - avg_vx;
                let dvy = vy - avg_vy;

                for i in 0..body.num_verts {
                    if body.inv_mass[i] > 0.0 {
                        body.vel[i * 2] += dvx;
                        body.vel[i * 2 + 1] += dvy;
                    }
                }
            }
            BodyType::Rigid(index) => {
                let body = &mut self.rigid_bodies[index];
                body.linear_velocity.x = vx;
                body.linear_velocity.y = vy;
            }
        }
    }

    /// Apply angular velocity (rotation) to the body
    /// Positive = counter-clockwise, negative = clockwise
    pub fn apply_angular_velocity(&mut self, handle: BodyHandle, omega: f32) {
        let Some(body_type) = self.body_types.get(handle.0).cloned().flatten() else { return };

        match body_type {
            BodyType::Soft(index) => {
                let body = &mut self.soft_bodies[index];

                // Get center of mass
                let mut cx = 0.0;
                let mut cy = 0.0;
                for i in 0..body.num_verts {
                    cx += body.pos[i * 2];
                    cy += body.pos[i * 2 + 1];
                }
                cx /= body.num_verts as f32;
                cy /= body.num_verts as f32;

                // Apply tangential velocity to each vertex
                for i in 0..body.num_verts {
                    if body.inv_mass[i] > 0.0 {
                        let rx = body.pos[i * 2] - cx;
                        let ry = body.pos[i * 2 + 1] - cy;
                        body.vel[i * 2] += -ry * omega;
                        body.vel[i * 2 + 1] += rx * omega;
                    }
                }
            }
            BodyType::Rigid(index) => {
                let body = &mut self.rigid_bodies[index];
                body.angular_velocity += omega;
            }
        }
    }

    /// Apply torque (angular acceleration) to the body. Accumulated and applied during the next physics step.
    /// Positive = counter-clockwise, negative = clockwise.
    pub fn apply_torque(&mut self, handle: BodyHandle, torque: f32, dt: f32) {
        let Some(body_type) = self.body_types.get(handle.0).cloned().flatten() else { return };

        match body_type {
            BodyType::Soft(index) => {
                self.soft_bodies[index].torque_accum += torque;
            }
            BodyType::Rigid(index) => {
                let body = &mut self.rigid_bodies[index];
                body.apply_angular_impulse(torque * dt);
            }
        }
    }

    // === Position/Transform ===

    /// Get the center of mass position
    pub fn get_position(&self, handle: BodyHandle) -> Option<Vec2> {
        let body_type = self.body_types.get(handle.0)?.as_ref()?;

        match body_type {
            BodyType::Soft(index) => {
                let (x, y) = self.soft_bodies[*index].get_center();
                Some(Vec2::new(x, y))
            }
            BodyType::Rigid(index) => Some(self.rigid_bodies[*index].get_center()),
        }
    }

    /// Translate a body by an offset
    pub fn translate(&mut self, handle: BodyHandle, dx: f32, dy: f32) {
        let Some(body_type) = self.body_types.get(handle.0).cloned().flatten() else { return };

        match body_type {
            BodyType::Soft(index) => {
                let body = &mut self.soft_bodies[index];
                for i in 0..body.num_verts {
                    body.pos[i * 2] += dx;
                    body.pos[i * 2 + 1] += dy;
                    body.prev_pos[i * 2] += dx;
                    body.prev_pos[i * 2 + 1] += dy;
                }
            }
            BodyType::Rigid(index) => {
                let body = &mut self.rigid_bodies[index];
                body.position.x += dx;
                body.position.y += dy;
                body.prev_position.x += dx;
                body.prev_position.y += dy;
            }
        }
    }

    /// Set the center position (translates whole body)
    pub fn set_position(&mut self, handle: BodyHandle, x: f32, y: f32) {
        let Some(current) = self.get_position(handle) else { return };
        let dx = x - current.x;
        let dy = y - current.y;
        self.translate(handle, dx, dy);
    }

    // === Deformation Control (soft bodies only) ===

    /// Compress a soft body vertically (ratio 0.0-1.0, where 1.0 = no compression)
    /// Useful for squash-and-stretch effects. Has no effect on rigid bodies.
    pub fn set_vertical_compression(&mut self, handle: BodyHandle, ratio: f32) {
        self.set_squash(handle, ratio, 1.0);
    }

    /// Apply squash-and-stretch deformation to a soft body
    /// Has no effect on rigid bodies.
    pub fn set_squash(&mut self, handle: BodyHandle, vertical_ratio: f32, horizontal_ratio: f32) {
        let Some(BodyType::Soft(index)) = self.body_types.get(handle.0).cloned().flatten() else {
            return;
        };

        let body = &mut self.soft_bodies[index];
        let data = &self.soft_body_data[index];

        for (i, constraint) in body.edge_constraints.iter_mut().enumerate() {
            let v0 = constraint.v0;
            let v1 = constraint.v1;

            let y0 = body.pos[v0 * 2 + 1];
            let y1 = body.pos[v1 * 2 + 1];
            let x0 = body.pos[v0 * 2];
            let x1 = body.pos[v1 * 2];

            let dy = (y1 - y0).abs();
            let dx = (x1 - x0).abs();

            let original_len = data.original_edge_lengths[i];

            if dy > dx * 2.0 {
                constraint.rest_length = original_len * vertical_ratio;
            } else if dx > dy * 2.0 {
                constraint.rest_length = original_len * horizontal_ratio;
            } else {
                let angle_factor = dy / (dx + dy + 0.001);
                let ratio = vertical_ratio * angle_factor + horizontal_ratio * (1.0 - angle_factor);
                constraint.rest_length = original_len * ratio;
            }
        }
    }

    /// Reset all rest lengths to original values (soft bodies only)
    pub fn reset_rest_lengths(&mut self, handle: BodyHandle) {
        let Some(BodyType::Soft(index)) = self.body_types.get(handle.0).cloned().flatten() else {
            return;
        };

        let body = &mut self.soft_bodies[index];
        let data = &self.soft_body_data[index];

        for (i, constraint) in body.edge_constraints.iter_mut().enumerate() {
            constraint.rest_length = data.original_edge_lengths[i];
        }

        for (i, constraint) in body.area_constraints.iter_mut().enumerate() {
            constraint.rest_area = data.original_areas[i];
        }
    }

    // === Queries ===

    /// Get the lowest Y coordinate of a body (for ground detection)
    pub fn get_lowest_y(&self, handle: BodyHandle) -> Option<f32> {
        let body_type = self.body_types.get(handle.0)?.as_ref()?;

        match body_type {
            BodyType::Soft(index) => Some(self.soft_bodies[*index].get_lowest_y()),
            BodyType::Rigid(index) => {
                let body = &self.rigid_bodies[*index];
                let (_, min_y, _, _) = body.get_aabb();
                Some(min_y)
            }
        }
    }

    /// Check if body is touching the ground, a platform, or another body below it.
    /// For soft bodies, also checks that the body is not moving upward significantly
    /// (to avoid false positives mid-jump).
    pub fn is_grounded(&self, handle: BodyHandle, threshold: f32) -> bool {
        // Use surface contact detection which checks ground + all other bodies
        if self.get_surface_contact_y(handle, threshold).is_none() {
            return false;
        }

        // For soft bodies, also check vertical velocity to avoid false positives.
        // If the body is moving upward significantly, it's not grounded (just launched).
        let body_type = self.body_types.get(handle.0).and_then(|bt| bt.as_ref());
        if let Some(BodyType::Soft(index)) = body_type {
            let body = &self.soft_bodies[*index];
            // Average vertical velocity of all dynamic vertices
            let mut vy_sum = 0.0f32;
            let mut count = 0;
            for i in 0..body.num_verts {
                if body.inv_mass[i] > 0.0 {
                    vy_sum += body.vel[i * 2 + 1];
                    count += 1;
                }
            }
            if count > 0 {
                let avg_vy = vy_sum / count as f32;
                // If moving upward faster than a small threshold, not grounded
                if avg_vy > 2.0 {
                    return false;
                }
            }
        }

        true
    }

    /// Check if a body is in contact with any other body (within threshold distance)
    /// Returns the handle of the first body in contact, if any
    pub fn get_contact(&self, handle: BodyHandle, threshold: f32) -> Option<BodyHandle> {
        let body_type = self.body_types.get(handle.0)?.as_ref()?;

        let aabb = match body_type {
            BodyType::Soft(index) => self.soft_bodies[*index].get_aabb(),
            BodyType::Rigid(index) => self.rigid_bodies[*index].get_aabb(),
        };
        let (min_x, min_y, max_x, max_y) = aabb;

        // Check against all other bodies
        for (i, bt) in self.body_types.iter().enumerate() {
            let Some(other_type) = bt else { continue };
            if i == handle.0 { continue; }

            let other_aabb = match other_type {
                BodyType::Soft(idx) => self.soft_bodies[*idx].get_aabb(),
                BodyType::Rigid(idx) => self.rigid_bodies[*idx].get_aabb(),
            };
            let (o_min_x, o_min_y, o_max_x, o_max_y) = other_aabb;

            if max_x + threshold >= o_min_x && min_x - threshold <= o_max_x &&
               max_y + threshold >= o_min_y && min_y - threshold <= o_max_y {
                return Some(BodyHandle(i));
            }
        }

        None
    }

    /// Check if body is contacting ground or any other body below it
    /// Returns the Y position of the contact surface, if any
    pub fn get_surface_contact_y(&self, handle: BodyHandle, threshold: f32) -> Option<f32> {
        let body_type = self.body_types.get(handle.0)?.as_ref()?;

        let (lowest_y, min_x, max_x) = match body_type {
            BodyType::Soft(index) => {
                let body = &self.soft_bodies[*index];
                let (bmin_x, _, bmax_x, _) = body.get_aabb();
                (body.get_lowest_y(), bmin_x, bmax_x)
            }
            BodyType::Rigid(index) => {
                let body = &self.rigid_bodies[*index];
                let (bmin_x, bmin_y, bmax_x, _) = body.get_aabb();
                (bmin_y, bmin_x, bmax_x)
            }
        };

        let mut best_surface_y = None;

        // Check ground
        if let Some(ground_y) = self.ground_y {
            if lowest_y < ground_y + threshold {
                best_surface_y = Some(ground_y);
            }
        }

        // Check other bodies
        for (i, bt) in self.body_types.iter().enumerate() {
            let Some(other_type) = bt else { continue };
            if i == handle.0 { continue; }

            let (o_min_x, _, o_max_x, o_max_y) = match other_type {
                BodyType::Soft(idx) => self.soft_bodies[*idx].get_aabb(),
                BodyType::Rigid(idx) => self.rigid_bodies[*idx].get_aabb(),
            };

            if max_x >= o_min_x && min_x <= o_max_x {
                if lowest_y < o_max_y + threshold && lowest_y > o_max_y - threshold * 2.0 {
                    let surface_y = o_max_y;
                    if best_surface_y.map_or(true, |y| surface_y > y) {
                        best_surface_y = Some(surface_y);
                    }
                }
            }
        }

        best_surface_y
    }

    /// Get kinetic energy of a body
    pub fn get_kinetic_energy(&self, handle: BodyHandle) -> Option<f32> {
        let body_type = self.body_types.get(handle.0)?.as_ref()?;

        match body_type {
            BodyType::Soft(index) => Some(self.soft_bodies[*index].get_kinetic_energy()),
            BodyType::Rigid(index) => {
                let body = &self.rigid_bodies[*index];
                if body.inv_mass > 0.0 {
                    let mass = 1.0 / body.inv_mass;
                    let v_sq = body.linear_velocity.x.powi(2) + body.linear_velocity.y.powi(2);
                    Some(0.5 * mass * v_sq)
                } else {
                    Some(0.0)
                }
            }
        }
    }

    /// Get total kinetic energy of all bodies
    pub fn total_kinetic_energy(&self) -> f32 {
        let soft_ke: f32 = self.soft_bodies.iter().map(|b| b.get_kinetic_energy()).sum();
        let rigid_ke: f32 = self.rigid_bodies.iter().map(|b| {
            if b.inv_mass > 0.0 {
                let mass = 1.0 / b.inv_mass;
                let v_sq = b.linear_velocity.x.powi(2) + b.linear_velocity.y.powi(2);
                0.5 * mass * v_sq
            } else {
                0.0
            }
        }).sum();
        soft_ke + rigid_ke
    }

    /// Get AABB (min_x, min_y, max_x, max_y) for a body
    pub fn get_aabb(&self, handle: BodyHandle) -> Option<(f32, f32, f32, f32)> {
        let body_type = self.body_types.get(handle.0)?.as_ref()?;

        match body_type {
            BodyType::Soft(index) => Some(self.soft_bodies[*index].get_aabb()),
            BodyType::Rigid(index) => Some(self.rigid_bodies[*index].get_aabb()),
        }
    }

    // === Simulation ===

    /// Snapshot current positions for render interpolation.
    /// Call this before stepping physics to enable smooth rendering.
    pub fn snapshot_for_render(&mut self) {
        for (i, body) in self.soft_bodies.iter().enumerate() {
            if i < self.prev_render_positions.len() {
                self.prev_render_positions[i].copy_from_slice(&body.pos);
            }
        }
    }

    /// Step the simulation forward by dt seconds
    pub fn step(&mut self, dt: f32) {
        if self.soft_bodies.is_empty() && self.rigid_bodies.is_empty() {
            return;
        }

        let substep_dt = dt / self.substeps as f32;

        // Prepare collision system for soft-soft collisions
        self.collision_system.prepare(&self.soft_bodies);

        for _ in 0..self.substeps {
            // Pre-solve soft bodies (includes pre-collision constraint iterations)
            for body in self.soft_bodies.iter_mut() {
                body.substep_pre_with_friction_iters(
                    substep_dt,
                    self.gravity,
                    self.ground_y,
                    self.ground_friction,
                    self.ground_restitution,
                    self.pre_collision_iters,
                    self.post_collision_iters,
                );
            }

            // Pre-solve rigid bodies
            for body in self.rigid_bodies.iter_mut() {
                body.pre_solve(substep_dt, self.gravity);
                if let Some(ground_y) = self.ground_y {
                    body.solve_ground_collision(ground_y, self.ground_friction, self.ground_restitution);
                }
            }

            // Resolve soft-soft collisions
            self.collision_system.resolve_collisions(&mut self.soft_bodies);

            // Resolve soft-vs-rigid collisions
            self.resolve_soft_rigid_collisions();

            // Note: constraint solving already done in substep_pre_with_friction_iters

            // Post-solve all bodies
            for body in self.soft_bodies.iter_mut() {
                body.substep_post(substep_dt);
            }
            for body in self.rigid_bodies.iter_mut() {
                body.post_solve(substep_dt);
            }
        }

        // Damping applied once per step (not per substep).
        // Stiff materials get internal-only damping to kill oscillation without
        // affecting fall speed. Soft materials get light global damping.
        for body in self.soft_bodies.iter_mut() {
            let compliance = body.edge_compliance + body.area_compliance;
            if compliance < 1e-7 {
                body.apply_internal_damping(0.08);
            } else {
                body.apply_damping(0.005);
            }
        }

        // Clear force/torque accumulators AFTER all substeps so forces apply uniformly
        for body in self.soft_bodies.iter_mut() {
            body.clear_accumulators();
        }
    }

    /// Step the simulation with terrain collision (variable height ground)
    pub fn step_with_terrain<F, G>(&mut self, dt: f32, height_at: F, normal_at: G)
    where
        F: Fn(f32) -> f32,
        G: Fn(f32) -> (f32, f32),
    {
        profile_scope!("physics.step_with_terrain");

        if self.soft_bodies.is_empty() && self.rigid_bodies.is_empty() {
            return;
        }

        let substep_dt = dt / self.substeps as f32;

        // Prepare collision system
        {
            profile_scope!("physics.collision_prepare");
            self.collision_system.prepare(&self.soft_bodies);
        }

        for _ in 0..self.substeps {
            // Pre-solve soft bodies with terrain
            {
                profile_scope!("physics.pre_solve_terrain");
                for body in self.soft_bodies.iter_mut() {
                    body.substep_pre_with_terrain_iters(
                        substep_dt,
                        self.gravity,
                        &height_at,
                        &normal_at,
                        self.ground_friction,
                        self.ground_restitution,
                        self.pre_collision_iters,
                        self.post_collision_iters,
                    );
                }
            }

            // Pre-solve rigid bodies (terrain not yet supported for rigid)
            for body in self.rigid_bodies.iter_mut() {
                body.pre_solve(substep_dt, self.gravity);
                // Use height_at for simple ground approximation
                let center = body.get_center();
                let cx = center.x;
                let ground_y = height_at(cx);
                body.solve_ground_collision(ground_y, self.ground_friction, self.ground_restitution);
            }

            // Resolve soft-soft collisions
            {
                profile_scope!("physics.soft_collisions");
                self.collision_system.resolve_collisions(&mut self.soft_bodies);
            }

            // Resolve soft-vs-rigid collisions
            {
                profile_scope!("physics.soft_rigid_collisions");
                self.resolve_soft_rigid_collisions();
            }

            // Note: constraint solving is already done in substep_pre_with_terrain_iters
            // (pre_collision_iters before terrain, post_collision_iters after).

            // Post-solve all bodies
            {
                profile_scope!("physics.post_solve");
                for body in self.soft_bodies.iter_mut() {
                    body.substep_post(substep_dt);
                }
                for body in self.rigid_bodies.iter_mut() {
                    body.post_solve(substep_dt);
                }
            }
        }

        // Damping applied once per step (not per substep).
        // Stiff materials get internal-only damping to kill oscillation without
        // affecting fall speed. Soft materials get light global damping.
        for body in self.soft_bodies.iter_mut() {
            let compliance = body.edge_compliance + body.area_compliance;
            if compliance < 1e-7 {
                body.apply_internal_damping(0.08);
            } else {
                body.apply_damping(0.005);
            }
        }

        // Clear force/torque accumulators AFTER all substeps so forces apply uniformly
        for body in self.soft_bodies.iter_mut() {
            body.clear_accumulators();
        }
    }

    /// Internal: resolve collisions between soft body vertices and rigid bodies
    fn resolve_soft_rigid_collisions(&mut self) {
        let min_dist = 0.15; // Same as collision system
        let contact_threshold = 0.05; // Near-contact zone for rolling friction

        for soft_body in self.soft_bodies.iter_mut() {
            for rigid_body in self.rigid_bodies.iter_mut() {
                // Broad phase: check AABB overlap (expanded by contact_threshold for near-contact friction)
                let soft_aabb = soft_body.get_aabb();
                let rigid_aabb = rigid_body.get_aabb();
                let margin = min_dist + contact_threshold;

                if soft_aabb.2 + margin < rigid_aabb.0 || rigid_aabb.2 + margin < soft_aabb.0 ||
                   soft_aabb.3 + margin < rigid_aabb.1 || rigid_aabb.3 + margin < soft_aabb.1 {
                    continue;
                }

                let friction = rigid_body.friction;

                // Narrow phase: check each soft body vertex against rigid collider
                for i in 0..soft_body.num_verts {
                    if soft_body.inv_mass[i] == 0.0 {
                        continue;
                    }

                    let vx = soft_body.pos[i * 2];
                    let vy = soft_body.pos[i * 2 + 1];

                    if let Some((penetration, nx, ny)) = rigid_body.contains_point(vx, vy) {
                        // Push soft body vertex out of rigid body
                        let soft_w = soft_body.inv_mass[i];
                        let rigid_w = rigid_body.inv_mass;
                        let total_w = soft_w + rigid_w;

                        if total_w < 1e-10 {
                            // Rigid is kinematic, push soft body entirely
                            soft_body.pos[i * 2] += nx * penetration;
                            soft_body.pos[i * 2 + 1] += ny * penetration;
                        } else {
                            // Distribute push based on inverse mass
                            let soft_push = penetration * (soft_w / total_w);
                            let rigid_push = penetration * (rigid_w / total_w);

                            soft_body.pos[i * 2] += nx * soft_push;
                            soft_body.pos[i * 2 + 1] += ny * soft_push;

                            rigid_body.position.x -= nx * rigid_push;
                            rigid_body.position.y -= ny * rigid_push;

                            // Apply torque to rigid body based on contact point
                            let rx = vx - rigid_body.position.x;
                            let ry = vy - rigid_body.position.y;
                            let torque = (rx * (-ny * rigid_push) - ry * (-nx * rigid_push)) * rigid_body.inv_inertia;
                            rigid_body.angular_velocity += torque;
                        }

                        // Coulomb friction: damp tangential displacement at the contact.
                        // Same model as ground friction — decompose displacement into
                        // normal and tangential components, then reduce the tangent part.
                        let prev_x = soft_body.prev_pos[i * 2];
                        let prev_y = soft_body.prev_pos[i * 2 + 1];
                        let dx = soft_body.pos[i * 2] - prev_x;
                        let dy = soft_body.pos[i * 2 + 1] - prev_y;

                        // Project displacement onto contact normal
                        let vel_normal = dx * nx + dy * ny;
                        // Tangential component = total - normal projection
                        let tan_x = dx - vel_normal * nx;
                        let tan_y = dy - vel_normal * ny;

                        let friction_factor = 1.0 - friction;
                        soft_body.pos[i * 2] = prev_x + vel_normal * nx + tan_x * friction_factor;
                        soft_body.pos[i * 2 + 1] = prev_y + vel_normal * ny + tan_y * friction_factor;
                    } else {
                        // Near-contact friction: check if vertex is just outside the rigid body.
                        // This gives rolling friction for vertices close to the surface.
                        let near = rigid_body.nearest_surface_dist(vx, vy);
                        if let Some((dist, nx, ny)) = near {
                            if dist < contact_threshold {
                                let prev_x = soft_body.prev_pos[i * 2];
                                let prev_y = soft_body.prev_pos[i * 2 + 1];
                                let dx = soft_body.pos[i * 2] - prev_x;
                                let dy = soft_body.pos[i * 2 + 1] - prev_y;

                                let vel_normal = dx * nx + dy * ny;
                                let tan_x = dx - vel_normal * nx;
                                let tan_y = dy - vel_normal * ny;

                                let friction_factor = 1.0 - friction * 0.3;
                                soft_body.pos[i * 2] = prev_x + vel_normal * nx + tan_x * friction_factor;
                                soft_body.pos[i * 2 + 1] = prev_y + vel_normal * ny + tan_y * friction_factor;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Try to put body to sleep if resting (soft bodies only)
    pub fn sleep_if_resting(&mut self, handle: BodyHandle, threshold: f32) -> bool {
        let Some(body) = self.get_body_mut(handle) else { return false };
        body.sleep_if_resting(threshold)
    }

    // === Rendering Helpers ===

    /// Get all soft body positions and triangles for rendering
    pub fn get_render_data(&self) -> Vec<(&[f32], &[u32])> {
        self.soft_bodies.iter()
            .zip(self.triangles.iter())
            .map(|(body, tris)| (body.pos.as_slice(), tris.as_slice()))
            .collect()
    }

    /// Get render data for a specific soft body
    pub fn get_body_render_data(&self, handle: BodyHandle) -> Option<(&[f32], &[u32])> {
        let BodyType::Soft(index) = self.body_types.get(handle.0)?.as_ref()? else {
            return None;
        };
        let body = self.soft_bodies.get(*index)?;
        let tris = self.triangles.get(*index)?;
        Some((body.pos.as_slice(), tris.as_slice()))
    }

    /// Get interpolated render data for smoother rendering (soft bodies only).
    /// `alpha` is the interpolation factor (0.0 = previous frame, 1.0 = current frame).
    pub fn get_body_render_data_interpolated(&self, handle: BodyHandle, alpha: f32) -> Option<(Vec<f32>, &[u32])> {
        let BodyType::Soft(index) = self.body_types.get(handle.0)?.as_ref()? else {
            return None;
        };
        let body = self.soft_bodies.get(*index)?;
        let prev = self.prev_render_positions.get(*index)?;
        let tris = self.triangles.get(*index)?;

        let alpha = alpha.clamp(0.0, 1.0);
        let one_minus_alpha = 1.0 - alpha;
        let interpolated: Vec<f32> = body.pos.iter()
            .zip(prev.iter())
            .map(|(&curr, &prev)| prev * one_minus_alpha + curr * alpha)
            .collect();

        Some((interpolated, tris.as_slice()))
    }

    /// Get interpolated center position for a body.
    /// Useful for camera tracking without jitter.
    pub fn get_position_interpolated(&self, handle: BodyHandle, alpha: f32) -> Option<Vec2> {
        let body_type = self.body_types.get(handle.0)?.as_ref()?;

        let alpha = alpha.clamp(0.0, 1.0);
        let one_minus_alpha = 1.0 - alpha;

        match body_type {
            BodyType::Soft(index) => {
                let body = self.soft_bodies.get(*index)?;
                let prev = self.prev_render_positions.get(*index)?;

                let mut cx = 0.0;
                let mut cy = 0.0;
                let n = body.num_verts;

                for i in 0..n {
                    let curr_x = body.pos[i * 2];
                    let curr_y = body.pos[i * 2 + 1];
                    let prev_x = prev[i * 2];
                    let prev_y = prev[i * 2 + 1];
                    cx += prev_x * one_minus_alpha + curr_x * alpha;
                    cy += prev_y * one_minus_alpha + curr_y * alpha;
                }

                Some(Vec2::new(cx / n as f32, cy / n as f32))
            }
            BodyType::Rigid(index) => {
                let body = self.rigid_bodies.get(*index)?;
                Some(body.prev_position.lerp(body.position, alpha))
            }
        }
    }

    /// Get interpolated rigid body render data (position, half_extents, rotation).
    /// Returns None if handle is not a rigid body.
    pub fn get_rigid_body_render_data_interpolated(
        &self,
        handle: BodyHandle,
        alpha: f32,
    ) -> Option<(Vec2, Vec2, f32)> {
        let BodyType::Rigid(index) = self.body_types.get(handle.0)?.as_ref()? else {
            return None;
        };
        let body = self.rigid_bodies.get(*index)?;

        let alpha = alpha.clamp(0.0, 1.0);
        let position = body.prev_position.lerp(body.position, alpha);
        let rotation = body.prev_rotation * (1.0 - alpha) + body.rotation * alpha;
        let half_extents = body.collider.half_extents();

        Some((position, half_extents, rotation))
    }
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::create_ring_mesh;

    #[test]
    fn test_add_remove_body() {
        let mut world = PhysicsWorld::new();
        let mesh = create_ring_mesh(1.0, 0.5, 8, 2);

        let h1 = world.add_body_simple(&mesh, 0.0, 0.0);
        let h2 = world.add_body_simple(&mesh, 2.0, 0.0);

        assert_eq!(world.body_count(), 2);
        assert!(world.contains(h1));
        assert!(world.contains(h2));

        world.remove_body(h1);

        assert_eq!(world.body_count(), 1);
        assert!(!world.contains(h1));
        assert!(world.contains(h2));
    }

    #[test]
    fn test_collision_groups() {
        let group_a = CollisionGroups::new(0b0001, 0b0010);
        let group_b = CollisionGroups::new(0b0010, 0b0001);
        let group_c = CollisionGroups::new(0b0100, 0b0100);

        assert!(group_a.can_collide(&group_b));
        assert!(group_b.can_collide(&group_a));
        assert!(!group_a.can_collide(&group_c));
        assert!(!group_c.can_collide(&group_a));
    }

    #[test]
    fn test_apply_impulse() {
        let mut world = PhysicsWorld::new();
        let mesh = create_ring_mesh(1.0, 0.5, 8, 2);
        let handle = world.add_body_simple(&mesh, 0.0, 0.0);

        world.apply_impulse(handle, 5.0, 10.0);

        let vel = world.get_velocity(handle).unwrap();
        assert!((vel.x - 5.0).abs() < 0.01);
        assert!((vel.y - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_position() {
        let mut world = PhysicsWorld::new();
        let mesh = create_ring_mesh(1.0, 0.5, 8, 2);
        let handle = world.add_body(&mesh, BodyConfig::new().at_position(3.0, 4.0));

        let pos = world.get_position(handle).unwrap();
        assert!((pos.x - 3.0).abs() < 0.01);
        assert!((pos.y - 4.0).abs() < 0.01);

        world.set_position(handle, 10.0, 20.0);
        let pos = world.get_position(handle).unwrap();
        assert!((pos.x - 10.0).abs() < 0.01);
        assert!((pos.y - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_add_rigid_body() {
        let mut world = PhysicsWorld::new();

        let rigid_handle = world.add_rigid_body(
            RigidBodyConfig::new()
                .as_circle(1.0)
                .at_position(5.0, 10.0)
                .with_density(1000.0)
        );

        assert!(world.is_rigid(rigid_handle));
        assert!(world.contains(rigid_handle));

        let pos = world.get_position(rigid_handle).unwrap();
        assert!((pos.x - 5.0).abs() < 0.01);
        assert!((pos.y - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_mixed_bodies() {
        let mut world = PhysicsWorld::new();
        let mesh = create_ring_mesh(1.0, 0.5, 8, 2);

        let soft_handle = world.add_body_simple(&mesh, 0.0, 0.0);
        let rigid_handle = world.add_rigid_body(
            RigidBodyConfig::new()
                .as_circle(0.5)
                .at_position(3.0, 0.0)
        );

        assert!(!world.is_rigid(soft_handle));
        assert!(world.is_rigid(rigid_handle));
        assert_eq!(world.body_count(), 2);

        // Check that get_body returns soft body only for soft handle
        assert!(world.get_body(soft_handle).is_some());
        assert!(world.get_body(rigid_handle).is_none());

        // Check that get_rigid_body returns rigid body only for rigid handle
        assert!(world.get_rigid_body(soft_handle).is_none());
        assert!(world.get_rigid_body(rigid_handle).is_some());
    }

    /// Helper: create a donut-game-like setup and return (world, donut_handle)
    fn setup_donut_world() -> (PhysicsWorld, BodyHandle) {
        let mut world = PhysicsWorld::new();
        world.set_gravity(-9.8);
        world.set_ground(Some(-4.5));

        let mesh = create_ring_mesh(1.0, 0.4, 16, 6);
        let handle = world.add_body(
            &mesh,
            BodyConfig::new()
                .at_position(0.0, 3.0)
                .with_material(crate::Material::RUBBER),
        );
        (world, handle)
    }

    /// BUG 1: Force accumulator is cleared after first substep.
    /// With 4 substeps, only substep 1 gets the force. Substeps 2-4 get nothing.
    /// This means the body receives 1/4 of the intended force.
    #[test]
    fn test_bug_force_accum_cleared_too_early() {
        let (mut world, handle) = setup_donut_world();

        // Let body settle on ground first
        let dt = 1.0 / 60.0;
        for _ in 0..120 {
            world.step(dt);
        }

        let pos_before = world.get_position(handle).unwrap();
        println!("Settled position: ({:.3}, {:.3})", pos_before.x, pos_before.y);

        // Apply rightward force for 60 frames (1 second)
        for frame in 0..60 {
            world.apply_force(handle, 1200.0, 0.0);
            world.step(dt);

            if frame % 10 == 0 {
                let pos = world.get_position(handle).unwrap();
                let vel = world.get_velocity(handle).unwrap();
                println!(
                    "Frame {:3}: pos=({:.3}, {:.3}) vel=({:.3}, {:.3})",
                    frame, pos.x, pos.y, vel.x, vel.y
                );
            }
        }

        let vel_after = world.get_velocity(handle).unwrap();
        let pos_after = world.get_position(handle).unwrap();
        println!("After 60 frames of 1200N force:");
        println!("  pos: ({:.3}, {:.3})", pos_after.x, pos_after.y);
        println!("  vel: ({:.3}, {:.3})", vel_after.x, vel_after.y);

        // The body should have moved significantly rightward.
        // With correct physics (force applied all 4 substeps), vel.x should be substantial.
        // With the bug (force only in substep 1), it's ~1/4 of expected.
        assert!(
            pos_after.x > pos_before.x + 1.0,
            "Donut barely moved! pos.x delta = {:.3}. Force may only be applied in first substep.",
            pos_after.x - pos_before.x
        );
    }

    /// BUG 2: Torque center-of-mass uses positions AFTER force integration.
    /// Apply a rightward force + clockwise torque (negative). Verify the donut
    /// consistently moves right AND rotates clockwise, not erratically.
    #[test]
    fn test_bug_torque_direction_consistency() {
        let (mut world, handle) = setup_donut_world();
        let dt = 1.0 / 60.0;

        // Settle
        for _ in 0..120 {
            world.step(dt);
        }

        // Apply rightward force + clockwise torque for 30 frames
        let mut angular_velocities = Vec::new();
        for frame in 0..30 {
            world.apply_force(handle, 1200.0, 0.0);
            world.apply_torque(handle, -800.0, dt);
            world.step(dt);

            let omega = world.get_angular_velocity(handle).unwrap_or(0.0);
            angular_velocities.push(omega);

            if frame % 5 == 0 {
                let pos = world.get_position(handle).unwrap();
                let vel = world.get_velocity(handle).unwrap();
                println!(
                    "Frame {:3}: pos=({:.3}, {:.3}) vel=({:.3}, {:.3}) omega={:.3}",
                    frame, pos.x, pos.y, vel.x, vel.y, omega
                );
            }
        }

        // Check: angular velocity should be consistently negative (clockwise)
        let negative_count = angular_velocities.iter().filter(|&&w| w < -0.01).count();
        let positive_count = angular_velocities.iter().filter(|&&w| w > 0.01).count();
        println!(
            "Angular velocity: {} negative, {} positive, {} near-zero (out of {})",
            negative_count,
            positive_count,
            angular_velocities.len() - negative_count - positive_count,
            angular_velocities.len()
        );

        // With clockwise torque (-3000), angular velocity should be consistently negative
        assert!(
            negative_count > positive_count * 3,
            "Torque direction is inconsistent! {} negative vs {} positive frames. \
             Center-of-mass may be computed from wrong positions.",
            negative_count, positive_count
        );
    }

    /// Diagnostic: Simulate the actual donut game input pattern and dump full state.
    /// Press right for 30 frames, release for 30 frames, jump, observe.
    #[test]
    fn test_diagnostic_full_game_simulation() {
        let (mut world, handle) = setup_donut_world();
        let dt = 1.0 / 60.0;

        println!("=== Full Game Simulation Diagnostic ===");
        println!("Phase 1: Settle (120 frames)");

        // Phase 1: Settle
        for _ in 0..120 {
            world.step(dt);
        }
        let pos = world.get_position(handle).unwrap();
        println!("Settled at: ({:.3}, {:.3})", pos.x, pos.y);

        // Phase 2: Press right for 30 frames
        println!("\nPhase 2: Press RIGHT (30 frames, force=1200, torque=-800)");
        for frame in 0..30 {
            world.apply_force(handle, 1200.0, 0.0);
            world.apply_torque(handle, -800.0, dt);
            world.step(dt);

            let pos = world.get_position(handle).unwrap();
            let vel = world.get_velocity(handle).unwrap();
            let omega = world.get_angular_velocity(handle).unwrap_or(0.0);
            let grounded = world.is_grounded(handle, 0.5);
            println!(
                "  [{:3}] pos=({:7.3}, {:7.3}) vel=({:7.3}, {:7.3}) omega={:7.3} grounded={}",
                frame, pos.x, pos.y, vel.x, vel.y, omega, grounded
            );
        }

        // Phase 3: Release for 30 frames — observe deceleration
        println!("\nPhase 3: RELEASE (30 frames, no input)");
        for frame in 0..30 {
            world.step(dt);

            let pos = world.get_position(handle).unwrap();
            let vel = world.get_velocity(handle).unwrap();
            let omega = world.get_angular_velocity(handle).unwrap_or(0.0);
            let grounded = world.is_grounded(handle, 0.5);
            println!(
                "  [{:3}] pos=({:7.3}, {:7.3}) vel=({:7.3}, {:7.3}) omega={:7.3} grounded={}",
                frame, pos.x, pos.y, vel.x, vel.y, omega, grounded
            );
        }

        // Phase 4: Jump
        println!("\nPhase 4: JUMP (impulse y=8.0, then 60 frames)");
        let grounded = world.is_grounded(handle, 0.5);
        println!("  Pre-jump grounded: {}", grounded);
        world.apply_impulse(handle, 0.0, 8.0);
        for frame in 0..60 {
            world.step(dt);

            let pos = world.get_position(handle).unwrap();
            let vel = world.get_velocity(handle).unwrap();
            let grounded = world.is_grounded(handle, 0.5);
            if frame % 5 == 0 {
                println!(
                    "  [{:3}] pos=({:7.3}, {:7.3}) vel=({:7.3}, {:7.3}) grounded={}",
                    frame, pos.x, pos.y, vel.x, vel.y, grounded
                );
            }
        }

        // Phase 5: Press LEFT for 30 frames — should go left reliably
        println!("\nPhase 5: Press LEFT (30 frames, force=-1200, torque=800)");
        for frame in 0..30 {
            world.apply_force(handle, -1200.0, 0.0);
            world.apply_torque(handle, 800.0, dt);
            world.step(dt);

            let pos = world.get_position(handle).unwrap();
            let vel = world.get_velocity(handle).unwrap();
            let omega = world.get_angular_velocity(handle).unwrap_or(0.0);
            println!(
                "  [{:3}] pos=({:7.3}, {:7.3}) vel=({:7.3}, {:7.3}) omega={:7.3}",
                frame, pos.x, pos.y, vel.x, vel.y, omega
            );
        }
    }
}
