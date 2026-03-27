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
world.set_contact_iterations(5);   // collision passes per substep
```

### Solver Defaults

| Parameter | Default | Description |
|-----------|---------|-------------|
| Gravity | -9.8 | Downward acceleration |
| Substeps | 4 | Physics substeps per frame |
| Pre-collision iterations | 3 | Constraint solves before collision |
| Post-collision iterations | 2 | Constraint solves after collision |
| Contact iterations | 5 | Collision resolution passes per substep |
| Ground friction | 0.8 | Coulomb friction coefficient |
| Ground restitution | 0.3 | Bounciness |
| Render inflation | 0.075 | Soft body mesh inflation for rendering |

The XPBD solver includes adaptive constraint behavior:

- **Velocity clamping**: Per-vertex velocity is capped at 25 units/sec in `pre_solve` to prevent tunneling and energy explosion during high-speed impacts.
- **Adaptive edge compliance**: Edges compressed below 40% or stretched beyond 250% of rest length get compliance capped at 0.1 for aggressive correction.
- **Adaptive area compliance**: Inverted triangles (signed area flipped) get zero compliance for stiff materials (alpha < 1.0) or 1% compliance for soft materials, preventing fold-through without energy injection.
- **Rotation-aware forensics**: `MeshForensics::analyze` compares sorted dimensions (min-to-min, max-to-max) so rigid body rotation doesn't register as collapse.
- **Internal damping**: Stiff materials (compliance < 1e-7) get per-substep deformation-only damping that preserves linear and angular momentum. This kills post-impact oscillation without affecting fall speed or rolling. Soft materials get light global damping (0.5% per frame).
- **Rigid-rigid collisions**: Circle-circle, circle-AABB, and AABB-AABB pairs are resolved with position-based correction, angular response at the contact point, restitution (geometric mean), and Coulomb friction. Runs `contact_iterations` times per substep alongside soft-rigid.

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

### Body Management

```rust
world.remove_body(handle)        // -> bool (true if removed)
world.contains(handle)           // -> bool (is handle still valid?)
world.body_count()               // -> usize (soft + rigid)
world.is_rigid(handle)           // -> bool
```

### Direct Body Access

```rust
world.get_body(handle)           // -> Option<&XPBDSoftBody>
world.get_body_mut(handle)       // -> Option<&mut XPBDSoftBody>
world.get_rigid_body(handle)     // -> Option<&RigidBody>
world.get_rigid_body_mut(handle) // -> Option<&mut RigidBody>
world.get_triangles(handle)      // -> Option<&[u32]>
world.get_collision_groups(handle)                // -> Option<CollisionGroups>
world.set_collision_groups(handle, groups)        // -> bool
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

### Parameter Getters

```rust
world.gravity()              // -> f32
world.ground()               // -> Option<f32>
world.ground_friction()      // -> f32
world.ground_restitution()   // -> f32
world.contact_iterations()   // -> u32
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
world.get_contact(handle, 0.1)   // -> Option<BodyHandle> (first AABB overlap)
world.are_overlapping(a, b, 0.1) // -> bool (check two specific bodies)
world.get_surface_contact_y(handle, 0.1) // -> Option<f32> (Y of contact surface below)
world.get_kinetic_energy(handle) // -> Option<f32>
world.total_kinetic_energy()     // -> f32
world.sleep_if_resting(handle, threshold) // -> bool (zero velocity if KE < threshold)
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

Soft body render positions are automatically inflated outward from center by `render_inflation` (default: 0.075) to visually hide the collision gap.

```rust
// All soft bodies (raw positions, no inflation)
world.get_render_data()                           // -> Vec<(&[f32], &[u32])>

// Single body (positions inflated for rendering)
world.get_body_render_data(handle)             // -> Option<(Vec<f32>, &[u32])>

