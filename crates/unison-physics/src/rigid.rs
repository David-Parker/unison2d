//! Rigid body physics using XPBD-style position-based dynamics
//!
//! This module provides true rigid bodies as an alternative to soft bodies.
//! Rigid bodies maintain their shape and support circle and AABB colliders.

use unison_math::Vec2;

/// Collider shape for rigid bodies
#[derive(Clone, Debug)]
pub enum Collider {
    /// Circle collider with given radius
    Circle { radius: f32 },
    /// Axis-aligned bounding box with half-extents
    AABB { half_width: f32, half_height: f32 },
}

impl Collider {
    /// Create a circle collider
    pub fn circle(radius: f32) -> Self {
        Collider::Circle { radius }
    }

    /// Create an AABB collider
    pub fn aabb(half_width: f32, half_height: f32) -> Self {
        Collider::AABB { half_width, half_height }
    }

    /// Get the half-extents of this collider (for rendering)
    pub fn half_extents(&self) -> Vec2 {
        match self {
            Collider::Circle { radius } => Vec2::new(*radius, *radius),
            Collider::AABB { half_width, half_height } => Vec2::new(*half_width, *half_height),
        }
    }

    /// Get the AABB bounds for this collider at a given position and rotation
    pub fn get_aabb(&self, position: Vec2, rotation: f32) -> (f32, f32, f32, f32) {
        match self {
            Collider::Circle { radius } => (
                position.x - radius,
                position.y - radius,
                position.x + radius,
                position.y + radius,
            ),
            Collider::AABB { half_width, half_height } => {
                // For rotated AABB, compute bounding box of the rotated rectangle
                let cos_r = rotation.cos();
                let sin_r = rotation.sin();

                // Corner offsets
                let corners = [
                    (*half_width, *half_height),
                    (-*half_width, *half_height),
                    (-*half_width, -*half_height),
                    (*half_width, -*half_height),
                ];

                let mut min_x = f32::INFINITY;
                let mut min_y = f32::INFINITY;
                let mut max_x = f32::NEG_INFINITY;
                let mut max_y = f32::NEG_INFINITY;

                for (cx, cy) in corners {
                    let rx = cx * cos_r - cy * sin_r + position.x;
                    let ry = cx * sin_r + cy * cos_r + position.y;
                    min_x = min_x.min(rx);
                    min_y = min_y.min(ry);
                    max_x = max_x.max(rx);
                    max_y = max_y.max(ry);
                }

                (min_x, min_y, max_x, max_y)
            }
        }
    }
}

/// A rigid body with position, rotation, and velocity state
#[derive(Clone, Debug)]
pub struct RigidBody {
    /// Current position
    pub position: Vec2,
    /// Current rotation in radians
    pub rotation: f32,
    /// Linear velocity
    pub linear_velocity: Vec2,
    /// Angular velocity in radians per second
    pub angular_velocity: f32,
    /// Previous position (for XPBD integration)
    pub prev_position: Vec2,
    /// Previous rotation (for XPBD integration)
    pub prev_rotation: f32,
    /// Inverse mass (0 = kinematic/static)
    pub inv_mass: f32,
    /// Inverse moment of inertia (0 = kinematic/static)
    pub inv_inertia: f32,
    /// Collider shape
    pub collider: Collider,
    /// Friction coefficient (0-1)
    pub friction: f32,
    /// Restitution/bounciness (0-1)
    pub restitution: f32,
}

impl RigidBody {
    /// Create a new rigid body from configuration
    pub fn new(config: &RigidBodyConfig) -> Self {
        let (inv_mass, inv_inertia) = if config.is_kinematic {
            (0.0, 0.0)
        } else {
            let mass = config.density * Self::compute_area(&config.collider);
            let inertia = Self::compute_inertia(&config.collider, mass);
            (
                if mass > 1e-10 { 1.0 / mass } else { 0.0 },
                if inertia > 1e-10 { 1.0 / inertia } else { 0.0 },
            )
        };

        RigidBody {
            position: config.position,
            rotation: config.rotation,
            linear_velocity: config.velocity,
            angular_velocity: config.angular_velocity,
            prev_position: config.position,
            prev_rotation: config.rotation,
            inv_mass,
            inv_inertia,
            collider: config.collider.clone(),
            friction: config.friction,
            restitution: config.restitution,
        }
    }

    /// Compute area for mass calculation
    fn compute_area(collider: &Collider) -> f32 {
        match collider {
            Collider::Circle { radius } => std::f32::consts::PI * radius * radius,
            Collider::AABB { half_width, half_height } => 4.0 * half_width * half_height,
        }
    }

    /// Compute moment of inertia for a given collider and mass
    fn compute_inertia(collider: &Collider, mass: f32) -> f32 {
        match collider {
            // I = 0.5 * m * r^2 for solid disk
            Collider::Circle { radius } => 0.5 * mass * radius * radius,
            // I = (1/12) * m * (w^2 + h^2) for rectangle
            Collider::AABB { half_width, half_height } => {
                let w = 2.0 * half_width;
                let h = 2.0 * half_height;
                (1.0 / 12.0) * mass * (w * w + h * h)
            }
        }
    }

