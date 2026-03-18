# unison-physics

XPBD soft body and rigid body physics simulation. Platform-independent. Uses `Vec2` from `unison-math`.

## PhysicsWorld

The main entry point. Manages all bodies and runs the simulation.

```rust
use unison_physics::{PhysicsWorld, BodyHandle, BodyConfig, Material, Mesh};
use unison_math::Vec2;

let mut world = PhysicsWorld::new();
world.set_gravity(-9.81);
world.set_ground(Some(0.0));
world.set_ground_friction(0.5);
world.set_ground_restitution(0.3);
world.set_substeps(8);
```

### Adding Bodies

```rust
// Soft body with config
let mesh = create_ring_mesh(1.0, 0.5, 16, 4);
let handle = world.add_body(&mesh, BodyConfig::new()
    .with_material(Material::RUBBER)
    .at_position(0.0, 5.0)
    .with_velocity(1.0, 0.0));

// Simple soft body
let handle = world.add_body_simple(&mesh, 0.0, 5.0);

// Rigid body
let handle = world.add_rigid_body(RigidBodyConfig::new()
    .as_circle(0.5)
    .at_position(3.0, 2.0)
    .with_density(1.0));
```

### Simulation

```rust
world.snapshot_for_render(); // call before step() for interpolation
world.step(dt);              // flat ground

// or with terrain
world.step_with_terrain(dt,
    |x| terrain_height(x),
    |x| terrain_normal(x));
```

### Forces & Velocity

```rust
world.apply_force(handle, fx, fy);
world.apply_impulse(handle, vx, vy);
world.apply_central_force(handle, fx, fy);
world.apply_central_impulse(handle, vx, vy);
world.set_velocity(handle, vx, vy);
world.set_linear_velocity(handle, vx, vy);
world.apply_angular_velocity(handle, omega);
world.apply_torque(handle, torque, dt);
world.apply_acceleration(handle, ax, ay, dt);
```

### Queries

```rust
world.get_position(handle)       // -> Option<Vec2>
world.get_velocity(handle)       // -> Option<Vec2>
world.get_angular_velocity(handle) // -> Option<f32>
world.get_aabb(handle)           // -> Option<(min_x, min_y, max_x, max_y)>
world.get_lowest_y(handle)       // -> Option<f32>
world.is_grounded(handle, 0.1)   // -> bool (checks ground, platforms, and other bodies)
world.get_contact(handle, 0.1)   // -> Option<BodyHandle>
world.get_kinetic_energy(handle) // -> Option<f32>
world.total_kinetic_energy()     // -> f32
```

### Position & Deformation

```rust
world.translate(handle, dx, dy);
world.set_position(handle, x, y);
world.set_vertical_compression(handle, 0.5);  // squash vertically
world.set_squash(handle, 0.5, 1.5);           // squash-and-stretch
world.reset_rest_lengths(handle);              // restore original shape
```

### Render Data

```rust
// All bodies
world.get_render_data() // -> Vec<(&[f32], &[u32])> (positions, triangles)

// Single body with interpolation
world.get_body_render_data_interpolated(handle, alpha) // -> Option<(Vec<f32>, &[u32])>
world.get_position_interpolated(handle, alpha)         // -> Option<Vec2>

// Rigid body interpolated
world.get_rigid_body_render_data_interpolated(handle, alpha)
    // -> Option<(Vec2, Vec2, f32)>  (position, half_extents, rotation)
```

### Iteration

```rust
world.handles()    // -> impl Iterator<Item = BodyHandle>
world.iter()       // -> impl Iterator<Item = (BodyHandle, &XPBDSoftBody)>
world.iter_mut()   // -> impl Iterator<Item = (BodyHandle, &mut XPBDSoftBody)>
world.iter_rigid() // -> impl Iterator<Item = (BodyHandle, &RigidBody)>
```

## BodyConfig

Builder for soft body creation.

```rust
BodyConfig::new()
    .with_material(Material::JELLO)
    .with_collision_groups(CollisionGroups::new(0x01, 0x03))
    .at_position(x, y)
    .with_velocity(vx, vy)
    .without_collisions()
```

