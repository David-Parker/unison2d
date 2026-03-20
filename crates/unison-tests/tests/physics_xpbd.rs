//! XPBD soft body simulation tests
//!
//! Multi-frame stability, collision, shape preservation, and stress tests
//! for the low-level XPBD solver.

use unison_physics::mesh::{
    create_ring_mesh, create_square_mesh, create_ellipse_mesh,
    create_star_mesh, create_blob_mesh,
};
use unison_physics::xpbd::{XPBDSoftBody, CollisionSystem};

// ── Helpers ──────────────────────────────────────────────────────────────

fn get_y_bounds(body: &XPBDSoftBody) -> (f32, f32) {
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for i in 0..body.num_verts {
        let y = body.pos[i * 2 + 1];
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    (min_y, max_y)
}

fn get_x_bounds(body: &XPBDSoftBody) -> (f32, f32) {
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    for i in 0..body.num_verts {
        let x = body.pos[i * 2];
        min_x = min_x.min(x);
        max_x = max_x.max(x);
    }
    (min_x, max_x)
}

// ── Ground collision & stability ─────────────────────────────────────────

#[test]
fn test_xpbd_ground_collision() {
    let mesh = create_square_mesh(1.0, 2);
    let mut body = XPBDSoftBody::new(&mesh.vertices, &mesh.triangles, 1000.0, 1e-4, 1e-3);

    // Position body ABOVE ground, let it fall naturally
    let ground_y = -2.0;
    for i in 0..body.num_verts {
        body.pos[i * 2 + 1] += 3.0;  // 3m above ground
        body.prev_pos[i * 2 + 1] += 3.0;
    }

    let dt = 1.0 / 60.0 / 8.0;

    // Run many frames (10 seconds)
    for frame in 0..600 {
        for _ in 0..8 {
            body.substep(dt, -9.8, Some(ground_y));
        }

        // Check for explosion
        let ke = body.get_kinetic_energy();
        assert!(ke.is_finite() && ke < 1e5, "Frame {}: KE exploded: {}", frame, ke);
    }

    // Should be resting on ground
    let lowest = body.get_lowest_y();
    assert!(lowest >= ground_y - 0.2, "Should be above ground, got {}", lowest);
}

#[test]
fn test_xpbd_stability_8_substeps() {
    // This is the critical test: stable with EXACTLY 8 substeps
    let mesh = create_ring_mesh(1.5, 1.0, 16, 4);

    // Use compliance values that give bouncy behavior without explosion
    let mut body = XPBDSoftBody::new(
        &mesh.vertices,
        &mesh.triangles,
        1100.0,  // density
        1e-4,    // edge compliance (medium stiff - like "bouncy" in material_range)
        1e-3,    // area compliance
    );

    // Offset up
    for i in 0..body.num_verts {
        body.pos[i * 2 + 1] += 6.0;
        body.prev_pos[i * 2 + 1] += 6.0;
    }

    let ground_y = -8.0;
    let dt = 1.0 / 60.0 / 8.0;  // EXACTLY 8 substeps

    let mut max_ke: f32 = 0.0;

    // Run 10 seconds (600 frames)
    for frame in 0..600 {
        for _ in 0..8 {
            body.substep(dt, -9.8, Some(ground_y));
            body.apply_damping(0.005);  // Light damping per substep
        }

        let ke = body.get_kinetic_energy();
        max_ke = max_ke.max(ke);

        // Check for explosion
        assert!(
            ke.is_finite() && ke < 1e5,
            "Frame {}: KE exploded: {}", frame, ke
        );

        // Check velocities are reasonable
        let max_vel = body.get_max_velocity();
        assert!(
            max_vel < 50.0,
            "Frame {}: velocity exploded: {}", frame, max_vel
        );
    }

    println!("XPBD 8-substep test passed. Max KE: {:.2}", max_ke);
}

// ── Multi-body collisions ────────────────────────────────────────────────

#[test]
fn test_xpbd_two_body_collision() {
    let mesh = create_ring_mesh(1.5, 1.0, 16, 4);

    // Use zero edge compliance (rigid edges) like actual simulation
    let mut body1 = XPBDSoftBody::new(
        &mesh.vertices, &mesh.triangles, 1100.0, 0.0, 1e-6
    );
    let mut body2 = XPBDSoftBody::new(
        &mesh.vertices, &mesh.triangles, 1100.0, 0.0, 1e-6
    );

    // Position body1 above body2
    for i in 0..body1.num_verts {
        body1.pos[i * 2 + 1] += 10.0;
        body1.prev_pos[i * 2 + 1] += 10.0;
    }
    for i in 0..body2.num_verts {
        body2.pos[i * 2 + 1] += 5.0;
        body2.prev_pos[i * 2 + 1] += 5.0;
    }

    let ground_y = -8.0;
    let dt = 1.0 / 60.0 / 8.0;
    let collision_dist = 0.15;

    // Run 10 seconds
    for frame in 0..600 {
        for _ in 0..8 {
            // Correct order: pre-solve → collisions → post-solve
            body1.substep_pre(dt, -9.8, Some(ground_y));
            body2.substep_pre(dt, -9.8, Some(ground_y));
            body1.collide_with_body(&mut body2, collision_dist);
            body1.substep_post(dt);
            body2.substep_post(dt);
            body1.apply_damping(0.01);
            body2.apply_damping(0.01);
        }

        let ke1 = body1.get_kinetic_energy();
        let ke2 = body2.get_kinetic_energy();

        assert!(
            ke1.is_finite() && ke1 < 1e5 && ke2.is_finite() && ke2 < 1e5,
            "Frame {}: explosion - KE1={}, KE2={}", frame, ke1, ke2
        );
    }

    println!("XPBD two-body collision test passed");
}

#[test]
fn test_xpbd_five_body_collision() {
    // The critical multi-body test
    let mesh = create_ring_mesh(1.5, 1.0, 16, 4);

    let mut bodies: Vec<XPBDSoftBody> = Vec::new();
    let offsets = [
        (0.0, 22.0),
        (-0.5, 18.0),
        (0.5, 14.0),
        (-0.3, 10.0),
        (0.3, 6.0),
    ];

    for (x_off, y_off) in offsets {
        // Use zero edge compliance (rigid edges) like actual simulation
        let mut body = XPBDSoftBody::new(
            &mesh.vertices, &mesh.triangles, 1100.0, 0.0, 1e-6
        );
        for i in 0..body.num_verts {
            body.pos[i * 2] += x_off;
            body.pos[i * 2 + 1] += y_off;
            body.prev_pos[i * 2] += x_off;
            body.prev_pos[i * 2 + 1] += y_off;
        }
        bodies.push(body);
    }

    let ground_y = -8.0;
    let dt = 1.0 / 60.0 / 8.0;
    let collision_dist = 0.15;

    // Run 10 seconds (600 frames)
    for frame in 0..600 {
        for _ in 0..8 {
            // Correct order: pre-solve → collisions → post-solve
            for body in &mut bodies {
                body.substep_pre(dt, -9.8, Some(ground_y));
            }

            // Inter-body collisions
            for i in 0..bodies.len() {
                for j in (i + 1)..bodies.len() {
                    let (left, right) = bodies.split_at_mut(j);
                    left[i].collide_with_body(&mut right[0], collision_dist);
                }
            }

            for body in &mut bodies {
                body.substep_post(dt);
                body.apply_damping(0.01);
            }
        }

        // Check all bodies
        for (idx, body) in bodies.iter().enumerate() {
            let ke = body.get_kinetic_energy();
            assert!(
                ke.is_finite() && ke < 1e5,
                "Frame {}, body {}: KE exploded: {}", frame, idx, ke
            );
        }
    }

    println!("XPBD 5-body collision test passed!");
}

// ── Material range & mixed stiffness ─────────────────────────────────────

/// Test different material stiffnesses
#[test]
fn test_xpbd_material_range() {
    let mesh = create_ring_mesh(1.5, 1.0, 16, 4);
    let ground_y = -8.0;
    let dt = 1.0 / 60.0 / 8.0;

    // Test range of compliance values (soft to stiff)
    let compliances = [
        (5e-4, 5e-3, "jello"),      // Very soft
        (2e-4, 2e-3, "rubber"),     // Soft
        (1e-4, 1e-3, "bouncy"),     // Medium
        (5e-5, 5e-4, "wood"),       // Stiff
        (1e-5, 1e-4, "metal"),      // Very stiff
    ];

    for (edge_c, area_c, name) in compliances {
        let mut body = XPBDSoftBody::new(
            &mesh.vertices, &mesh.triangles, 1000.0, edge_c, area_c
        );

        // Offset up
        for i in 0..body.num_verts {
            body.pos[i * 2 + 1] += 6.0;
            body.prev_pos[i * 2 + 1] += 6.0;
        }

        let mut max_ke: f32 = 0.0;

        // Run 10 seconds
        for frame in 0..600 {
            for _ in 0..8 {
                body.substep(dt, -9.8, Some(ground_y));
                body.apply_damping(0.01);
            }

            let ke = body.get_kinetic_energy();
            max_ke = max_ke.max(ke);

            assert!(
                ke.is_finite() && ke < 1e5,
                "Material '{}' frame {}: KE exploded: {}", name, frame, ke
            );
        }

        println!("Material '{}' passed. Max KE: {:.2}", name, max_ke);
    }
}

/// Stress test: 5 bodies with different stiffnesses
#[test]
fn test_xpbd_mixed_stiffness_pile() {
    let mesh = create_ring_mesh(1.5, 1.0, 16, 4);

    // Create bodies with different stiffnesses
    let configs = [
        (5e-4, 5e-3, 1000.0),  // Very soft (top)
        (2e-4, 2e-3, 1100.0),  // Soft
        (1e-4, 1e-3, 1100.0),  // Medium
        (5e-5, 5e-4, 600.0),   // Stiff
        (1e-5, 1e-4, 2000.0),  // Very stiff (bottom)
    ];

    let offsets = [
        (0.0, 22.0),
        (-0.3, 18.0),
        (0.3, 14.0),
        (-0.2, 10.0),
        (0.2, 6.0),
    ];

    let mut bodies: Vec<XPBDSoftBody> = Vec::new();
    for ((edge_c, area_c, density), (x_off, y_off)) in
        configs.iter().zip(offsets.iter())
    {
        let mut body = XPBDSoftBody::new(
            &mesh.vertices, &mesh.triangles, *density, *edge_c, *area_c
        );
        for j in 0..body.num_verts {
            body.pos[j * 2] += x_off;
            body.pos[j * 2 + 1] += y_off;
            body.prev_pos[j * 2] += x_off;
            body.prev_pos[j * 2 + 1] += y_off;
        }
        bodies.push(body);
    }

    let ground_y = -8.0;
    let dt = 1.0 / 60.0 / 8.0;
    let collision_dist = 0.15;

    // Run 10 seconds (600 frames)
    for frame in 0..600 {
        for _ in 0..8 {
            for body in &mut bodies {
                body.substep(dt, -9.8, Some(ground_y));
                body.apply_damping(0.01);
            }

            for i in 0..bodies.len() {
                for j in (i + 1)..bodies.len() {
                    let (left, right) = bodies.split_at_mut(j);
                    left[i].collide_with_body(&mut right[0], collision_dist);
                }
            }
        }

        for (idx, body) in bodies.iter().enumerate() {
            let ke = body.get_kinetic_energy();
            assert!(
                ke.is_finite() && ke < 1e5,
                "Frame {}, body {}: KE exploded: {}", frame, idx, ke
            );
        }
    }

    println!("XPBD mixed stiffness pile test passed!");
}

// ── Shape preservation ───────────────────────────────────────────────────

/// Test that shape is preserved (no pancaking)
#[test]
fn test_xpbd_shape_preservation() {
    let mesh = create_ring_mesh(1.5, 1.0, 16, 4);

    // Use zero edge compliance for perfectly rigid edges
    let mut body = XPBDSoftBody::new(
        &mesh.vertices, &mesh.triangles, 1100.0,
        0.0,   // Perfectly rigid edges - no stretching allowed
        1e-6,  // Very stiff area
    );

    // Offset up
    for i in 0..body.num_verts {
        body.pos[i * 2 + 1] += 6.0;
        body.prev_pos[i * 2 + 1] += 6.0;
    }

    let initial_aspect = body.get_aspect_ratio();
    let ground_y = -8.0;
    let dt = 1.0 / 60.0 / 8.0;

    // Run 5 seconds (300 frames)
    for frame in 0..300 {
        for _ in 0..8 {
            body.substep(dt, -9.8, Some(ground_y));
        }

        let aspect = body.get_aspect_ratio();

        // Shape should not pancake: aspect ratio should not exceed 3x initial
        assert!(
            aspect < initial_aspect * 3.0,
            "Frame {}: shape pancaked! Initial aspect: {:.2}, current: {:.2}",
            frame, initial_aspect, aspect
        );
    }

    let final_aspect = body.get_aspect_ratio();
    println!("Shape preservation test passed. Initial: {:.2}, Final: {:.2}", initial_aspect, final_aspect);
}

// ── Mesh shape stability ─────────────────────────────────────────────────

/// Test ellipse mesh simulation stability
#[test]
fn test_xpbd_ellipse_mesh() {
    let mesh = create_ellipse_mesh(2.5, 1.8, 16, 4);
    let mut body = XPBDSoftBody::new(
        &mesh.vertices, &mesh.triangles, 1100.0, 0.0, 1e-6
    );

    // Offset up
    for i in 0..body.num_verts {
        body.pos[i * 2 + 1] += 8.0;
        body.prev_pos[i * 2 + 1] += 8.0;
    }

    let ground_y = -8.0;
    let dt = 1.0 / 60.0 / 8.0;

    // Run 5 seconds
    for frame in 0..300 {
        for _ in 0..8 {
            body.substep(dt, -9.8, Some(ground_y));
            body.apply_damping(0.01);
        }

        let ke = body.get_kinetic_energy();
        assert!(
            ke.is_finite() && ke < 1e5,
            "Frame {}: ellipse KE exploded: {}", frame, ke
        );
    }

    println!("XPBD ellipse mesh test passed");
}

/// Test star mesh simulation stability
#[test]
fn test_xpbd_star_mesh() {
    let mesh = create_star_mesh(1.6, 0.7, 5, 4);
    let mut body = XPBDSoftBody::new(
        &mesh.vertices, &mesh.triangles, 1100.0, 0.0, 1e-6
    );

    // Offset up
    for i in 0..body.num_verts {
        body.pos[i * 2 + 1] += 8.0;
        body.prev_pos[i * 2 + 1] += 8.0;
    }

    let ground_y = -8.0;
    let dt = 1.0 / 60.0 / 8.0;

    // Run 5 seconds
    for frame in 0..300 {
        for _ in 0..8 {
            body.substep(dt, -9.8, Some(ground_y));
            body.apply_damping(0.01);
        }

        let ke = body.get_kinetic_energy();
        assert!(
            ke.is_finite() && ke < 1e5,
            "Frame {}: star KE exploded: {}", frame, ke
        );
    }

    println!("XPBD star mesh test passed");
}

/// Test blob mesh simulation stability
#[test]
fn test_xpbd_blob_mesh() {
    let mesh = create_blob_mesh(1.4, 0.25, 16, 4, 42);
    let mut body = XPBDSoftBody::new(
        &mesh.vertices, &mesh.triangles, 1100.0, 0.0, 1e-6
    );

    // Offset up
    for i in 0..body.num_verts {
        body.pos[i * 2 + 1] += 8.0;
        body.prev_pos[i * 2 + 1] += 8.0;
    }

    let ground_y = -8.0;
    let dt = 1.0 / 60.0 / 8.0;

    // Run 5 seconds
    for frame in 0..300 {
        for _ in 0..8 {
            body.substep(dt, -9.8, Some(ground_y));
            body.apply_damping(0.01);
        }

        let ke = body.get_kinetic_energy();
        assert!(
            ke.is_finite() && ke < 1e5,
            "Frame {}: blob KE exploded: {}", frame, ke
        );
    }

    println!("XPBD blob mesh test passed");
}

/// Test mixed shape collisions
#[test]
fn test_xpbd_mixed_shape_collision() {
    // Create different shapes
    let ring_mesh = create_ring_mesh(1.5, 1.0, 16, 4);
    let ellipse_mesh = create_ellipse_mesh(2.0, 1.5, 16, 4);
    let star_mesh = create_star_mesh(1.4, 0.6, 5, 4);
    let blob_mesh = create_blob_mesh(1.3, 0.2, 16, 4, 123);

    let meshes = [&ring_mesh, &ellipse_mesh, &star_mesh, &blob_mesh];
    let offsets = [(0.0, 18.0), (-0.3, 14.0), (0.3, 10.0), (0.0, 6.0)];

    let mut bodies: Vec<XPBDSoftBody> = Vec::new();
    for (mesh, (x_off, y_off)) in meshes.iter().zip(offsets.iter()) {
        let mut body = XPBDSoftBody::new(
            &mesh.vertices, &mesh.triangles, 1100.0, 0.0, 1e-6
        );
        for i in 0..body.num_verts {
            body.pos[i * 2] += x_off;
            body.pos[i * 2 + 1] += y_off;
            body.prev_pos[i * 2] += x_off;
            body.prev_pos[i * 2 + 1] += y_off;
        }
        bodies.push(body);
    }

    let ground_y = -8.0;
    let dt = 1.0 / 60.0 / 8.0;
    let collision_dist = 0.15;

    // Run 10 seconds
    for frame in 0..600 {
        for _ in 0..8 {
            // Correct order: pre-solve → collisions → post-solve
            for body in &mut bodies {
                body.substep_pre(dt, -9.8, Some(ground_y));
            }

            // Inter-body collisions
            for i in 0..bodies.len() {
                for j in (i + 1)..bodies.len() {
                    let (left, right) = bodies.split_at_mut(j);
                    left[i].collide_with_body(&mut right[0], collision_dist);
                }
            }

            for body in &mut bodies {
                body.substep_post(dt);
                body.apply_damping(0.01);
            }
        }

        for (idx, body) in bodies.iter().enumerate() {
            let ke = body.get_kinetic_energy();
            assert!(
                ke.is_finite() && ke < 1e5,
                "Frame {}, body {}: KE exploded: {}", frame, idx, ke
            );
        }
    }

    println!("XPBD mixed shape collision test passed!");
}

// ── Collision correctness ────────────────────────────────────────────────

/// Test that collisions actually prevent penetration
#[test]
fn test_collision_prevents_penetration() {
    let mesh = create_ring_mesh(1.5, 1.0, 16, 4);

    // Two bodies, one directly above the other
    let mut body1 = XPBDSoftBody::new(
        &mesh.vertices, &mesh.triangles, 1100.0, 0.0, 1e-6
    );
    let mut body2 = XPBDSoftBody::new(
        &mesh.vertices, &mesh.triangles, 1100.0, 0.0, 1e-6
    );

    // Body1 at y=6, Body2 at y=2 (will collide when body1 falls)
    for i in 0..body1.num_verts {
        body1.pos[i * 2 + 1] += 6.0;
        body1.prev_pos[i * 2 + 1] += 6.0;
    }
    for i in 0..body2.num_verts {
        body2.pos[i * 2 + 1] += 2.0;
        body2.prev_pos[i * 2 + 1] += 2.0;
    }

    let ground_y = -8.0;
    let dt = 1.0 / 60.0 / 8.0;
    let collision_dist = 0.15;

    // Run 3 seconds - enough for collision to occur
    for _ in 0..180 {
        for _ in 0..8 {
            body1.substep_pre(dt, -9.8, Some(ground_y));
            body2.substep_pre(dt, -9.8, Some(ground_y));
            body1.collide_with_body(&mut body2, collision_dist);
            body1.substep_post(dt);
            body2.substep_post(dt);
        }
    }

    // Verify bodies didn't pass through each other
    // Body1 center should still be above body2 center
    let (_, cy1) = body1.get_center();
    let (_, cy2) = body2.get_center();

    assert!(
        cy1 > cy2,
        "Bodies penetrated! Body1 center y={:.2}, Body2 center y={:.2}",
        cy1, cy2
    );

    // Bodies should maintain some minimum separation (at least body diameter minus compression)
    let separation = cy1 - cy2;
    assert!(
        separation > 1.0,  // At least 1 unit apart (bodies have ~3 unit diameter)
        "Bodies too close! Separation={:.2}", separation
    );

    println!("Collision penetration test passed. Separation: {:.2}", separation);
}

/// Test that falling body bounces off stationary body
#[test]
fn test_collision_momentum_transfer() {
    let mesh = create_ring_mesh(1.5, 1.0, 16, 4);

    // Falling body
    let mut falling = XPBDSoftBody::new(
        &mesh.vertices, &mesh.triangles, 1100.0, 0.0, 1e-6
    );
    // Stationary body (will be pushed by collision)
    let mut stationary = XPBDSoftBody::new(
        &mesh.vertices, &mesh.triangles, 1100.0, 0.0, 1e-6
    );

    // Falling body at y=10, stationary at y=2
    for i in 0..falling.num_verts {
        falling.pos[i * 2 + 1] += 10.0;
        falling.prev_pos[i * 2 + 1] += 10.0;
    }
    for i in 0..stationary.num_verts {
        stationary.pos[i * 2 + 1] += 2.0;
        stationary.prev_pos[i * 2 + 1] += 2.0;
    }

    let (_, initial_stationary_y) = stationary.get_center();
    let ground_y = -8.0;
    let dt = 1.0 / 60.0 / 8.0;
    let collision_dist = 0.15;

    // Run until collision happens and momentum transfers
    let mut collision_occurred = false;
    for frame in 0..300 {
        for _ in 0..8 {
            falling.substep_pre(dt, -9.8, Some(ground_y));
            stationary.substep_pre(dt, -9.8, Some(ground_y));
            let collisions = falling.collide_with_body(&mut stationary, collision_dist);
            if collisions > 0 {
                collision_occurred = true;
            }
            falling.substep_post(dt);
            stationary.substep_post(dt);
        }

        // Check after collision that stationary body moved
        if collision_occurred && frame > 60 {
            let (_, current_stationary_y) = stationary.get_center();
            // Stationary body should have been pushed down
            assert!(
                current_stationary_y < initial_stationary_y,
                "Stationary body should move down after collision. Initial: {:.2}, Current: {:.2}",
                initial_stationary_y, current_stationary_y
            );
            println!("Collision momentum test passed. Stationary body moved from {:.2} to {:.2}",
                initial_stationary_y, current_stationary_y);
            return;
        }
    }

    assert!(collision_occurred, "No collision occurred during test");
}

/// Test CollisionSystem with multiple bodies
#[test]
fn test_collision_system_separation() {
    let mesh = create_ring_mesh(1.5, 1.0, 16, 4);

    // Stack 3 bodies vertically
    let mut bodies: Vec<XPBDSoftBody> = Vec::new();
    for y in [2.0, 6.0, 10.0] {
        let mut body = XPBDSoftBody::new(
            &mesh.vertices, &mesh.triangles, 1100.0, 0.0, 1e-6
        );
        for j in 0..body.num_verts {
            body.pos[j * 2 + 1] += y;
            body.prev_pos[j * 2 + 1] += y;
        }
        bodies.push(body);
    }

    let mut collision_system = CollisionSystem::new(0.00);
    let ground_y = -8.0;
    let dt = 1.0 / 60.0 / 8.0;

    // Run 5 seconds
    for _ in 0..300 {
        for _ in 0..8 {
            for body in &mut bodies {
                body.substep_pre(dt, -9.8, Some(ground_y));
            }
            collision_system.solve_collisions(&mut bodies);
            for body in &mut bodies {
                body.substep_post(dt);
                body.apply_damping(0.01);
            }
        }
    }

    // Verify all bodies are separated (none passed through each other)
    let centers: Vec<f32> = bodies.iter().map(|b| b.get_center().1).collect();

    // All bodies should have distinct y positions
    for i in 0..centers.len() {
        for j in (i + 1)..centers.len() {
            let diff = (centers[i] - centers[j]).abs();
            assert!(
                diff > 0.5,  // At least 0.5 units apart
                "Bodies {} and {} too close: y1={:.2}, y2={:.2}, diff={:.2}",
                i, j, centers[i], centers[j], diff
            );
        }
    }

    println!("CollisionSystem separation test passed. Centers: {:?}", centers);
}

// ── High-velocity impact & recovery ──────────────────────────────────────

/// Test soft ring under high-velocity ground impact
/// Verifies the ring doesn't permanently collapse
#[test]
fn test_high_velocity_impact_recovery() {
    // Use exact game ring dimensions
    let mesh = create_ring_mesh(1.01, 0.37, 16, 4);  // DROP_OUTER/INNER_RADIUS

    // Soft material like the game uses
    let edge_compliance = 1e-7;
    let area_compliance = 5e-8;

    let mut body = XPBDSoftBody::new(
        &mesh.vertices, &mesh.triangles, 950.0, edge_compliance, area_compliance
    );

    // Position high and give high downward velocity (dropped from above camera)
    let start_height = 15.0;
    let impact_velocity = -25.0;  // Fast downward like dropped rings
    for i in 0..body.num_verts {
        body.pos[i * 2 + 1] += start_height;
        body.prev_pos[i * 2 + 1] += start_height;
        body.vel[i * 2 + 1] = impact_velocity;
    }

    // Measure initial shape
    let mesh_height = 1.01 * 2.0;  // outer diameter
    println!("Initial height (mesh): {:.3}", mesh_height);

    let ground_y = -5.0;  // Game's GROUND_Y
    let dt = 1.0 / 60.0;
    let substeps = 4;  // Game uses 4 substeps
    let substep_dt = dt / substeps as f32;

    // Simulate 5 seconds
    for frame in 0..300 {
        for _ in 0..substeps {
            body.substep_pre_with_friction(substep_dt, -15.0, Some(ground_y), 0.7, 0.0);
            body.substep_post(substep_dt);
        }

        // Check shape at key moments
        if frame == 10 || frame == 30 || frame == 60 || frame == 120 || frame == 299 {
            let (min_y, max_y) = get_y_bounds(&body);
            let height = max_y - min_y;
            let height_ratio = height / mesh_height;
            println!("Frame {}: height={:.3}, ratio={:.2}%", frame, height, height_ratio * 100.0);
        }
    }

    // After settling, height should recover to at least 70% of original
    let (_, final_max_y) = get_y_bounds(&body);
    let (final_min_y, _) = get_y_bounds(&body);
    let final_height = final_max_y - final_min_y;
    let recovery_ratio = final_height / mesh_height;

    println!("Final height: {:.3}, recovery: {:.1}%", final_height, recovery_ratio * 100.0);

    assert!(
        recovery_ratio > 0.7,
        "Ring collapsed! Final height {:.3} is only {:.1}% of mesh height {:.3}",
        final_height, recovery_ratio * 100.0, mesh_height
    );
}

/// Test small soft rings at extreme impact velocities
/// Verifies that soft bodies maintain shape after high-speed ground collision
#[test]
fn test_small_ring_extreme_impact() {
    // Small ring dimensions
    let mesh = create_ring_mesh(1.01, 0.37, 16, 4);
    let mesh_diameter = 1.01 * 2.0;

    // Use the softer game material that causes issues
    let edge_compliance = 5e-7;
    let area_compliance = 5e-7;

    let mut body = XPBDSoftBody::new(
        &mesh.vertices, &mesh.triangles, 950.0, edge_compliance, area_compliance
    );

    // Extreme velocity - falling from high up
    let start_height = 20.0;
    let impact_velocity = -40.0;  // Very fast
    for i in 0..body.num_verts {
        body.pos[i * 2 + 1] += start_height;
        body.prev_pos[i * 2 + 1] += start_height;
        body.vel[i * 2 + 1] = impact_velocity;
    }

    println!("=== Small Ring Extreme Impact Test ===");
    println!("Mesh diameter: {:.3}", mesh_diameter);
    println!("Material: edge_compliance={:e}, area_compliance={:e}", edge_compliance, area_compliance);
    println!("Impact velocity: {:.1}", impact_velocity);

    let ground_y = -5.0;
    let dt = 1.0 / 60.0;
    let substeps = 4;
    let substep_dt = dt / substeps as f32;

    // Simulate 10 seconds
    for frame in 0..600 {
        for _ in 0..substeps {
            body.substep_pre_with_friction(substep_dt, -15.0, Some(ground_y), 0.7, 0.0);
            // Extra constraint solving after ground collision to restore shape
            for _ in 0..3 {
                body.solve_constraints(substep_dt);
            }
            body.substep_post(substep_dt);
        }

        if frame == 5 || frame == 10 || frame == 30 || frame == 60 || frame == 300 || frame == 599 {
            let (min_y, max_y) = get_y_bounds(&body);
            let (min_x, max_x) = get_x_bounds(&body);
            let height = max_y - min_y;
            let width = max_x - min_x;
            println!("Frame {:3}: height={:.3} ({:.1}%), width={:.3} ({:.1}%)",
                frame, height, height / mesh_diameter * 100.0,
                width, width / mesh_diameter * 100.0);
        }
    }

    let (min_y, max_y) = get_y_bounds(&body);
    let (min_x, max_x) = get_x_bounds(&body);
    let final_height = max_y - min_y;
    let final_width = max_x - min_x;

    println!("Final: height={:.3} ({:.1}%), width={:.3} ({:.1}%)",
        final_height, final_height / mesh_diameter * 100.0,
        final_width, final_width / mesh_diameter * 100.0);

    assert!(
        final_height / mesh_diameter > 0.7,
        "Ring height collapsed to {:.1}%", final_height / mesh_diameter * 100.0
    );
    assert!(
        final_width / mesh_diameter > 0.7,
        "Ring width collapsed to {:.1}%", final_width / mesh_diameter * 100.0
    );
}

/// Test small soft rings colliding with each other at high speed
#[test]
fn test_small_rings_collision_crushing() {
    let mesh = create_ring_mesh(1.01, 0.37, 16, 4);
    let mesh_diameter = 1.01 * 2.0;

    // Softer material
    let edge_compliance = 5e-7;
    let area_compliance = 5e-7;

    // Create multiple rings falling at different times
    let mut bodies: Vec<XPBDSoftBody> = Vec::new();
    for i in 0..5 {
        let mut body = XPBDSoftBody::new(
            &mesh.vertices, &mesh.triangles, 950.0, edge_compliance, area_compliance
        );
        // Stagger heights so they land on each other
        let height = 5.0 + i as f32 * 3.0;
        // Slight horizontal offset
        let x_offset = (i as f32 - 2.0) * 0.3;
        for j in 0..body.num_verts {
            body.pos[j * 2] += x_offset;
            body.pos[j * 2 + 1] += height;
            body.prev_pos[j * 2] += x_offset;
            body.prev_pos[j * 2 + 1] += height;
            body.vel[j * 2 + 1] = -30.0;  // Fast fall
        }
        bodies.push(body);
    }

    println!("=== Small Rings Collision Crushing Test ===");
    println!("5 rings falling and colliding");

    let mut collision_system = CollisionSystem::new(0.15);
    let ground_y = -5.0;
    let dt = 1.0 / 60.0;
    let substeps = 4;
    let substep_dt = dt / substeps as f32;

    // Simulate 10 seconds
    for _ in 0..600 {
        for _ in 0..substeps {
            for body in &mut bodies {
                body.substep_pre_with_friction(substep_dt, -15.0, Some(ground_y), 0.7, 0.0);
            }
            collision_system.solve_collisions(&mut bodies);
            // FIX: Re-solve constraints after collisions to restore shape
            for body in &mut bodies {
                for _ in 0..5 {
                    body.solve_constraints(substep_dt);
                }
            }
            for body in &mut bodies {
                body.substep_post(substep_dt);
            }
        }
    }

    // Check all rings maintained shape
    let mut any_crushed = false;
    for (i, body) in bodies.iter().enumerate() {
        let (min_y, max_y) = get_y_bounds(body);
        let (min_x, max_x) = get_x_bounds(body);
        let height = max_y - min_y;
        let width = max_x - min_x;
        let h_ratio = height / mesh_diameter;
        let w_ratio = width / mesh_diameter;

        println!("Ring {}: h={:.1}%, w={:.1}%", i, h_ratio * 100.0, w_ratio * 100.0);
        if h_ratio < 0.6 || w_ratio < 0.6 {
            println!("  ^ CRUSHED!");
            any_crushed = true;
        }
    }

    assert!(!any_crushed, "Some rings got permanently crushed!");
}

/// Test multiple soft rings colliding and stacking
/// Verifies rings recover shape after being stacked and compressed
#[test]
fn test_stacked_soft_rings_recovery() {
    let mesh = create_ring_mesh(1.01, 0.37, 16, 4);
    let mesh_height = 1.01 * 2.0;

    let edge_compliance = 1e-7;
    let area_compliance = 5e-8;

    // Create 5 rings at different heights, all falling
    let mut bodies: Vec<XPBDSoftBody> = Vec::new();
    for i in 0..5 {
        let mut body = XPBDSoftBody::new(
            &mesh.vertices, &mesh.triangles, 950.0, edge_compliance, area_compliance
        );
        let height = 5.0 + i as f32 * 4.0;  // Staggered heights
        for j in 0..body.num_verts {
            body.pos[j * 2 + 1] += height;
            body.prev_pos[j * 2 + 1] += height;
            body.vel[j * 2 + 1] = -15.0;  // Falling
        }
        bodies.push(body);
    }

    let mut collision_system = CollisionSystem::new(0.15);
    let ground_y = -5.0;
    let dt = 1.0 / 60.0;
    let substeps = 4;
    let substep_dt = dt / substeps as f32;

    // Simulate 10 seconds
    for frame in 0..600 {
        for _ in 0..substeps {
            for body in &mut bodies {
                body.substep_pre_with_friction(substep_dt, -15.0, Some(ground_y), 0.7, 0.0);
            }
            collision_system.solve_collisions(&mut bodies);
            for body in &mut bodies {
                body.substep_post(substep_dt);
            }
        }

        // Check shapes periodically
        if frame == 60 || frame == 180 || frame == 599 {
            println!("Frame {}:", frame);
            for (i, body) in bodies.iter().enumerate() {
                let (min_y, max_y) = get_y_bounds(body);
                let height = max_y - min_y;
                let ratio = height / mesh_height;
                println!("  Ring {}: height={:.3}, ratio={:.1}%", i, height, ratio * 100.0);
            }
        }
    }

    // Check all rings maintained shape
    for (i, body) in bodies.iter().enumerate() {
        let (min_y, max_y) = get_y_bounds(body);
        let height = max_y - min_y;
        let ratio = height / mesh_height;
        assert!(
            ratio > 0.6,
            "Ring {} collapsed! Height {:.3} is only {:.1}% of expected {:.3}",
            i, height, ratio * 100.0, mesh_height
        );
    }
    println!("All rings maintained shape!");
}