    /// Check if this body is kinematic (not affected by physics)
    pub fn is_kinematic(&self) -> bool {
        self.inv_mass == 0.0
    }

    /// Get the AABB for this body
    pub fn get_aabb(&self) -> (f32, f32, f32, f32) {
        self.collider.get_aabb(self.position, self.rotation)
    }

    /// Get center position
    pub fn get_center(&self) -> Vec2 {
        self.position
    }

    /// Apply an impulse at the center of mass
    pub fn apply_impulse(&mut self, impulse_x: f32, impulse_y: f32) {
        if self.inv_mass > 0.0 {
            self.linear_velocity.x += impulse_x * self.inv_mass;
            self.linear_velocity.y += impulse_y * self.inv_mass;
        }
    }

    /// Apply an impulse at a world point (creates torque)
    pub fn apply_impulse_at_point(&mut self, impulse_x: f32, impulse_y: f32, point_x: f32, point_y: f32) {
        if self.inv_mass > 0.0 {
            self.linear_velocity.x += impulse_x * self.inv_mass;
            self.linear_velocity.y += impulse_y * self.inv_mass;
        }
        if self.inv_inertia > 0.0 {
            // Torque = r x F (2D cross product)
            let rx = point_x - self.position.x;
            let ry = point_y - self.position.y;
            let torque = rx * impulse_y - ry * impulse_x;
            self.angular_velocity += torque * self.inv_inertia;
        }
    }

    /// Apply angular impulse (torque * dt)
    pub fn apply_angular_impulse(&mut self, angular_impulse: f32) {
        if self.inv_inertia > 0.0 {
            self.angular_velocity += angular_impulse * self.inv_inertia;
        }
    }

    /// Pre-solve: store previous state and integrate velocity
    pub fn pre_solve(&mut self, dt: f32, gravity: f32) {
        if self.inv_mass == 0.0 {
            return; // Kinematic body
        }

        // Store previous state
        self.prev_position = self.position;
        self.prev_rotation = self.rotation;

        // Apply gravity
        self.linear_velocity.y += gravity * dt;

        // Integrate position
        self.position.x += self.linear_velocity.x * dt;
        self.position.y += self.linear_velocity.y * dt;
        self.rotation += self.angular_velocity * dt;
    }

    /// Post-solve: derive velocities from position change
    pub fn post_solve(&mut self, dt: f32) {
        if self.inv_mass == 0.0 {
            return; // Kinematic body
        }

        let inv_dt = 1.0 / dt;
        self.linear_velocity.x = (self.position.x - self.prev_position.x) * inv_dt;
        self.linear_velocity.y = (self.position.y - self.prev_position.y) * inv_dt;
        self.angular_velocity = (self.rotation - self.prev_rotation) * inv_dt;
    }

    /// Solve ground collision
    pub fn solve_ground_collision(&mut self, ground_y: f32, friction: f32, restitution: f32) {
        if self.inv_mass == 0.0 {
            return;
        }

        let (_, min_y, _, _) = self.get_aabb();

        if min_y < ground_y {
            let penetration = ground_y - min_y;

            // Push out of ground
            self.position.y += penetration;

            // Apply restitution to vertical velocity
            if self.linear_velocity.y < 0.0 {
                self.linear_velocity.y = -self.linear_velocity.y * restitution;
            }

            // Apply friction to horizontal velocity
            self.linear_velocity.x *= 1.0 - friction;

            // Dampen angular velocity when on ground
            self.angular_velocity *= 1.0 - friction * 0.5;
        }
    }

    /// Check if a point is inside this collider (for soft body collision)
    pub fn contains_point(&self, px: f32, py: f32) -> Option<(f32, f32, f32)> {
        match &self.collider {
            Collider::Circle { radius } => {
                let dx = px - self.position.x;
                let dy = py - self.position.y;
                let dist_sq = dx * dx + dy * dy;
                let radius_sq = radius * radius;

                if dist_sq < radius_sq && dist_sq > 1e-10 {
                    let dist = dist_sq.sqrt();
                    let penetration = radius - dist;
                    let nx = dx / dist;
                    let ny = dy / dist;
                    Some((penetration, nx, ny))
                } else {
                    None
                }
            }
            Collider::AABB { half_width, half_height } => {
                // Transform point to local space
                let cos_r = self.rotation.cos();
                let sin_r = self.rotation.sin();
                let dx = px - self.position.x;
                let dy = py - self.position.y;
                let local_x = dx * cos_r + dy * sin_r;
                let local_y = -dx * sin_r + dy * cos_r;

                // Check if inside AABB
                if local_x.abs() < *half_width && local_y.abs() < *half_height {
                    // Find closest edge and compute penetration
                    let pen_left = local_x + half_width;
                    let pen_right = half_width - local_x;
                    let pen_bottom = local_y + half_height;
                    let pen_top = half_height - local_y;

                    let min_pen = pen_left.min(pen_right).min(pen_bottom).min(pen_top);

                    // Determine normal based on closest edge
                    let (local_nx, local_ny) = if min_pen == pen_left {
                        (-1.0, 0.0)
                    } else if min_pen == pen_right {
                        (1.0, 0.0)
                    } else if min_pen == pen_bottom {
                        (0.0, -1.0)
                    } else {
                        (0.0, 1.0)
                    };

                    // Transform normal back to world space
                    let nx = local_nx * cos_r - local_ny * sin_r;
                    let ny = local_nx * sin_r + local_ny * cos_r;

                    Some((min_pen, nx, ny))
                } else {
                    None
                }
            }
        }
    }
}

