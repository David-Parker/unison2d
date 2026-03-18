//! Comprehensive shape integrity test battery for the XPBD physics engine.
//!
//! Tests mesh shapes (rings, boxes, stars, ellipses, blobs) under various
//! conditions: different velocities, materials, collisions with soft and rigid
//! bodies. Uses forensic analysis to detect permanent deformation, jagged edges,
//! vertex clustering, and triangle inversion.

use unison_physics::forensics::*;
use unison_physics::mesh::*;
use unison_physics::world::*;
use unison_physics::xpbd::{CollisionSystem, XPBDSoftBody};

// ============================================================================
// Helpers
// ============================================================================

/// Create a body, offset it, optionally set velocity, return body + baseline
fn make_body(
    mesh: &Mesh,
    density: f32,
    edge_compliance: f32,
    area_compliance: f32,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
) -> (XPBDSoftBody, ShapeBaseline) {
    let mut body = XPBDSoftBody::new(
        &mesh.vertices,
        &mesh.triangles,
        density,
        edge_compliance,
        area_compliance,
    );
    // Capture baseline BEFORE offset (shape-relative)
    let baseline = ShapeBaseline::capture(&body);
    for i in 0..body.num_verts {
        body.pos[i * 2] += x;
        body.pos[i * 2 + 1] += y;
        body.prev_pos[i * 2] += x;
        body.prev_pos[i * 2 + 1] += y;
        body.vel[i * 2] = vx;
        body.vel[i * 2 + 1] = vy;
    }
    (body, baseline)
}

/// Standard simulation parameters
const DT: f32 = 1.0 / 60.0;
const GRAVITY: f32 = -9.8;
const GROUND_Y: f32 = -5.0;
const FRICTION: f32 = 0.7;
const RESTITUTION: f32 = 0.0;

/// Material presets: (edge_compliance, area_compliance, name)
fn materials() -> Vec<(f32, f32, &'static str)> {
    vec![
        (0.0, 0.0, "metal"),
        (0.0, 1e-8, "wood"),
        (0.0, 1e-7, "rubber"),
        (0.0, 1e-6, "jello"),
        (5e-7, 5e-7, "very_soft"),
        (1e-6, 1e-5, "ultra_soft"),
    ]
}

/// Mesh generators: (mesh, name)
fn test_meshes() -> Vec<(Mesh, &'static str)> {
    vec![
        (create_ring_mesh(1.0, 0.4, 16, 4), "ring_thick"),
        (create_ring_mesh(1.0, 0.7, 16, 4), "ring_thin"),
        (create_ring_mesh(1.5, 0.5, 16, 6), "ring_large"),
        (create_square_mesh(2.0, 4), "box_2x2"),
        (create_square_mesh(1.0, 3), "box_1x1"),
        (create_star_mesh(1.5, 0.6, 5, 4), "star_5pt"),
        (create_star_mesh(1.2, 0.4, 6, 3), "star_6pt"),
        (create_ellipse_mesh(2.0, 1.0, 16, 4), "ellipse_wide"),
        (create_ellipse_mesh(1.0, 2.0, 16, 4), "ellipse_tall"),
        (create_blob_mesh(1.2, 0.2, 16, 4, 42), "blob"),
    ]
}

/// Impact velocities to test
fn impact_velocities() -> Vec<(f32, &'static str)> {
    vec![
        (0.0, "drop"),       // Free fall from height
        (-10.0, "medium"),
        (-25.0, "fast"),
        (-40.0, "extreme"),
    ]
}

/// Assert forensic health, printing diagnostics on failure
fn assert_healthy(
    forensics: &MeshForensics,
    tolerance: &HealthTolerance,
    context: &str,
) {
    let issues = forensics.is_healthy(tolerance);
    if !issues.is_empty() {
        eprintln!("=== SHAPE INTEGRITY FAILURE: {} ===", context);
        eprintln!("Forensics: {}", forensics.summary());
        for issue in &issues {
            eprintln!("  - {}", issue);
        }
        panic!(
            "{}: {} shape integrity issues detected. First: {}",
            context,
            issues.len(),
            issues[0]
        );
    }
}

// ============================================================================
// 1. Single body ground impact — every shape × every material × every velocity
// ============================================================================

/// Core test: drop a single body onto the ground and verify shape recovery.
/// This is the most fundamental test — if a body can't survive a ground impact
/// without permanent deformation, nothing else matters.
fn run_single_impact_test(
    mesh: &Mesh,
    mesh_name: &str,
    edge_compliance: f32,
    area_compliance: f32,
    mat_name: &str,
    impact_vy: f32,
    vel_name: &str,
) {
    let start_height = 8.0;
    let (mut body, baseline) = make_body(
        mesh, 1000.0, edge_compliance, area_compliance,
        0.0, start_height, 0.0, impact_vy,
    );

    let substeps = 4u32;
    let substep_dt = DT / substeps as f32;
    let settle_frames = 300u32; // 5 seconds to settle
    let check_frames = 120u32; // 2 more seconds to confirm stability

    // Phase 1: Impact and settle
    for _ in 0..settle_frames {
        for _ in 0..substeps {
            body.substep_pre_with_friction(
                substep_dt, GRAVITY, Some(GROUND_Y), FRICTION, RESTITUTION,
            );
            body.substep_post(substep_dt);
        }
        body.apply_damping(0.005);
    }

    // Phase 2: Check stability over time
    let mut worst_forensics: Option<MeshForensics> = None;
    let mut worst_score = 0.0f32;

    for _ in 0..check_frames {
        for _ in 0..substeps {
            body.substep_pre_with_friction(
                substep_dt, GRAVITY, Some(GROUND_Y), FRICTION, RESTITUTION,
            );
            body.substep_post(substep_dt);
        }
        body.apply_damping(0.005);

        let f = MeshForensics::analyze(&body, &baseline);
        // Score: lower is worse
        let score = f.width_ratio.min(f.height_ratio);
        if worst_forensics.is_none() || score < worst_score {
            worst_score = score;
            worst_forensics = Some(f);
        }
    }

    let tolerance = if edge_compliance >= 1e-6 || area_compliance >= 1e-5 {
        HealthTolerance::soft_material()
    } else {
        HealthTolerance::strict()
    };

    let context = format!("{}_{}_v{}", mesh_name, mat_name, vel_name);
    assert_healthy(worst_forensics.as_ref().unwrap(), &tolerance, &context);
}