// Single body with interpolation (also inflated)
world.get_body_render_data_interpolated(handle, alpha) // -> Option<(Vec<f32>, &[u32])>
world.get_position_interpolated(handle, alpha)         // -> Option<Vec2>

// Rigid body interpolated
world.get_rigid_body_render_data_interpolated(handle, alpha)
    // -> Option<(Vec2, Vec2, f32)>  (position, half_extents, rotation)

// Control inflation
world.set_render_inflation(0.075);  // default: 0.075
world.render_inflation()            // -> f32
```

### Iteration

```rust
world.handles()    // -> impl Iterator<Item = BodyHandle>
world.iter()       // -> impl Iterator<Item = (BodyHandle, &XPBDSoftBody)>
world.iter_mut()   // -> impl Iterator<Item = (BodyHandle, &mut XPBDSoftBody)>
world.iter_rigid() // -> impl Iterator<Item = (BodyHandle, &RigidBody)>
```

## BodyHandle

Unique identifier for a physics body. Implements `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `Hash`.

```rust
handle.index()  // -> usize (raw index for advanced usage)
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

Public fields: `material: Material`, `collision_groups: CollisionGroups`, `position: Vec2`, `velocity: Vec2`.

## Material

Predefined materials and custom creation.

| Preset | Density | Edge Compliance | Area Compliance | Description |
|--------|---------|-----------------|-----------------|-------------|
| `Material::SLIME` | 800 | 1e-5 | 1e-4 | Ultra-soft, blobby |
| `Material::JELLO` | 1000 | 0.0 | 1e-6 | Soft, jiggly |
| `Material::RUBBER` | 1100 | 0.0 | 1e-7 | Bouncy (default) |
| `Material::WOOD` | 600 | 0.0 | 1e-8 | Stiff |
| `Material::METAL` | 2000 | 0.0 | 0.0 | Nearly rigid |

```rust
Material::new(density, edge_compliance, area_compliance)
```

Public fields: `density: f32`, `edge_compliance: f32`, `area_compliance: f32`.

## CollisionGroups

Bitfield-based collision filtering.

```rust
CollisionGroups::ALL   // collide with everything (default)
CollisionGroups::NONE  // no collisions
CollisionGroups::new(membership, filter)
groups.can_collide(&other) // bidirectional check
```

Public fields: `membership: u32`, `filter: u32`.

## Mesh

Vertex/triangle data for soft bodies.

```rust
Mesh::new(vertices, triangles)
Mesh::with_uvs(vertices, triangles, uvs)
mesh.vertex_count()
mesh.uvs_or_default()
mesh.ensure_boundary_edges()  // compute & cache boundary edges for shadow casting
```

Public fields: `vertices: Vec<f32>`, `triangles: Vec<u32>`, `uvs: Option<Vec<f32>>`, `boundary_edges: Option<Vec<(u32, u32)>>`.

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

// Wireframes
create_ring_wireframe(segments, radial_divisions)   // -> Vec<u32> (line index pairs)
create_radial_wireframe(segments, rings)             // -> Vec<u32> (line index pairs)

// Boundary detection
compute_boundary_edges_from_triangles(&triangles)    // -> Vec<(u32, u32)>

// Utilities
offset_vertices(&mut vertices, dx, dy)
```

## RigidBody

Non-deformable physics body with circle or AABB collider. Position and velocity fields use `Vec2`.

```rust
RigidBodyConfig::new()
    .as_circle(radius)           // or .as_aabb(half_w, half_h)
    .with_collider(collider)     // set collider directly
    .with_density(1.0)
    .at_position(x, y)
    .with_rotation(angle)
    .with_velocity(vx, vy)
    .with_angular_velocity(omega)
    .with_friction(0.8)          // default: 0.8
    .with_restitution(0.3)
    .as_kinematic()              // immovable
```

