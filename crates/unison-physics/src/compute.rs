//! Platform-agnostic compute abstraction for parallel physics
//!
//! This module provides traits and implementations for accelerated computation:
//! - Scalar: Default, works everywhere
//! - SIMD: Uses `wide` crate, works on x86/ARM/WASM (feature = "simd")
//! - GPU: Trait only - implement with Metal (iOS), WebGPU (web), etc.
//!
//! Data is kept in Structure of Arrays (SoA) format for SIMD/GPU efficiency.

/// Batch operations on position/velocity arrays
pub trait ComputeBackend {
    /// Apply gravity and predict positions: pos += vel * dt; vel.y += gravity * dt
    fn integrate_gravity(
        pos: &mut [f32],      // [x0, y0, x1, y1, ...]
        vel: &mut [f32],
        prev_pos: &mut [f32],
        gravity: f32,
        dt: f32,
        inv_mass: &[f32],
    );

    /// Derive velocities from position change: vel = (pos - prev_pos) / dt
    fn derive_velocities(
        pos: &[f32],
        prev_pos: &[f32],
        vel: &mut [f32],
        dt: f32,
    );

    /// Batch distance constraint solve (processes 4 constraints at a time with SIMD)
    fn solve_distance_constraints_batch(
        pos: &mut [f32],
        constraints: &[(usize, usize, f32)],  // (v0, v1, rest_length)
        inv_mass: &[f32],
        alpha: f32,  // compliance / dt²
    );
}

/// Scalar (non-SIMD) implementation - works everywhere
pub struct ScalarBackend;

impl ComputeBackend for ScalarBackend {
    fn integrate_gravity(
        pos: &mut [f32],
        vel: &mut [f32],
        prev_pos: &mut [f32],
        gravity: f32,
        dt: f32,
        inv_mass: &[f32],
    ) {
        let num_verts = inv_mass.len();
        for i in 0..num_verts {
            if inv_mass[i] == 0.0 { continue; }

            // Store previous position
            prev_pos[i * 2] = pos[i * 2];
            prev_pos[i * 2 + 1] = pos[i * 2 + 1];

            // Apply gravity
            vel[i * 2 + 1] += gravity * dt;

            // Predict position
            pos[i * 2] += vel[i * 2] * dt;
            pos[i * 2 + 1] += vel[i * 2 + 1] * dt;
        }
    }

    fn derive_velocities(
        pos: &[f32],
        prev_pos: &[f32],
        vel: &mut [f32],
        dt: f32,
    ) {
        let inv_dt = 1.0 / dt;
        for i in 0..vel.len() {
            vel[i] = (pos[i] - prev_pos[i]) * inv_dt;
        }
    }

    fn solve_distance_constraints_batch(
        pos: &mut [f32],
        constraints: &[(usize, usize, f32)],
        inv_mass: &[f32],
        alpha: f32,
    ) {
        for &(i0, i1, rest_len) in constraints {
            let w0 = inv_mass[i0];
            let w1 = inv_mass[i1];
            let w_sum = w0 + w1;
            if w_sum < 1e-10 { continue; }

            let dx = pos[i1 * 2] - pos[i0 * 2];
            let dy = pos[i1 * 2 + 1] - pos[i0 * 2 + 1];
            let len = (dx * dx + dy * dy).sqrt();
            if len < 1e-10 { continue; }

            let c = len - rest_len;
            let lambda = -c / (w_sum + alpha);

            let nx = dx / len;
            let ny = dy / len;

            let corr0 = -lambda * w0;
            let corr1 = lambda * w1;

            pos[i0 * 2] += corr0 * nx;
            pos[i0 * 2 + 1] += corr0 * ny;
            pos[i1 * 2] += corr1 * nx;
            pos[i1 * 2 + 1] += corr1 * ny;
        }
    }
}

/// SIMD implementation using `wide` crate
#[cfg(feature = "simd")]
pub mod simd {
    use super::ComputeBackend;
    use wide::f32x4;

    pub struct SimdBackend;

    impl ComputeBackend for SimdBackend {
        fn integrate_gravity(
            pos: &mut [f32],
            vel: &mut [f32],
            prev_pos: &mut [f32],
            gravity: f32,
            dt: f32,
            inv_mass: &[f32],
        ) {
            // Process 2 vertices at a time (4 floats = 2 x,y pairs)
            let num_verts = inv_mass.len();
            let dt_v = f32x4::splat(dt);
            let grav_v = f32x4::new([0.0, gravity, 0.0, gravity]); // Only Y components

            let mut i = 0;
            while i + 1 < num_verts {
                // Skip if both vertices are fixed
                if inv_mass[i] == 0.0 && inv_mass[i + 1] == 0.0 {
                    i += 2;
                    continue;
                }

                let base = i * 2;

                // Load positions and velocities for 2 vertices
                let pos_v = f32x4::new([pos[base], pos[base + 1], pos[base + 2], pos[base + 3]]);
                let mut vel_v = f32x4::new([vel[base], vel[base + 1], vel[base + 2], vel[base + 3]]);

                // Store prev_pos
                let pos_arr = pos_v.to_array();
                prev_pos[base] = pos_arr[0];
                prev_pos[base + 1] = pos_arr[1];
                prev_pos[base + 2] = pos_arr[2];
                prev_pos[base + 3] = pos_arr[3];

                // Apply gravity to velocity Y components
                vel_v = vel_v + grav_v * dt_v;

                // Predict position
                let new_pos = pos_v + vel_v * dt_v;

                // Store back
                let new_pos_arr = new_pos.to_array();
                let vel_arr = vel_v.to_array();

                // Handle fixed vertices
                if inv_mass[i] != 0.0 {
                    pos[base] = new_pos_arr[0];
                    pos[base + 1] = new_pos_arr[1];
                    vel[base] = vel_arr[0];
                    vel[base + 1] = vel_arr[1];
                }
                if inv_mass[i + 1] != 0.0 {
                    pos[base + 2] = new_pos_arr[2];
                    pos[base + 3] = new_pos_arr[3];
                    vel[base + 2] = vel_arr[2];
                    vel[base + 3] = vel_arr[3];
                }

                i += 2;
            }

            // Handle remaining vertex
            if i < num_verts && inv_mass[i] != 0.0 {
                let base = i * 2;
                prev_pos[base] = pos[base];
                prev_pos[base + 1] = pos[base + 1];
                vel[base + 1] += gravity * dt;
                pos[base] += vel[base] * dt;
                pos[base + 1] += vel[base + 1] * dt;
            }
        }