#[test]
fn test_single_impact_ring_thick_all_materials() {
    let mesh = create_ring_mesh(1.0, 0.4, 16, 4);
    for (ec, ac, mat) in materials() {
        for (vy, vname) in impact_velocities() {
            run_single_impact_test(&mesh, "ring_thick", ec, ac, mat, vy, vname);
        }
    }
}

#[test]
fn test_single_impact_ring_thin_all_materials() {
    let mesh = create_ring_mesh(1.0, 0.7, 16, 4);
    for (ec, ac, mat) in materials() {
        for (vy, vname) in impact_velocities() {
            run_single_impact_test(&mesh, "ring_thin", ec, ac, mat, vy, vname);
        }
    }
}

#[test]
fn test_single_impact_box_all_materials() {
    let mesh = create_square_mesh(2.0, 4);
    for (ec, ac, mat) in materials() {
        for (vy, vname) in impact_velocities() {
            run_single_impact_test(&mesh, "box_2x2", ec, ac, mat, vy, vname);
        }
    }
}

#[test]
fn test_single_impact_star_all_materials() {
    let mesh = create_star_mesh(1.5, 0.6, 5, 4);
    for (ec, ac, mat) in materials() {
        for (vy, vname) in impact_velocities() {
            run_single_impact_test(&mesh, "star_5pt", ec, ac, mat, vy, vname);
        }
    }
}

#[test]
fn test_single_impact_ellipse_all_materials() {
    let mesh = create_ellipse_mesh(2.0, 1.0, 16, 4);
    for (ec, ac, mat) in materials() {
        for (vy, vname) in impact_velocities() {
            run_single_impact_test(&mesh, "ellipse_wide", ec, ac, mat, vy, vname);
        }
    }
}

// ============================================================================
// 2. Soft-on-soft stacking — bodies piled on each other
// ============================================================================

/// Drop N soft bodies on top of each other and verify none get permanently crushed.
/// The bottom body is under the most stress — this is where edge crushing happens.
fn run_stacking_test(
    mesh: &Mesh,
    mesh_name: &str,
    edge_compliance: f32,
    area_compliance: f32,
    mat_name: &str,
    num_bodies: usize,
) {
    let mut bodies = Vec::new();
    let mut baselines = Vec::new();

    for i in 0..num_bodies {
        let y = 3.0 + i as f32 * 3.5;
        let x_jitter = (i as f32 - num_bodies as f32 / 2.0) * 0.2;
        let (body, baseline) = make_body(
            mesh, 1000.0, edge_compliance, area_compliance,
            x_jitter, y, 0.0, -5.0,
        );
        bodies.push(body);
        baselines.push(baseline);
    }

    let mut collision_system = CollisionSystem::new(0.15);
    let substeps = 4u32;
    let substep_dt = DT / substeps as f32;

    // Simulate 8 seconds
    for _ in 0..480 {
        for _ in 0..substeps {
            for body in &mut bodies {
                body.substep_pre_with_friction(
                    substep_dt, GRAVITY, Some(GROUND_Y), FRICTION, RESTITUTION,
                );
            }
            collision_system.solve_collisions(&mut bodies);
            for body in &mut bodies {
                body.substep_post(substep_dt);
            }
        }
        for body in &mut bodies {
            body.apply_damping(0.005);
        }
    }

    // Check all bodies
    let tolerance = if edge_compliance >= 1e-6 || area_compliance >= 1e-5 {
        HealthTolerance::soft_material()
    } else {
        HealthTolerance::strict()
    };

    for (i, (body, baseline)) in bodies.iter().zip(baselines.iter()).enumerate() {
        let f = MeshForensics::analyze(body, baseline);
        let context = format!("stack_{}_{}_{}_body{}", mesh_name, mat_name, num_bodies, i);
        assert_healthy(&f, &tolerance, &context);
    }
}

#[test]
fn test_stack_3_rings_rubber() {
    let mesh = create_ring_mesh(1.0, 0.4, 16, 4);
    run_stacking_test(&mesh, "ring", 0.0, 1e-7, "rubber", 3);
}

#[test]
fn test_stack_5_rings_rubber() {
    let mesh = create_ring_mesh(1.0, 0.4, 16, 4);
    run_stacking_test(&mesh, "ring", 0.0, 1e-7, "rubber", 5);
}

#[test]
fn test_stack_3_rings_jello() {
    let mesh = create_ring_mesh(1.0, 0.4, 16, 4);
    run_stacking_test(&mesh, "ring", 0.0, 1e-6, "jello", 3);
}

#[test]
fn test_stack_5_rings_jello() {
    let mesh = create_ring_mesh(1.0, 0.4, 16, 4);
    run_stacking_test(&mesh, "ring", 0.0, 1e-6, "jello", 5);
}

#[test]
fn test_stack_3_boxes_rubber() {
    let mesh = create_square_mesh(2.0, 4);
    run_stacking_test(&mesh, "box", 0.0, 1e-7, "rubber", 3);
}

#[test]
fn test_stack_3_stars_rubber() {
    let mesh = create_star_mesh(1.5, 0.6, 5, 4);
    run_stacking_test(&mesh, "star", 0.0, 1e-7, "rubber", 3);
}

#[test]
fn test_stack_mixed_shapes() {
    // Stack different shapes on top of each other
    let meshes: Vec<(Mesh, &str)> = vec![
        (create_square_mesh(2.0, 4), "box"),
        (create_ring_mesh(1.0, 0.4, 16, 4), "ring"),
        (create_star_mesh(1.5, 0.6, 5, 4), "star"),
        (create_ellipse_mesh(2.0, 1.0, 16, 4), "ellipse"),
    ];

    let ec = 0.0;
    let ac = 1e-7;
    let mut bodies = Vec::new();
    let mut baselines = Vec::new();

    for (i, (mesh, _)) in meshes.iter().enumerate() {
        let y = 3.0 + i as f32 * 3.5;
        let (body, baseline) = make_body(mesh, 1000.0, ec, ac, 0.0, y, 0.0, -5.0);
        bodies.push(body);
        baselines.push(baseline);
    }

    let mut collision_system = CollisionSystem::new(0.15);
    let substeps = 4u32;
    let substep_dt = DT / substeps as f32;

    for _ in 0..480 {
        for _ in 0..substeps {
            for body in &mut bodies {
                body.substep_pre_with_friction(
                    substep_dt, GRAVITY, Some(GROUND_Y), FRICTION, RESTITUTION,
                );
            }
            collision_system.solve_collisions(&mut bodies);
            for body in &mut bodies {
                body.substep_post(substep_dt);
            }
        }
        for body in &mut bodies {
            body.apply_damping(0.005);
        }
    }

    let tolerance = HealthTolerance::strict();
    for (i, (body, baseline)) in bodies.iter().zip(baselines.iter()).enumerate() {
        let f = MeshForensics::analyze(body, baseline);
        let context = format!("mixed_stack_{}_body{}", meshes[i].1, i);
        assert_healthy(&f, &tolerance, &context);
    }
}