### RigidBodyConfig Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `collider` | `Collider` | Circle r=1.0 | Collider shape |
| `density` | `f32` | 1000.0 | Density in kg/m^2 |
| `position` | `Vec2` | ZERO | Initial position |
| `rotation` | `f32` | 0.0 | Initial rotation (radians) |
| `velocity` | `Vec2` | ZERO | Initial velocity |
| `angular_velocity` | `f32` | 0.0 | Initial angular velocity |
| `is_kinematic` | `bool` | false | If true, not affected by physics |
| `friction` | `f32` | 0.8 | Friction coefficient (0-1) |
| `restitution` | `f32` | 0.3 | Bounciness (0-1) |

### RigidBody Fields

| Field | Type | Description |
|-------|------|-------------|
| `position` | `Vec2` | Current position |
| `rotation` | `f32` | Current rotation (radians) |
| `linear_velocity` | `Vec2` | Linear velocity |
| `angular_velocity` | `f32` | Angular velocity (radians/sec) |
| `prev_position` | `Vec2` | Previous position (for XPBD integration) |
| `prev_rotation` | `f32` | Previous rotation |
| `inv_mass` | `f32` | Inverse mass (0 = kinematic) |
| `inv_inertia` | `f32` | Inverse moment of inertia (0 = kinematic) |
| `collider` | `Collider` | Collider shape |
| `friction` | `f32` | Friction coefficient |
| `restitution` | `f32` | Bounciness |

### RigidBody Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `(config: &RigidBodyConfig) -> Self` | Create from configuration |
| `is_kinematic` | `(&self) -> bool` | Check if body is kinematic |
| `get_aabb` | `(&self) -> (f32, f32, f32, f32)` | Bounding box (min_x, min_y, max_x, max_y) |
| `get_center` | `(&self) -> Vec2` | Center position |
| `apply_impulse` | `(&mut self, ix, iy)` | Impulse at center of mass |
| `apply_impulse_at_point` | `(&mut self, ix, iy, px, py)` | Impulse at world point (creates torque) |
| `apply_angular_impulse` | `(&mut self, angular_impulse)` | Angular impulse |
| `pre_solve` | `(&mut self, dt, gravity)` | Store state, integrate velocity |
| `post_solve` | `(&mut self, dt)` | Derive velocities from position change |
| `solve_ground_collision` | `(&mut self, ground_y, friction, restitution)` | Ground collision response |
| `contains_point` | `(&self, px, py) -> Option<(depth, nx, ny)>` | Point-in-collider test |
| `nearest_surface_dist` | `(&self, px, py) -> Option<(dist, nx, ny)>` | Distance to surface (outside only) |
| `query_point` | `(&self, px, py, threshold, cos_r, sin_r) -> PointQuery` | Unified point query |

### Collider

```rust
Collider::circle(radius)
Collider::aabb(half_width, half_height)
collider.half_extents()  // -> Vec2
collider.get_aabb(position, rotation)                  // -> (min_x, min_y, max_x, max_y)
collider.get_aabb_with_trig(position, abs_cos, abs_sin) // same, with pre-computed trig
```

### PointQuery

Unified result from `RigidBody::query_point`, combining penetration and near-surface checks in a single geometric pass.

```rust
use unison_physics::rigid::PointQuery;

let cos_r = body.rotation.cos();
let sin_r = body.rotation.sin();
match body.query_point(px, py, contact_threshold, cos_r, sin_r) {
    PointQuery::Penetrating(depth, nx, ny) => { /* inside collider */ }
    PointQuery::NearSurface(dist, nx, ny)  => { /* outside but within threshold */ }
    PointQuery::Far                        => { /* too far away */ }
}
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

## Compute

Platform-agnostic compute abstraction for parallel physics. Data is kept in Structure of Arrays (SoA) format for SIMD/GPU efficiency.

### ComputeBackend Trait

```rust
use unison_physics::compute::ComputeBackend;