/// Configuration for creating a rigid body
#[derive(Clone, Debug)]
pub struct RigidBodyConfig {
    /// Collider shape
    pub collider: Collider,
    /// Density in kg/m^2
    pub density: f32,
    /// Initial position
    pub position: Vec2,
    /// Initial rotation in radians
    pub rotation: f32,
    /// Initial velocity
    pub velocity: Vec2,
    /// Initial angular velocity
    pub angular_velocity: f32,
    /// If true, body is kinematic (not affected by physics)
    pub is_kinematic: bool,
    /// Friction coefficient (0-1)
    pub friction: f32,
    /// Restitution/bounciness (0-1)
    pub restitution: f32,
}

impl Default for RigidBodyConfig {
    fn default() -> Self {
        Self {
            collider: Collider::Circle { radius: 1.0 },
            density: 1000.0,
            position: Vec2::ZERO,
            rotation: 0.0,
            velocity: Vec2::ZERO,
            angular_velocity: 0.0,
            is_kinematic: false,
            friction: 0.5,
            restitution: 0.3,
        }
    }
}

impl RigidBodyConfig {
    /// Create a new config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the collider shape
    pub fn with_collider(mut self, collider: Collider) -> Self {
        self.collider = collider;
        self
    }

    /// Set as a circle collider
    pub fn as_circle(mut self, radius: f32) -> Self {
        self.collider = Collider::Circle { radius };
        self
    }

    /// Set as an AABB collider
    pub fn as_aabb(mut self, half_width: f32, half_height: f32) -> Self {
        self.collider = Collider::AABB { half_width, half_height };
        self
    }

    /// Set density
    pub fn with_density(mut self, density: f32) -> Self {
        self.density = density;
        self
    }

    /// Set position
    pub fn at_position(mut self, x: f32, y: f32) -> Self {
        self.position = Vec2::new(x, y);
        self
    }

    /// Set rotation
    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set velocity
    pub fn with_velocity(mut self, vx: f32, vy: f32) -> Self {
        self.velocity = Vec2::new(vx, vy);
        self
    }

    /// Set angular velocity
    pub fn with_angular_velocity(mut self, omega: f32) -> Self {
        self.angular_velocity = omega;
        self
    }

    /// Make kinematic (not affected by physics)
    pub fn as_kinematic(mut self) -> Self {
        self.is_kinematic = true;
        self
    }

    /// Set friction coefficient
    pub fn with_friction(mut self, friction: f32) -> Self {
        self.friction = friction.clamp(0.0, 1.0);
        self
    }

    /// Set restitution (bounciness)
    pub fn with_restitution(mut self, restitution: f32) -> Self {
        self.restitution = restitution.clamp(0.0, 1.0);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rigid_body_creation() {
        let config = RigidBodyConfig::new()
            .as_circle(1.0)
            .at_position(0.0, 5.0)
            .with_density(1000.0);

        let body = RigidBody::new(&config);

        assert!(body.inv_mass > 0.0);
        assert!(body.inv_inertia > 0.0);
        assert_eq!(body.position, Vec2::new(0.0, 5.0));
    }

    #[test]
    fn test_kinematic_body() {
        let config = RigidBodyConfig::new()
            .as_circle(1.0)
            .as_kinematic();

        let body = RigidBody::new(&config);

        assert!(body.is_kinematic());
        assert_eq!(body.inv_mass, 0.0);
        assert_eq!(body.inv_inertia, 0.0);
    }

    #[test]
    fn test_collider_aabb() {
        let collider = Collider::Circle { radius: 1.0 };
        let aabb = collider.get_aabb(Vec2::ZERO, 0.0);

        assert_eq!(aabb, (-1.0, -1.0, 1.0, 1.0));
    }

    #[test]
    fn test_contains_point_circle() {
        let config = RigidBodyConfig::new()
            .as_circle(1.0)
            .at_position(0.0, 0.0);
        let body = RigidBody::new(&config);

        // Point inside
        assert!(body.contains_point(0.5, 0.0).is_some());

        // Point outside
        assert!(body.contains_point(2.0, 0.0).is_none());
    }
}