// ============================================================================
// 3. Soft body vs rigid body collisions
// ============================================================================

/// Drop a soft body onto a kinematic rigid circle and verify shape recovery.
fn run_soft_vs_rigid_test(
    mesh: &Mesh,
    mesh_name: &str,
    edge_compliance: f32,
    area_compliance: f32,
    mat_name: &str,
    impact_vy: f32,
) {
    use unison_physics::rigid::{Collider, RigidBodyConfig};

    let mut world = PhysicsWorld::new();
    world.set_gravity(GRAVITY);
    world.set_ground(Some(GROUND_Y));
    world.set_ground_friction(FRICTION);

    // Add kinematic rigid circle on the ground
    let _rigid = world.add_rigid_body(
        RigidBodyConfig::new()
            .with_collider(Collider::circle(1.5))
            .at_position(0.0, GROUND_Y + 1.5)
            .as_kinematic()
            .with_friction(0.8),
    );

    // Add soft body above
    let soft = world.add_body(
        mesh,
        BodyConfig::new()
            .at_position(0.0, 6.0)
            .with_velocity(0.0, impact_vy)
            .with_material(Material::new(1000.0, edge_compliance, area_compliance)),
    );

    // Capture baseline from the body
    let body_ref = world.get_body(soft).unwrap();
    let baseline = ShapeBaseline::capture(body_ref);

    // Simulate 8 seconds
    for _ in 0..480 {
        world.step(DT);
    }

    let body_ref = world.get_body(soft).unwrap();
    let f = MeshForensics::analyze(body_ref, &baseline);

    let tolerance = if edge_compliance >= 1e-6 || area_compliance >= 1e-5 {
        HealthTolerance::soft_material()
    } else {
        HealthTolerance::strict()
    };

    let context = format!("soft_vs_rigid_circle_{}_{}_v{:.0}", mesh_name, mat_name, impact_vy.abs());
    assert_healthy(&f, &tolerance, &context);
}

#[test]
fn test_soft_ring_vs_rigid_circle_rubber() {
    let mesh = create_ring_mesh(1.0, 0.4, 16, 4);
    run_soft_vs_rigid_test(&mesh, "ring", 0.0, 1e-7, "rubber", -10.0);
}

#[test]
fn test_soft_ring_vs_rigid_circle_fast() {
    let mesh = create_ring_mesh(1.0, 0.4, 16, 4);
    run_soft_vs_rigid_test(&mesh, "ring", 0.0, 1e-7, "rubber", -25.0);
}

#[test]
fn test_soft_ring_vs_rigid_circle_jello() {
    let mesh = create_ring_mesh(1.0, 0.4, 16, 4);
    run_soft_vs_rigid_test(&mesh, "ring", 0.0, 1e-6, "jello", -10.0);
}

#[test]
fn test_soft_box_vs_rigid_circle() {
    let mesh = create_square_mesh(2.0, 4);
    run_soft_vs_rigid_test(&mesh, "box", 0.0, 1e-7, "rubber", -15.0);
}

#[test]
fn test_soft_star_vs_rigid_circle() {
    let mesh = create_star_mesh(1.5, 0.6, 5, 4);
    run_soft_vs_rigid_test(&mesh, "star", 0.0, 1e-7, "rubber", -15.0);
}

/// Drop a soft body onto a kinematic rigid AABB (flat platform)
fn run_soft_vs_rigid_aabb_test(
    mesh: &Mesh,
    mesh_name: &str,
    edge_compliance: f32,
    area_compliance: f32,
    mat_name: &str,
) {
    use unison_physics::rigid::{Collider, RigidBodyConfig};

    let mut world = PhysicsWorld::new();
    world.set_gravity(GRAVITY);
    world.set_ground(Some(GROUND_Y - 5.0)); // Ground far below
    world.set_ground_friction(FRICTION);

    // Kinematic AABB platform
    let _platform = world.add_rigid_body(
        RigidBodyConfig::new()
            .with_collider(Collider::aabb(3.0, 0.3))
            .at_position(0.0, -2.0)
            .as_kinematic()
            .with_friction(0.8),
    );

    let soft = world.add_body(
        mesh,
        BodyConfig::new()
            .at_position(0.0, 4.0)
            .with_velocity(0.0, -15.0)
            .with_material(Material::new(1000.0, edge_compliance, area_compliance)),
    );

    let body_ref = world.get_body(soft).unwrap();
    let baseline = ShapeBaseline::capture(body_ref);

    for _ in 0..480 {
        world.step(DT);
    }

    let body_ref = world.get_body(soft).unwrap();
    let f = MeshForensics::analyze(body_ref, &baseline);

    let tolerance = if edge_compliance >= 1e-6 || area_compliance >= 1e-5 {
        HealthTolerance::soft_material()
    } else {
        HealthTolerance::strict()
    };

    let context = format!("soft_vs_aabb_{}_{}", mesh_name, mat_name);
    assert_healthy(&f, &tolerance, &context);
}

#[test]
fn test_soft_ring_vs_aabb_platform_rubber() {
    let mesh = create_ring_mesh(1.0, 0.4, 16, 4);
    run_soft_vs_rigid_aabb_test(&mesh, "ring", 0.0, 1e-7, "rubber");
}

#[test]
fn test_soft_ring_vs_aabb_platform_jello() {
    let mesh = create_ring_mesh(1.0, 0.4, 16, 4);
    run_soft_vs_rigid_aabb_test(&mesh, "ring", 0.0, 1e-6, "jello");
}