trait ComputeBackend {
    fn integrate_gravity(pos, vel, prev_pos, gravity, dt, inv_mass);
    fn derive_velocities(pos, prev_pos, vel, dt);
    fn solve_distance_constraints_batch(pos, constraints, inv_mass, alpha);
}
```

| Implementation | Description |
|----------------|-------------|
| `ScalarBackend` | Default, works everywhere |
| `SimdBackend` | SIMD via `wide` crate (feature = "simd") |

### GpuComputeBackend Trait

```rust
use unison_physics::compute::GpuComputeBackend;

trait GpuComputeBackend {
    fn upload(&mut self, pos, vel, inv_mass);
    fn solve_constraints(&mut self, iterations, dt);
    fn download(&self, pos, vel);
}
```

## SimulationTracer

Debug tool for capturing frame snapshots. `FrameTrace` fields `centroid` and `linear_velocity` are `Vec2`. `TraceStatistics` fields `start_centroid` and `end_centroid` are `Vec2`.

```rust
let mut tracer = SimulationTracer::new(120); // keep last 120 frames
tracer.enable();
tracer.disable();
tracer.clear();
tracer.capture_frame(frame, dt, positions, velocities, triangles, rest_areas);
tracer.traces()           // -> &VecDeque<FrameTrace>
tracer.last_n(10)         // -> impl Iterator<Item = &FrameTrace>
tracer.get_frame(42)      // -> Option<&FrameTrace> (by frame number)
tracer.statistics()       // -> TraceStatistics
tracer.detect_anomalies() // -> Vec<String>
tracer.print_summary(10);
tracer.to_csv()           // -> String
```

### FrameTrace

Per-frame simulation snapshot.

| Field | Type | Description |
|-------|------|-------------|
| `frame` | `u32` | Frame number |
| `time` | `f32` | Accumulated time |
| `centroid` | `Vec2` | Center of mass |
| `bounding_box` | `[f32; 4]` | min_x, max_x, min_y, max_y |
| `orientation` | `f32` | Principal axis angle (radians) |
| `linear_velocity` | `Vec2` | Average velocity |
| `angular_velocity` | `f32` | Estimated rotation rate |
| `max_velocity` | `f32` | Maximum vertex speed |
| `min_j`, `max_j`, `avg_j` | `f32` | Triangle area ratios (J) |
| `inverted_triangles` | `u32` | Count of inverted triangles |
| `kinetic_energy` | `f32` | Total kinetic energy |
| `fastest_vertex` | `(usize, f32)` | Index and speed of fastest vertex |
| `lowest_vertex` | `(usize, f32)` | Index and Y of lowest vertex |
| `markers` | `Vec<(String, f32)>` | Custom key-value markers |

```rust
trace.add_marker("event", 1.0);
```

### TraceStatistics

| Field | Type | Description |
|-------|------|-------------|
| `num_frames` | `usize` | Total frames traced |
| `total_time` | `f32` | Total simulated time |
| `min_j_ever` | `f32` | Minimum area ratio across all frames |
| `max_j_ever` | `f32` | Maximum area ratio across all frames |
| `max_velocity_ever` | `f32` | Peak vertex speed |
| `max_angular_velocity_ever` | `f32` | Peak angular velocity |
| `total_inverted_frames` | `u32` | Sum of inverted triangles |
| `start_centroid` | `Vec2` | First-frame centroid |
| `end_centroid` | `Vec2` | Last-frame centroid |

```rust
stats.print();       // print formatted summary
stats.is_stable()    // -> bool (no inversions, no explosion)
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
- `width_ratio`, `height_ratio` -- dimension preservation (1.0 = perfect)
- `aspect_ratio`, `original_aspect_ratio` -- current and original aspect ratios
- `min_edge_ratio`, `max_edge_ratio`, `edge_ratio_stddev` -- edge length health
- `severely_compressed_edges`, `severely_stretched_edges` -- edge extremes
- `min_area_ratio`, `max_area_ratio`, `inverted_triangles`, `collapsed_triangles` -- triangle health
- `max_boundary_angle_deviation` -- jagged edge detection
- `convexity_ratio` -- ratio of mesh area to bounding box area
- `min_vertex_separation` -- vertex clustering detection
- `center: (f32, f32)` -- center of mass position
- `kinetic_energy`, `max_speed` -- energy and velocity metrics

