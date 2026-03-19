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
world.set_solver_iterations(3, 2); // pre-collision, post-collision
```

### Solver Defaults

| Parameter | Default | Description |
|-----------|---------|-------------|
| Gravity | -9.8 | Downward acceleration |
| Substeps | 4 | Physics substeps per frame |
| Pre-collision iterations | 3 | Constraint solves before collision |
| Post-collision iterations | 2 | Constraint solves after collision |
| Ground friction | 0.8 | Coulomb friction coefficient |
| Ground restitution | 0.3 | Bounciness |

The XPBD solver includes adaptive constraint behavior:

- **Velocity clamping**: Per-vertex velocity is capped at 25 units/sec in `pre_solve` to prevent tunneling and energy explosion during high-speed impacts.
- **Adaptive edge compliance**: Edges compressed below 40% or stretched beyond 250% of rest length get compliance capped at 0.1 for aggressive correction.
- **Adaptive area compliance**: Inverted triangles (signed area flipped) get zero compliance for stiff materials (alpha < 1.0) or 1% compliance for soft materials, preventing fold-through without energy injection.
- **Rotation-aware forensics**: `MeshForensics::analyze` compares sorted dimensions (min-to-min, max-to-max) so rigid body rotation doesn't register as collapse.
- **Internal damping**: Stiff materials (compliance < 1e-7) get per-substep deformation-only damping that preserves linear and angular momentum. This kills post-impact oscillation without affecting fall speed or rolling. Soft materials get light global damping (0.5% per frame).

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

Soft body render positions are automatically inflated outward from center by `render_inflation` (default: half of collision skin) to visually hide the collision gap.

```rust
// Single body (positions inflated for rendering)
world.get_body_render_data(handle)             // -> Option<(Vec<f32>, &[u32])>

// Single body with interpolation (also inflated)
world.get_body_render_data_interpolated(handle, alpha) // -> Option<(Vec<f32>, &[u32])>
world.get_position_interpolated(handle, alpha)         // -> Option<Vec2>

// Rigid body interpolated
world.get_rigid_body_render_data_interpolated(handle, alpha)
    // -> Option<(Vec2, Vec2, f32)>  (position, half_extents, rotation)

// Control inflation
world.set_render_inflation(0.025);  // default: 0.025
world.render_inflation()            // -> f32
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
| `Material::SLIME` | Ultra-soft, blobby |
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

## Forensics (Testing)

Mesh shape forensics for detecting permanent deformation, jagged edges, vertex clustering, and triangle inversion. Used by the `shape_integrity` integration test battery.

```rust
use unison_physics::forensics::*;

// Capture baseline before simulation
let baseline = ShapeBaseline::capture(&body);

// Analyze after simulation
let f = MeshForensics::analyze(&body, &baseline);
println!("{}", f.summary());

// Check health against tolerance
let issues = f.is_healthy(&HealthTolerance::strict());
// Also: HealthTolerance::soft_material(), HealthTolerance::during_collision()
```

Key metrics in `MeshForensics`:
- `width_ratio`, `height_ratio` — dimension preservation (1.0 = perfect)
- `min_edge_ratio`, `max_edge_ratio`, `edge_ratio_stddev` — edge length health
- `severely_compressed_edges`, `severely_stretched_edges` — edge extremes
- `min_area_ratio`, `inverted_triangles`, `collapsed_triangles` — triangle health
- `max_boundary_angle_deviation` — jagged edge detection
- `min_vertex_separation` — vertex clustering detection

`ForensicSimulation::run_single(...)` runs a simulation with periodic forensic capture.

## Low-Level: XPBDSoftBody

Direct access to the XPBD solver. Most users should use `PhysicsWorld` instead.

Key public fields: `pos`, `prev_pos`, `vel`, `inv_mass`, `triangles`, `edge_constraints`, `area_constraints`, `edge_compliance`, `area_compliance`.

Key methods: `pre_solve`, `post_solve`, `solve_constraints`, `apply_damping`, `apply_internal_damping`, `substep`, `collide_with_body`, `get_center`, `get_aabb`, `get_kinetic_energy`.

## Low-Level: CollisionSystem

Spatial-hash based broad-phase collision detection.

```rust
let mut collision = CollisionSystem::new(min_dist);
collision.prepare(&bodies);     // broad phase
collision.resolve_collisions(&mut bodies); // narrow phase + resolve
```