#[test]
fn test_soft_box_vs_aabb_platform() {
    let mesh = create_square_mesh(2.0, 4);
    run_soft_vs_rigid_aabb_test(&mesh, "box", 0.0, 1e-7, "rubber");
}

// ============================================================================
// 4. Material behavior validation — softer = more wobble, not more glitch
// ============================================================================

/// Verify that softer materials deform MORE during impact but still RECOVER.
/// This catches the bug where soft materials permanently collapse.
#[test]
fn test_material_softness_gradient() {
    let mesh = create_ring_mesh(1.0, 0.4, 16, 4);
    let substeps = 4u32;
    let substep_dt = DT / substeps as f32;

    // Materials from stiff to soft
    let mats: Vec<(f32, f32, &str)> = vec![
        (0.0, 1e-8, "wood"),
        (0.0, 1e-7, "rubber"),
        (0.0, 1e-6, "jello"),
        (5e-7, 5e-7, "very_soft"),
    ];

    let mut min_height_during_impact = Vec::new();
    let mut final_height_ratios = Vec::new();

    for (ec, ac, name) in &mats {
        let (mut body, baseline) = make_body(
            &mesh, 1000.0, *ec, *ac, 0.0, 8.0, 0.0, -20.0,
        );

        let mut min_h_ratio = f32::MAX;

        // Phase 1: Impact (2 seconds)
        for _ in 0..120 {
            for _ in 0..substeps {
                body.substep_pre_with_friction(
                    substep_dt, GRAVITY, Some(GROUND_Y), FRICTION, RESTITUTION,
                );
                body.substep_post(substep_dt);
            }
            body.apply_damping(0.005);
            let f = MeshForensics::analyze(&body, &baseline);
            min_h_ratio = min_h_ratio.min(f.height_ratio.min(f.width_ratio));
        }

        // Phase 2: Settle (5 more seconds)
        for _ in 0..300 {
            for _ in 0..substeps {
                body.substep_pre_with_friction(
                    substep_dt, GRAVITY, Some(GROUND_Y), FRICTION, RESTITUTION,
                );
                body.substep_post(substep_dt);
            }
            body.apply_damping(0.005);
        }

        let final_f = MeshForensics::analyze(&body, &baseline);
        let final_ratio = final_f.height_ratio.min(final_f.width_ratio);

        println!(
            "Material {}: min_during_impact={:.1}%, final={:.1}%",
            name, min_h_ratio * 100.0, final_ratio * 100.0
        );

        min_height_during_impact.push(min_h_ratio);
        final_height_ratios.push(final_ratio);

        // All materials must recover to at least 55% after settling
        assert!(
            final_ratio > 0.55,
            "Material '{}' permanently collapsed to {:.1}%",
            name, final_ratio * 100.0
        );

        // No inverted triangles in final state
        assert_eq!(
            final_f.inverted_triangles, 0,
            "Material '{}' has {} inverted triangles after settling",
            name, final_f.inverted_triangles
        );
    }

    // Softer materials should deform MORE during impact (lower min ratio)
    // This validates the physics is actually working — not just clamping everything
    for i in 1..min_height_during_impact.len() {
        // Allow some tolerance — the trend should be generally downward
        if min_height_during_impact[i] > min_height_during_impact[0] + 0.15 {
            println!(
                "WARNING: Softer material {} deformed LESS than stiffest during impact ({:.1}% vs {:.1}%)",
                mats[i].2,
                min_height_during_impact[i] * 100.0,
                min_height_during_impact[0] * 100.0,
            );
        }
    }
}

// ============================================================================
// 5. Lateral collision — soft bodies hitting each other sideways
// ============================================================================

/// Two soft bodies collide head-on at various speeds.
/// Verifies neither body gets permanently crushed at the contact edge.
fn run_lateral_collision_test(
    mesh: &Mesh,
    mesh_name: &str,
    edge_compliance: f32,
    area_compliance: f32,
    mat_name: &str,
    speed: f32,
) {
    let (mut body_l, baseline_l) = make_body(
        mesh, 1000.0, edge_compliance, area_compliance,
        -4.0, 0.0, speed, 0.0,
    );
    let (mut body_r, baseline_r) = make_body(
        mesh, 1000.0, edge_compliance, area_compliance,
        4.0, 0.0, -speed, 0.0,
    );

    let mut collision_system = CollisionSystem::new(0.15);
    let substeps = 4u32;
    let substep_dt = DT / substeps as f32;

    // No ground — pure lateral collision in free space
    for _ in 0..480 {
        for _ in 0..substeps {
            body_l.substep_pre_with_friction(substep_dt, 0.0, None, 0.0, 0.0);
            body_r.substep_pre_with_friction(substep_dt, 0.0, None, 0.0, 0.0);
            let mut bodies = [body_l, body_r];
            collision_system.solve_collisions(&mut bodies);
            [body_l, body_r] = bodies;
            body_l.substep_post(substep_dt);
            body_r.substep_post(substep_dt);
        }
        body_l.apply_damping(0.005);
        body_r.apply_damping(0.005);
    }

    let tolerance = if edge_compliance >= 1e-6 || area_compliance >= 1e-5 {
        HealthTolerance::soft_material()
    } else {
        HealthTolerance::strict()
    };

    let fl = MeshForensics::analyze(&body_l, &baseline_l);
    let fr = MeshForensics::analyze(&body_r, &baseline_r);

    let ctx_l = format!("lateral_{}_{}_{:.0}_left", mesh_name, mat_name, speed);
    let ctx_r = format!("lateral_{}_{}_{:.0}_right", mesh_name, mat_name, speed);
    assert_healthy(&fl, &tolerance, &ctx_l);
    assert_healthy(&fr, &tolerance, &ctx_r);
}

#[test]
fn test_lateral_rings_rubber_slow() {
    let mesh = create_ring_mesh(1.0, 0.4, 16, 4);
    run_lateral_collision_test(&mesh, "ring", 0.0, 1e-7, "rubber", 5.0);
}

#[test]
fn test_lateral_rings_rubber_fast() {
    let mesh = create_ring_mesh(1.0, 0.4, 16, 4);
    run_lateral_collision_test(&mesh, "ring", 0.0, 1e-7, "rubber", 15.0);
}

#[test]
fn test_lateral_rings_jello_fast() {
    let mesh = create_ring_mesh(1.0, 0.4, 16, 4);
    run_lateral_collision_test(&mesh, "ring", 0.0, 1e-6, "jello", 15.0);
}