### ShapeBaseline

Captures original shape metrics for comparison.

| Field | Type | Description |
|-------|------|-------------|
| `width` | `f32` | Original width |
| `height` | `f32` | Original height |
| `aspect_ratio` | `f32` | Original aspect ratio |
| `total_edge_length` | `f32` | Sum of all edge rest lengths |
| `avg_edge_length` | `f32` | Average edge rest length |
| `total_area` | `f32` | Sum of all triangle rest areas |
| `num_edges` | `usize` | Edge constraint count |
| `num_triangles` | `usize` | Area constraint count |
| `num_verts` | `usize` | Vertex count |

### HealthTolerance

Configurable thresholds for mesh health checks.

| Field | strict() | soft_material() | during_collision() |
|-------|----------|------------------|--------------------|
| `min_dimension_ratio` | 0.70 | 0.55 | 0.40 |
| `min_edge_ratio` | 0.30 | 0.20 | 0.15 |
| `max_edge_ratio` | 2.0 | 2.5 | 3.0 |
| `max_edge_stddev` | 0.25 | 0.35 | 0.40 |
| `max_severely_compressed` | 2 | 5 | 10 |
| `min_area_ratio` | 0.05 | 0.0 | -0.05 |
| `max_collapsed_triangles` | 3 | 5 | 10 |
| `min_vertex_separation` | 0.10 | 0.05 | 0.03 |

### ForensicSimulation

Runs a simulation with periodic forensic capture.

```rust
let sim = ForensicSimulation::run_single(
    &mut body, &baseline, frames, substeps, dt,
    gravity, ground_y, friction, restitution, capture_interval,
);
sim.final_snapshot()                    // -> Option<&MeshForensics>
sim.worst_issues(&HealthTolerance::strict()) // -> Vec<(frame, Vec<String>)>
sim.print_summary();
```

Public fields: `snapshots: Vec<(u32, MeshForensics)>`, `baseline: ShapeBaseline`.

## Low-Level: XPBDSoftBody

Direct access to the XPBD solver. Most users should use `PhysicsWorld` instead.

### Public Fields

| Field | Type | Description |
|-------|------|-------------|
| `pos` | `Vec<f32>` | Current positions [x0, y0, x1, y1, ...] |
| `prev_pos` | `Vec<f32>` | Previous positions (for velocity) |
| `vel` | `Vec<f32>` | Velocities [vx0, vy0, ...] |
| `inv_mass` | `Vec<f32>` | Inverse masses (0 = fixed) |
| `force_accum` | `Vec<f32>` | Force accumulator [fx0, fy0, ...], cleared each step |
| `torque_accum` | `f32` | Torque accumulator, cleared each step |
| `edge_constraints` | `Vec<EdgeConstraint>` | Edge distance constraints |
| `area_constraints` | `Vec<AreaConstraint>` | Triangle area constraints |
| `edge_compliance` | `f32` | Edge constraint compliance |
| `area_compliance` | `f32` | Area constraint compliance |
| `triangles` | `Vec<u32>` | Triangle indices (for rendering) |
| `num_verts` | `usize` | Vertex count |

### EdgeConstraint

```rust
pub struct EdgeConstraint {
    pub v0: usize,        // First vertex index
    pub v1: usize,        // Second vertex index
    pub rest_length: f32,  // Rest length
}
```

### AreaConstraint

```rust
pub struct AreaConstraint {
    pub v0: usize,
    pub v1: usize,
    pub v2: usize,
    pub rest_area: f32,
}
```

### Methods