        fn derive_velocities(
            pos: &[f32],
            prev_pos: &[f32],
            vel: &mut [f32],
            dt: f32,
        ) {
            let inv_dt = f32x4::splat(1.0 / dt);
            let chunks = vel.len() / 4;

            for i in 0..chunks {
                let base = i * 4;
                let p = f32x4::new([pos[base], pos[base + 1], pos[base + 2], pos[base + 3]]);
                let pp = f32x4::new([prev_pos[base], prev_pos[base + 1], prev_pos[base + 2], prev_pos[base + 3]]);
                let v = (p - pp) * inv_dt;
                let v_arr = v.to_array();
                vel[base] = v_arr[0];
                vel[base + 1] = v_arr[1];
                vel[base + 2] = v_arr[2];
                vel[base + 3] = v_arr[3];
            }

            // Handle remainder
            let remainder_start = chunks * 4;
            let inv_dt_scalar = 1.0 / dt;
            for i in remainder_start..vel.len() {
                vel[i] = (pos[i] - prev_pos[i]) * inv_dt_scalar;
            }
        }

        fn solve_distance_constraints_batch(
            pos: &mut [f32],
            constraints: &[(usize, usize, f32)],
            inv_mass: &[f32],
            alpha: f32,
        ) {
            // For now, fall back to scalar - constraint solving is harder to vectorize
            // due to dependencies between constraints sharing vertices
            super::ScalarBackend::solve_distance_constraints_batch(pos, constraints, inv_mass, alpha);
        }
    }
}

/// GPU compute trait - implement per-platform (Metal, WebGPU, etc.)
pub trait GpuComputeBackend {
    /// Upload position/velocity data to GPU
    fn upload(&mut self, pos: &[f32], vel: &[f32], inv_mass: &[f32]);

    /// Run constraint solver on GPU (multiple iterations)
    fn solve_constraints(&mut self, iterations: u32, dt: f32);

    /// Download results back to CPU
    fn download(&self, pos: &mut [f32], vel: &mut [f32]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_integrate() {
        let mut pos = vec![0.0, 10.0, 5.0, 10.0];
        let mut vel = vec![1.0, 0.0, -1.0, 0.0];
        let mut prev_pos = vec![0.0; 4];
        let inv_mass = vec![1.0, 1.0];

        ScalarBackend::integrate_gravity(&mut pos, &mut vel, &mut prev_pos, -10.0, 0.1, &inv_mass);

        // Check gravity applied to Y velocity
        assert!((vel[1] - (-1.0)).abs() < 0.01);  // 0 + (-10) * 0.1 = -1
        assert!((vel[3] - (-1.0)).abs() < 0.01);

        // Check position updated
        assert!((pos[0] - 0.1).abs() < 0.01);  // 0 + 1 * 0.1 = 0.1
        assert!((pos[1] - 9.9).abs() < 0.01);  // 10 + (-1) * 0.1 = 9.9
    }

    #[test]
    fn test_scalar_derive_velocities() {
        let pos = vec![1.0, 2.0, 3.0, 4.0];
        let prev_pos = vec![0.0, 0.0, 1.0, 2.0];
        let mut vel = vec![0.0; 4];

        ScalarBackend::derive_velocities(&pos, &prev_pos, &mut vel, 0.1);

        assert!((vel[0] - 10.0).abs() < 0.01);
        assert!((vel[1] - 20.0).abs() < 0.01);
        assert!((vel[2] - 20.0).abs() < 0.01);
        assert!((vel[3] - 20.0).abs() < 0.01);
    }

    #[cfg(feature = "simd")]
    #[test]
    fn test_simd_integrate() {
        use simd::SimdBackend;

        let mut pos = vec![0.0, 10.0, 5.0, 10.0];
        let mut vel = vec![1.0, 0.0, -1.0, 0.0];
        let mut prev_pos = vec![0.0; 4];
        let inv_mass = vec![1.0, 1.0];

        SimdBackend::integrate_gravity(&mut pos, &mut vel, &mut prev_pos, -10.0, 0.1, &inv_mass);

        assert!((vel[1] - (-1.0)).abs() < 0.01);
        assert!((vel[3] - (-1.0)).abs() < 0.01);
        assert!((pos[0] - 0.1).abs() < 0.01);
        assert!((pos[1] - 9.9).abs() < 0.01);
    }
}