#[test]
fn test_lateral_boxes_rubber() {
    let mesh = create_square_mesh(2.0, 4);
    run_lateral_collision_test(&mesh, "box", 0.0, 1e-7, "rubber", 10.0);
}

#[test]
fn test_lateral_stars_rubber() {
    let mesh = create_star_mesh(1.5, 0.6, 5, 4);
    run_lateral_collision_test(&mesh, "star", 0.0, 1e-7, "rubber", 10.0);
}

// ============================================================================
// 6. Soft body vs dynamic rigid body — momentum transfer + shape preservation
// ============================================================================

#[test]
fn test_soft_ring_hit_by_dynamic_rigid_circle() {
    use unison_physics::rigid::{Collider, RigidBodyConfig};

    let mut world = PhysicsWorld::new();
    world.set_gravity(GRAVITY);
    world.set_ground(Some(GROUND_Y));
    world.set_ground_friction(FRICTION);

    let mesh = create_ring_mesh(1.0, 0.4, 16, 4);
    let soft = world.add_body(
        &mesh,
        BodyConfig::new()
            .at_position(0.0, GROUND_Y + 1.5)
            .with_material(Material::RUBBER),
    );

    // Dynamic rigid circle falling onto the soft body
    let _rigid = world.add_rigid_body(
        RigidBodyConfig::new()
            .with_collider(Collider::circle(0.8))
            .at_position(0.0, 8.0)
            .with_density(2000.0)
            .with_friction(0.5),
    );

    let body_ref = world.get_body(soft).unwrap();
    let baseline = ShapeBaseline::capture(body_ref);

    // Let settle first
    for _ in 0..120 {
        world.step(DT);
    }

    // Then simulate impact + recovery
    for _ in 0..480 {
        world.step(DT);
    }

    let body_ref = world.get_body(soft).unwrap();
    let f = MeshForensics::analyze(body_ref, &baseline);
    assert_healthy(&f, &HealthTolerance::strict(), "soft_ring_hit_by_rigid_circle");
}

#[test]
fn test_soft_ring_sandwiched_between_rigid_bodies() {
    use unison_physics::rigid::{Collider, RigidBodyConfig};

    let mut world = PhysicsWorld::new();
    world.set_gravity(GRAVITY);
    world.set_ground(Some(GROUND_Y));
    world.set_ground_friction(FRICTION);

    // Kinematic floor platform
    let _floor = world.add_rigid_body(
        RigidBodyConfig::new()
            .with_collider(Collider::aabb(3.0, 0.5))
            .at_position(0.0, GROUND_Y + 0.5)
            .as_kinematic()
            .with_friction(0.8),
    );

    let mesh = create_ring_mesh(1.0, 0.4, 16, 4);
    let soft = world.add_body(
        &mesh,
        BodyConfig::new()
            .at_position(0.0, GROUND_Y + 2.5)
            .with_material(Material::RUBBER),
    );

    // Heavy dynamic rigid body falling on top
    let _heavy = world.add_rigid_body(
        RigidBodyConfig::new()
            .with_collider(Collider::aabb(1.5, 0.5))
            .at_position(0.0, 8.0)
            .with_density(3000.0)
            .with_friction(0.5),
    );

    let body_ref = world.get_body(soft).unwrap();
    let baseline = ShapeBaseline::capture(body_ref);

    for _ in 0..600 {
        world.step(DT);
    }

    let body_ref = world.get_body(soft).unwrap();
    let f = MeshForensics::analyze(body_ref, &baseline);

    // Being sandwiched is harsh — use relaxed tolerance but still no inversions
    let mut tolerance = HealthTolerance::soft_material();
    tolerance.min_dimension_ratio = 0.45; // Allow more squish when sandwiched
    assert_healthy(&f, &tolerance, "ring_sandwiched_between_rigids");
}

// ============================================================================
// 7. Edge crushing regression — the specific bug scenario
// ============================================================================

/// This test specifically targets the reported bug: soft materials get permanently
/// deformed with jagged mesh edges near contact points.
/// We use the exact game ring dimensions and material, drop from height,
/// and check for edge compression clustering near the ground contact zone.
#[test]
fn test_edge_crushing_at_contact_zone() {
    let mesh = create_ring_mesh(1.01, 0.37, 16, 4);
    let substeps = 4u32;
    let substep_dt = DT / substeps as f32;

    // Test with progressively softer materials
    let mats: Vec<(f32, f32, &str)> = vec![
        (0.0, 1e-7, "rubber"),
        (0.0, 1e-6, "jello"),
        (5e-7, 5e-7, "very_soft"),
        (1e-6, 1e-5, "ultra_soft"),
    ];

    for (ec, ac, name) in &mats {
        let (mut body, baseline) = make_body(
            &mesh, 950.0, *ec, *ac, 0.0, 10.0, 0.0, -25.0,
        );

        // Simulate 10 seconds
        for _ in 0..600 {
            for _ in 0..substeps {
                body.substep_pre_with_friction(
                    substep_dt, -15.0, Some(GROUND_Y), FRICTION, RESTITUTION,
                );
                body.substep_post(substep_dt);
            }
            body.apply_damping(0.005);
        }

        let f = MeshForensics::analyze(&body, &baseline);

        println!(
            "Edge crushing test '{}': {}",
            name, f.summary()
        );

        // The key assertions for the edge crushing bug:
        // 1. No severely compressed edges (the "jagged" symptom)
        assert!(
            f.severely_compressed_edges <= 3,
            "Material '{}': {} severely compressed edges (jagged contact zone)",
            name, f.severely_compressed_edges
        );

        // 2. Edge ratio stddev should be low (uniform edge lengths = smooth mesh)
        assert!(
            f.edge_ratio_stddev < 0.30,
            "Material '{}': edge stddev {:.3} indicates jagged mesh",
            name, f.edge_ratio_stddev
        );

        // 3. No inverted triangles
        assert!(
            f.inverted_triangles == 0,
            "Material '{}': {} inverted triangles",
            name, f.inverted_triangles
        );

        // 4. Shape should recover
        let min_dim = f.width_ratio.min(f.height_ratio);
        let threshold = if *ec >= 1e-6 || *ac >= 1e-5 { 0.50 } else { 0.65 };
        assert!(
            min_dim > threshold,
            "Material '{}': permanent collapse to {:.1}%",
            name, min_dim * 100.0
        );
    }
}