| Method | Description |
|--------|-------------|
| `new(vertices, triangles, density, edge_compliance, area_compliance)` | Create from mesh data |
| `from_material(vertices, triangles, young_modulus, density)` | Create from FEM-style parameters |
| `pre_solve(dt, gravity)` | Apply forces, predict positions |
| `post_solve(dt)` | Derive velocities from position change |
| `solve_constraints(dt) -> f32` | Solve edge and area constraints |
| `clear_accumulators()` | Zero out force and torque accumulators |
| `apply_damping(damping)` | Global velocity damping |
| `apply_internal_damping(damping)` | Deformation-only damping (preserves momentum) |
| `solve_ground_collision(ground_y, dt)` | Simple ground collision |
| `solve_ground_collision_with_friction(ground_y, friction, restitution)` | Ground with friction |
| `solve_terrain_collision(height_at, normal_at, friction, restitution)` | Variable-height terrain |
| `substep_pre(dt, gravity, ground_y)` | Pre-solve + ground + constraints |
| `substep_pre_with_friction(dt, gravity, ground_y, friction, restitution)` | Pre-solve + friction ground |
| `substep_pre_with_friction_iters(dt, gravity, ground_y, friction, restitution, pre_iters, post_iters)` | With iteration control |
| `substep_pre_with_terrain(dt, gravity, height_at, normal_at, friction, restitution)` | Pre-solve + terrain |
| `substep_pre_with_terrain_iters(dt, gravity, height_at, normal_at, friction, restitution, pre_iters, post_iters)` | Terrain with iteration control |
| `substep_post(dt)` | Post-solve step |
| `substep(dt, gravity, ground_y)` | Complete substep (pre + post) |
| `collide_with_body(other, min_dist) -> u32` | Position-based collision |
| `get_center() -> (f32, f32)` | Center of mass |
| `get_aabb() -> (min_x, min_y, max_x, max_y)` | Bounding box |
| `get_lowest_y() -> f32` | Lowest vertex Y |
| `get_kinetic_energy() -> f32` | Total kinetic energy |
| `get_max_velocity() -> f32` | Peak vertex speed |
| `get_aspect_ratio() -> f32` | Width / height ratio |
| `sleep_if_resting(ke_threshold) -> bool` | Zero velocity if energy below threshold |

## Low-Level: CollisionSystem

Spatial-hash based broad-phase collision detection.

```rust
let mut collision = CollisionSystem::new(min_dist);
collision.prepare(&bodies);     // broad phase + spatial hash build
collision.resolve_collisions(&mut bodies);           // narrow phase (3 iters default)
collision.resolve_collisions_n(&mut bodies, 5);      // narrow phase with custom iter count
collision.resolve_collisions_with_kinematic(&mut bodies, &is_kinematic); // with kinematic support
collision.resolve_collisions_with_kinematic_n(&mut bodies, &is_kinematic, 5); // kinematic + custom iters
collision.solve_collisions(&mut bodies);             // legacy: prepare + resolve in one call
```

### Diagnostic Stats

| Field | Type | Description |
|-------|------|-------------|
| `stats_candidates` | `u32` | Candidate pairs evaluated |
| `stats_cached_edges` | `u32` | Edges in spatial hash |
| `stats_overlapping_pairs` | `u32` | Broad-phase AABB overlaps |
| `stats_collisions_found` | `u32` | Actual collisions resolved |
| `stats_iterations_run` | `u32` | Narrow-phase iterations executed |

## Low-Level: SpatialHash

Flat-grid spatial hash for O(1) neighbor queries in collision detection. Uses a dense grid for cache-friendly lookups.

```rust
let mut hash = SpatialHash::new(cell_size);
hash.clear();
hash.insert(body_idx, edge_idx, x, y);
hash.build();                          // must call after all inserts
hash.query_neighbors_into(x, y, &mut results); // fills Vec with (body_idx, edge_idx) pairs
```