## Material

Predefined materials and custom creation.

| Preset | Description |
|--------|-------------|
| `Material::JELLO` | Soft, jiggly |
| `Material::RUBBER` | Bouncy (default) |
| `Material::WOOD` | Stiff |
| `Material::METAL` | Nearly rigid |

```rust
Material::new(density, edge_compliance, area_compliance)
```

## CollisionGroups

Bitfield-based collision filtering.

```rust
CollisionGroups::ALL   // collide with everything (default)
CollisionGroups::NONE  // no collisions
CollisionGroups::new(membership, filter)
groups.can_collide(&other) // bidirectional check
```

## Mesh

Vertex/triangle data for soft bodies.

```rust
Mesh::new(vertices, triangles)
Mesh::with_uvs(vertices, triangles, uvs)
mesh.vertex_count()
mesh.uvs_or_default()
```

### Mesh Generators

```rust
// Ring/donut shape
create_ring_mesh(outer_radius, inner_radius, segments, radial_divisions)

// Simple shapes
create_square_mesh(size, divisions)
create_rounded_box_mesh(width, height, corner_radius, corner_segments)

// Parametric shapes
create_ellipse_mesh(width, height, segments, rings)
create_star_mesh(outer_radius, inner_radius, points, rings)
create_blob_mesh(base_radius, variation, segments, rings, seed)

// Utilities
offset_vertices(&mut vertices, dx, dy)
```

## RigidBody

Non-deformable physics body with circle or AABB collider. Position and velocity fields use `Vec2`.

```rust
RigidBodyConfig::new()
    .as_circle(radius)           // or .as_aabb(half_w, half_h)
    .with_density(1.0)
    .at_position(x, y)
    .with_rotation(angle)
    .with_velocity(vx, vy)
    .with_angular_velocity(omega)
    .with_friction(0.8)          // default: 0.8
    .with_restitution(0.3)
    .as_kinematic()              // immovable
```

### Collider

```rust
Collider::circle(radius)
Collider::aabb(half_width, half_height)
collider.half_extents()  // -> Vec2
```

## Math

Internal 2x2 matrix utilities used by the FEM solver. For `Vec2`, see `unison-math`.

```rust
type Mat2 = [f32; 4]; // column-major

// Matrix ops
mat2_create, mat2_identity, mat2_det, mat2_inv, mat2_transpose,
mat2_inv_transpose, mat2_mul, mat2_mul_vec, mat2_add, mat2_sub,
mat2_scale, mat2_trace, mat2_frobenius_norm_sq
```

## SimulationTracer

Debug tool for capturing frame snapshots. `FrameTrace` fields `centroid` and `linear_velocity` are `Vec2`. `TraceStatistics` fields `start_centroid` and `end_centroid` are `Vec2`.

```rust
let mut tracer = SimulationTracer::new(120); // keep last 120 frames
tracer.enable();
tracer.capture_frame(frame, dt, positions, velocities, triangles, rest_areas);
tracer.statistics()       // -> TraceStatistics
tracer.detect_anomalies() // -> Vec<String>
tracer.print_summary(10);
tracer.to_csv()           // -> String
```

## Low-Level: XPBDSoftBody

Direct access to the XPBD solver. Most users should use `PhysicsWorld` instead.

Key public fields: `pos`, `prev_pos`, `vel`, `inv_mass`, `triangles`, `edge_constraints`, `area_constraints`, `edge_compliance`, `area_compliance`.

Key methods: `pre_solve`, `post_solve`, `solve_constraints`, `apply_damping`, `substep`, `collide_with_body`, `get_center`, `get_aabb`, `get_kinetic_energy`.

## Low-Level: CollisionSystem

Spatial-hash based broad-phase collision detection.

```rust
let mut collision = CollisionSystem::new(min_dist);
collision.prepare(&bodies);     // broad phase
collision.resolve_collisions(&mut bodies); // narrow phase + resolve
```
