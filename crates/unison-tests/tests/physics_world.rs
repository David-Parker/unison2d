//! PhysicsWorld regression and diagnostic tests
//!
//! Multi-phase simulation tests that verify force accumulation, torque
//! direction consistency, and full game-like input sequences.

use unison_physics::{PhysicsWorld, BodyHandle, BodyConfig, Material};
use unison_physics::mesh::create_ring_mesh;

// ── Helpers ──────────────────────────────────────────────────────────────

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
            .with_material(Material::RUBBER),
    );
    (world, handle)
}

// ── Regression tests ─────────────────────────────────────────────────────

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

// ── Diagnostic / integration tests ───────────────────────────────────────

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