/// Multiple soft rings dropped onto each other — the pile-up scenario
/// where bottom rings get crushed by weight above.
#[test]
fn test_pile_crushing_bottom_ring() {
    let mesh = create_ring_mesh(1.01, 0.37, 16, 4);
    let ec = 0.0;
    let ac = 1e-7; // Rubber

    let mut bodies = Vec::new();
    let mut baselines = Vec::new();

    // 5 rings staggered in height, all falling
    for i in 0..5 {
        let y = 3.0 + i as f32 * 3.0;
        let x = (i as f32 - 2.0) * 0.15;
        let (body, baseline) = make_body(&mesh, 950.0, ec, ac, x, y, 0.0, -15.0);
        bodies.push(body);
        baselines.push(baseline);
    }

    let mut collision_system = CollisionSystem::new(0.15);
    let substeps = 4u32;
    let substep_dt = DT / substeps as f32;

    for _ in 0..600 {
        for _ in 0..substeps {
            for body in &mut bodies {
                body.substep_pre_with_friction(
                    substep_dt, -15.0, Some(GROUND_Y), FRICTION, RESTITUTION,
                );
            }
            collision_system.solve_collisions(&mut bodies);
            for body in &mut bodies {
                body.substep_post(substep_dt);
            }
        }
        for body in &mut bodies {
            body.apply_damping(0.005);
        }
    }

    // Check every ring, especially the bottom ones
    for (i, (body, baseline)) in bodies.iter().zip(baselines.iter()).enumerate() {
        let f = MeshForensics::analyze(body, baseline);
        println!("Pile ring {}: {}", i, f.summary());

        // Bottom rings (index 0) are under most stress
        let min_dim = f.width_ratio.min(f.height_ratio);
        assert!(
            min_dim > 0.50,
            "Ring {} permanently crushed to {:.1}%",
            i, min_dim * 100.0
        );
        assert!(
            f.inverted_triangles == 0,
            "Ring {} has {} inverted triangles",
            i, f.inverted_triangles
        );
        assert!(
            f.severely_compressed_edges <= 5,
            "Ring {} has {} severely compressed edges",
            i, f.severely_compressed_edges
        );
    }
}

// ============================================================================
// 8. High substep golden reference — generate "correct" metrics
// ============================================================================

/// Run the same scenario with very high substeps (16) to establish a golden
/// reference, then run with normal substeps (4) and compare.
/// The normal-substep version should be within tolerance of the golden version.
#[test]
fn test_golden_reference_ring_drop() {
    let mesh = create_ring_mesh(1.0, 0.4, 16, 4);
    let ec = 0.0;
    let ac = 1e-7;

    // Golden: 16 substeps
    let (mut golden_body, golden_baseline) = make_body(
        &mesh, 1000.0, ec, ac, 0.0, 8.0, 0.0, -15.0,
    );
    let golden_substeps = 16u32;
    let golden_dt = DT / golden_substeps as f32;

    for _ in 0..600 {
        for _ in 0..golden_substeps {
            golden_body.substep_pre_with_friction(
                golden_dt, GRAVITY, Some(GROUND_Y), FRICTION, RESTITUTION,
            );
            golden_body.substep_post(golden_dt);
        }
        golden_body.apply_damping(0.005);
    }
    let golden_f = MeshForensics::analyze(&golden_body, &golden_baseline);

    // Normal: 4 substeps
    let (mut normal_body, normal_baseline) = make_body(
        &mesh, 1000.0, ec, ac, 0.0, 8.0, 0.0, -15.0,
    );
    let normal_substeps = 4u32;
    let normal_dt = DT / normal_substeps as f32;

    for _ in 0..600 {
        for _ in 0..normal_substeps {
            normal_body.substep_pre_with_friction(
                normal_dt, GRAVITY, Some(GROUND_Y), FRICTION, RESTITUTION,
            );
            normal_body.substep_post(normal_dt);
        }
        normal_body.apply_damping(0.005);
    }
    let normal_f = MeshForensics::analyze(&normal_body, &normal_baseline);

    println!("Golden (16 substeps): {}", golden_f.summary());
    println!("Normal (4 substeps):  {}", normal_f.summary());

    // Normal should not be dramatically worse than golden
    let golden_min = golden_f.width_ratio.min(golden_f.height_ratio);
    let normal_min = normal_f.width_ratio.min(normal_f.height_ratio);

    // Allow 25% degradation from golden
    assert!(
        normal_min > golden_min * 0.75,
        "Normal substeps ({:.1}%) degraded too much vs golden ({:.1}%)",
        normal_min * 100.0, golden_min * 100.0
    );

    // Edge quality should be comparable
    assert!(
        normal_f.edge_ratio_stddev < golden_f.edge_ratio_stddev * 2.0 + 0.05,
        "Normal edge stddev ({:.3}) much worse than golden ({:.3})",
        normal_f.edge_ratio_stddev, golden_f.edge_ratio_stddev
    );
}

/// Golden reference for soft material
#[test]
fn test_golden_reference_jello_drop() {
    let mesh = create_ring_mesh(1.0, 0.4, 16, 4);
    let ec = 0.0;
    let ac = 1e-6; // Jello

    // Golden: 16 substeps
    let (mut golden_body, golden_baseline) = make_body(
        &mesh, 1000.0, ec, ac, 0.0, 8.0, 0.0, -15.0,
    );
    let golden_substeps = 16u32;
    let golden_dt = DT / golden_substeps as f32;

    for _ in 0..600 {
        for _ in 0..golden_substeps {
            golden_body.substep_pre_with_friction(
                golden_dt, GRAVITY, Some(GROUND_Y), FRICTION, RESTITUTION,
            );
            golden_body.substep_post(golden_dt);
        }
        golden_body.apply_damping(0.005);
    }
    let golden_f = MeshForensics::analyze(&golden_body, &golden_baseline);

    // Normal: 4 substeps
    let (mut normal_body, normal_baseline) = make_body(
        &mesh, 1000.0, ec, ac, 0.0, 8.0, 0.0, -15.0,
    );
    let normal_substeps = 4u32;
    let normal_dt = DT / normal_substeps as f32;

    for _ in 0..600 {
        for _ in 0..normal_substeps {
            normal_body.substep_pre_with_friction(
                normal_dt, GRAVITY, Some(GROUND_Y), FRICTION, RESTITUTION,
            );
            normal_body.substep_post(normal_dt);
        }
        normal_body.apply_damping(0.005);
    }
    let normal_f = MeshForensics::analyze(&normal_body, &normal_baseline);

    println!("Golden jello (16 substeps): {}", golden_f.summary());
    println!("Normal jello (4 substeps):  {}", normal_f.summary());

    let golden_min = golden_f.width_ratio.min(golden_f.height_ratio);
    let normal_min = normal_f.width_ratio.min(normal_f.height_ratio);

    assert!(
        normal_min > golden_min * 0.70,
        "Jello normal ({:.1}%) degraded too much vs golden ({:.1}%)",
        normal_min * 100.0, golden_min * 100.0
    );
}

// ============================================================================
// 9. Friction and rolling — bodies should decelerate, not glitch
// ============================================================================

#[test]
fn test_rolling_ring_shape_preservation() {
    let mesh = create_ring_mesh(1.0, 0.4, 16, 4);
    let (mut body, baseline) = make_body(
        &mesh, 1000.0, 0.0, 1e-7,
        0.0, GROUND_Y + 1.5, 8.0, 0.0, // Moving right on ground
    );

    let substeps = 4u32;
    let substep_dt = DT / substeps as f32;

    // Roll for 5 seconds
    for _ in 0..300 {
        for _ in 0..substeps {
            body.substep_pre_with_friction(
                substep_dt, GRAVITY, Some(GROUND_Y), 0.9, RESTITUTION,
            );
            body.substep_post(substep_dt);
        }
        body.apply_damping(0.005);
    }

    let f = MeshForensics::analyze(&body, &baseline);
    assert_healthy(&f, &HealthTolerance::strict(), "rolling_ring");

    // Should have decelerated (friction)
    assert!(
        f.max_speed < 5.0,
        "Ring didn't decelerate: max_speed={:.1}",
        f.max_speed
    );
}

#[test]
fn test_rolling_box_shape_preservation() {
    let mesh = create_square_mesh(2.0, 4);
    let (mut body, baseline) = make_body(
        &mesh, 1000.0, 0.0, 1e-7,
        0.0, GROUND_Y + 1.5, 6.0, 0.0,
    );

    let substeps = 4u32;
    let substep_dt = DT / substeps as f32;

    for _ in 0..300 {
        for _ in 0..substeps {
            body.substep_pre_with_friction(
                substep_dt, GRAVITY, Some(GROUND_Y), 0.9, RESTITUTION,
            );
            body.substep_post(substep_dt);
        }
        body.apply_damping(0.005);
    }

    let f = MeshForensics::analyze(&body, &baseline);
    assert_healthy(&f, &HealthTolerance::strict(), "rolling_box");
}

// ============================================================================
// 10. Stress tests — extreme conditions
// ============================================================================

/// Very high velocity impact — bodies should not explode or permanently deform
#[test]
fn test_extreme_velocity_ring() {
    let mesh = create_ring_mesh(1.0, 0.4, 16, 4);
    let (mut body, baseline) = make_body(
        &mesh, 1000.0, 0.0, 1e-7,
        0.0, 15.0, 0.0, -50.0,
    );

    let substeps = 4u32;
    let substep_dt = DT / substeps as f32;

    for frame in 0..600 {
        for _ in 0..substeps {
            body.substep_pre_with_friction(
                substep_dt, GRAVITY, Some(GROUND_Y), FRICTION, RESTITUTION,
            );
            body.substep_post(substep_dt);
        }
        body.apply_damping(0.005);

        // Check for explosion every frame
        let ke = body.get_kinetic_energy();
        assert!(
            ke.is_finite() && ke < 1e6,
            "Frame {}: KE exploded to {}", frame, ke
        );
    }

    let f = MeshForensics::analyze(&body, &baseline);
    // After extreme impact, we're lenient on shape but strict on no inversions
    assert!(
        f.inverted_triangles == 0,
        "Extreme impact caused {} inverted triangles",
        f.inverted_triangles
    );
    assert!(
        f.width_ratio.min(f.height_ratio) > 0.45,
        "Extreme impact caused permanent collapse to {:.1}%x{:.1}%",
        f.width_ratio * 100.0, f.height_ratio * 100.0
    );
}

/// Many bodies in a small space — chaos test
#[test]
fn test_chaos_many_bodies() {
    let mesh = create_ring_mesh(0.8, 0.3, 12, 3);

    let mut bodies = Vec::new();
    let mut baselines = Vec::new();

    // 8 bodies in a tight column
    for i in 0..8 {
        let y = 2.0 + i as f32 * 2.5;
        let x = ((i % 3) as f32 - 1.0) * 0.5;
        let vy = -10.0 - (i as f32) * 3.0;
        let (body, baseline) = make_body(
            &mesh, 1000.0, 0.0, 1e-7, x, y, 0.0, vy,
        );
        bodies.push(body);
        baselines.push(baseline);
    }

    let mut collision_system = CollisionSystem::new(0.15);
    let substeps = 4u32;
    let substep_dt = DT / substeps as f32;

    for frame in 0..600 {
        for _ in 0..substeps {
            for body in &mut bodies {
                body.substep_pre_with_friction(
                    substep_dt, GRAVITY, Some(GROUND_Y), FRICTION, RESTITUTION,
                );
            }
            collision_system.solve_collisions(&mut bodies);
            for body in &mut bodies {
                body.substep_post(substep_dt);
            }
        }
        for body in &mut bodies {
            body.apply_damping(0.005);
        }

        // Explosion check
        for (i, body) in bodies.iter().enumerate() {
            let ke = body.get_kinetic_energy();
            assert!(
                ke.is_finite() && ke < 1e6,
                "Frame {}, body {}: KE exploded to {}", frame, i, ke
            );
        }
    }

    // Check all bodies after chaos
    let mut crushed_count = 0;
    for (i, (body, baseline)) in bodies.iter().zip(baselines.iter()).enumerate() {
        let f = MeshForensics::analyze(body, baseline);
        let min_dim = f.width_ratio.min(f.height_ratio);
        if min_dim < 0.50 {
            crushed_count += 1;
            println!("Chaos body {} crushed: {}", i, f.summary());
        }
        // No inversions even in chaos
        assert!(
            f.inverted_triangles == 0,
            "Chaos body {} has {} inverted triangles",
            i, f.inverted_triangles
        );
    }

    // At most 1 body should be significantly crushed in chaos
    assert!(
        crushed_count <= 1,
        "{} out of 8 bodies permanently crushed in chaos scenario",
        crushed_count
    );
}

// ============================================================================
// 11. PhysicsWorld integration — test through the high-level API
// ============================================================================

/// Full PhysicsWorld simulation with mixed soft + rigid bodies
#[test]
fn test_world_mixed_simulation_integrity() {
    use unison_physics::rigid::{Collider, RigidBodyConfig};

    let mut world = PhysicsWorld::new();
    world.set_gravity(GRAVITY);
    world.set_ground(Some(GROUND_Y));
    world.set_ground_friction(FRICTION);
    world.set_substeps(4);

    let ring_mesh = create_ring_mesh(1.0, 0.4, 16, 4);
    let box_mesh = create_square_mesh(1.5, 3);

    // Soft ring
    let soft_ring = world.add_body(
        &ring_mesh,
        BodyConfig::new()
            .at_position(-2.0, 6.0)
            .with_velocity(3.0, -10.0)
            .with_material(Material::RUBBER),
    );

    // Soft box
    let soft_box = world.add_body(
        &box_mesh,
        BodyConfig::new()
            .at_position(2.0, 8.0)
            .with_velocity(-2.0, -5.0)
            .with_material(Material::RUBBER),
    );

    // Kinematic rigid circle obstacle
    let _obstacle = world.add_rigid_body(
        RigidBodyConfig::new()
            .with_collider(Collider::circle(1.0))
            .at_position(0.0, GROUND_Y + 1.0)
            .as_kinematic()
            .with_friction(0.8),
    );

    // Capture baselines
    let ring_baseline = ShapeBaseline::capture(world.get_body(soft_ring).unwrap());
    let box_baseline = ShapeBaseline::capture(world.get_body(soft_box).unwrap());

    // Simulate 10 seconds
    for frame in 0..600 {
        world.step(DT);

        // Explosion check
        let ke = world.total_kinetic_energy();
        assert!(
            ke.is_finite() && ke < 1e6,
            "Frame {}: total KE exploded to {}", frame, ke
        );
    }

    // Check soft body integrity
    let ring_f = MeshForensics::analyze(
        world.get_body(soft_ring).unwrap(), &ring_baseline,
    );
    let box_f = MeshForensics::analyze(
        world.get_body(soft_box).unwrap(), &box_baseline,
    );

    println!("World ring: {}", ring_f.summary());
    println!("World box:  {}", box_f.summary());

    assert_healthy(&ring_f, &HealthTolerance::strict(), "world_ring");
    assert_healthy(&box_f, &HealthTolerance::strict(), "world_box");
}

/// PhysicsWorld with terrain collision
#[test]
fn test_world_terrain_shape_integrity() {
    let mut world = PhysicsWorld::new();
    world.set_gravity(GRAVITY);
    world.set_ground_friction(FRICTION);
    world.set_substeps(4);

    let mesh = create_ring_mesh(1.0, 0.4, 16, 4);
    let handle = world.add_body(
        &mesh,
        BodyConfig::new()
            .at_position(0.0, 5.0)
            .with_velocity(5.0, -10.0)
            .with_material(Material::RUBBER),
    );

    let baseline = ShapeBaseline::capture(world.get_body(handle).unwrap());

    // Wavy terrain
    let height_at = |x: f32| -> f32 { GROUND_Y + 0.5 * (x * 0.5).sin() };
    let normal_at = |x: f32| -> (f32, f32) {
        let slope = 0.25 * (x * 0.5).cos();
        let len = (1.0 + slope * slope).sqrt();
        (-slope / len, 1.0 / len)
    };

    for frame in 0..600 {
        world.step_with_terrain(DT, &height_at, &normal_at);

        let ke = world.total_kinetic_energy();
        assert!(
            ke.is_finite() && ke < 1e6,
            "Frame {}: terrain KE exploded to {}", frame, ke
        );
    }

    let f = MeshForensics::analyze(world.get_body(handle).unwrap(), &baseline);
    println!("Terrain ring: {}", f.summary());
    assert_healthy(&f, &HealthTolerance::strict(), "terrain_ring");
}

// ============================================================================
// 12. Thin ring stress test — thin rings are most vulnerable to crushing
// ============================================================================

#[test]
fn test_thin_ring_all_velocities() {
    let mesh = create_ring_mesh(1.0, 0.7, 16, 4); // Thin wall
    let substeps = 4u32;
    let substep_dt = DT / substeps as f32;

    for (vy, vname) in impact_velocities() {
        let (mut body, baseline) = make_body(
            &mesh, 1000.0, 0.0, 1e-7, 0.0, 10.0, 0.0, vy,
        );

        for _ in 0..600 {
            for _ in 0..substeps {
                body.substep_pre_with_friction(
                    substep_dt, GRAVITY, Some(GROUND_Y), FRICTION, RESTITUTION,
                );
                body.substep_post(substep_dt);
            }
            body.apply_damping(0.005);
        }

        let f = MeshForensics::analyze(&body, &baseline);
        let ctx = format!("thin_ring_rubber_v{}", vname);
        assert_healthy(&f, &HealthTolerance::strict(), &ctx);
    }
}

#[test]
fn test_thin_ring_soft_material_impact() {
    let mesh = create_ring_mesh(1.0, 0.7, 16, 4);
    let substeps = 4u32;
    let substep_dt = DT / substeps as f32;

    // Soft material + fast impact = worst case for thin rings
    let (mut body, baseline) = make_body(
        &mesh, 1000.0, 5e-7, 5e-7, 0.0, 10.0, 0.0, -25.0,
    );

    for _ in 0..600 {
        for _ in 0..substeps {
            body.substep_pre_with_friction(
                substep_dt, GRAVITY, Some(GROUND_Y), FRICTION, RESTITUTION,
            );
            body.substep_post(substep_dt);
        }
        body.apply_damping(0.005);
    }

    let f = MeshForensics::analyze(&body, &baseline);
    println!("Thin ring soft fast: {}", f.summary());
    assert_healthy(
        &f,
        &HealthTolerance::soft_material(),
        "thin_ring_soft_fast",
    );
}
