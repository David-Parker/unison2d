//! Programmatic tracing system for simulation debugging
//!
//! Captures detailed state snapshots during simulation for analysis.

use std::collections::VecDeque;
use unison_math::Vec2;

/// A single frame's worth of simulation state
#[derive(Clone, Debug)]
pub struct FrameTrace {
    pub frame: u32,
    pub time: f32,

    // Mesh state
    pub centroid: Vec2,
    pub bounding_box: [f32; 4],  // min_x, max_x, min_y, max_y
    pub orientation: f32,        // angle of principal axis in radians

    // Velocity state
    pub linear_velocity: Vec2,      // average velocity
    pub angular_velocity: f32,       // estimated rotation rate
    pub max_velocity: f32,

    // Deformation state
    pub min_j: f32,
    pub max_j: f32,
    pub avg_j: f32,
    pub inverted_triangles: u32,

    // Energy
    pub kinetic_energy: f32,

    // Per-vertex extremes (index, value)
    pub fastest_vertex: (usize, f32),
    pub lowest_vertex: (usize, f32),

    // Custom markers
    pub markers: Vec<(String, f32)>,
}

impl FrameTrace {
    pub fn new(frame: u32, time: f32) -> Self {
        FrameTrace {
            frame,
            time,
            centroid: Vec2::ZERO,
            bounding_box: [0.0, 0.0, 0.0, 0.0],
            orientation: 0.0,
            linear_velocity: Vec2::ZERO,
            angular_velocity: 0.0,
            max_velocity: 0.0,
            min_j: 1.0,
            max_j: 1.0,
            avg_j: 1.0,
            inverted_triangles: 0,
            kinetic_energy: 0.0,
            fastest_vertex: (0, 0.0),
            lowest_vertex: (0, 0.0),
            markers: Vec::new(),
        }
    }

    pub fn add_marker(&mut self, name: &str, value: f32) {
        self.markers.push((name.to_string(), value));
    }
}

/// Simulation tracer that collects frame snapshots
pub struct SimulationTracer {
    traces: VecDeque<FrameTrace>,
    max_frames: usize,
    enabled: bool,
    current_time: f32,
}

impl SimulationTracer {
    pub fn new(max_frames: usize) -> Self {
        SimulationTracer {
            traces: VecDeque::with_capacity(max_frames),
            max_frames,
            enabled: true,
            current_time: 0.0,
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn clear(&mut self) {
        self.traces.clear();
        self.current_time = 0.0;
    }

    /// Capture a frame from positions, velocities, and triangles
    pub fn capture_frame(
        &mut self,
        frame: u32,
        dt: f32,
        positions: &[f32],
        velocities: &[f32],
        triangles: &[u32],
        rest_areas: &[f32],
    ) -> Option<&FrameTrace> {
        if !self.enabled {
            return None;
        }

        self.current_time += dt;
        let num_verts = positions.len() / 2;
        let num_tris = triangles.len() / 3;

        let mut trace = FrameTrace::new(frame, self.current_time);

        // Compute centroid
        let mut cx = 0.0f32;
        let mut cy = 0.0f32;
        for i in 0..num_verts {
            cx += positions[i * 2];
            cy += positions[i * 2 + 1];
        }
        cx /= num_verts as f32;
        cy /= num_verts as f32;
        trace.centroid = Vec2::new(cx, cy);

        // Compute bounding box
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        for i in 0..num_verts {
            let x = positions[i * 2];
            let y = positions[i * 2 + 1];
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        trace.bounding_box = [min_x, max_x, min_y, max_y];

        // Compute orientation using covariance matrix of vertices relative to centroid
        let mut cxx = 0.0f32;
        let mut cyy = 0.0f32;
        let mut cxy = 0.0f32;
        for i in 0..num_verts {
            let dx = positions[i * 2] - cx;
            let dy = positions[i * 2 + 1] - cy;
            cxx += dx * dx;
            cyy += dy * dy;
            cxy += dx * dy;
        }
        // Principal axis angle from covariance matrix
        trace.orientation = 0.5 * (2.0 * cxy).atan2(cxx - cyy);

        // Compute linear velocity (average)
        let mut vx = 0.0f32;
        let mut vy = 0.0f32;
        let mut max_vel = 0.0f32;
        let mut fastest_idx = 0usize;
        for i in 0..num_verts {
            let vel_x = velocities[i * 2];
            let vel_y = velocities[i * 2 + 1];
            vx += vel_x;
            vy += vel_y;
            let speed = (vel_x * vel_x + vel_y * vel_y).sqrt();
            if speed > max_vel {
                max_vel = speed;
                fastest_idx = i;
            }
        }
        vx /= num_verts as f32;
        vy /= num_verts as f32;
        trace.linear_velocity = Vec2::new(vx, vy);
        trace.max_velocity = max_vel;
        trace.fastest_vertex = (fastest_idx, max_vel);

        // Estimate angular velocity from tangential velocities relative to centroid
        let mut angular_sum = 0.0f32;
        let mut weight_sum = 0.0f32;
        for i in 0..num_verts {
            let dx = positions[i * 2] - cx;
            let dy = positions[i * 2 + 1] - cy;
            let r_sq = dx * dx + dy * dy;
            if r_sq > 1e-6 {
                // Tangential velocity component: (v × r) / |r|²
                let rel_vx = velocities[i * 2] - vx;
                let rel_vy = velocities[i * 2 + 1] - vy;
                let omega = (dx * rel_vy - dy * rel_vx) / r_sq;
                angular_sum += omega * r_sq;  // weight by r²
                weight_sum += r_sq;
            }
        }
        trace.angular_velocity = if weight_sum > 1e-6 { angular_sum / weight_sum } else { 0.0 };

        // Find lowest vertex
        let mut lowest_idx = 0usize;
        let mut lowest_y = f32::MAX;
        for i in 0..num_verts {
            let y = positions[i * 2 + 1];
            if y < lowest_y {
                lowest_y = y;
                lowest_idx = i;
            }
        }
        trace.lowest_vertex = (lowest_idx, lowest_y);

        // Compute J (area ratio) for each triangle
        let mut min_j = f32::MAX;
        let mut max_j = f32::MIN;
        let mut sum_j = 0.0f32;
        let mut inverted = 0u32;

        for t in 0..num_tris {
            let i0 = triangles[t * 3] as usize;
            let i1 = triangles[t * 3 + 1] as usize;
            let i2 = triangles[t * 3 + 2] as usize;

            let x0 = positions[i0 * 2];
            let y0 = positions[i0 * 2 + 1];
            let x1 = positions[i1 * 2];
            let y1 = positions[i1 * 2 + 1];
            let x2 = positions[i2 * 2];
            let y2 = positions[i2 * 2 + 1];

            // Signed area
            let area = 0.5 * ((x1 - x0) * (y2 - y0) - (x2 - x0) * (y1 - y0));
            let rest_area = rest_areas[t];
            let j = area / rest_area;

            min_j = min_j.min(j);
            max_j = max_j.max(j);
            sum_j += j;

            if j < 0.0 {
                inverted += 1;
            }
        }

        trace.min_j = min_j;
        trace.max_j = max_j;
        trace.avg_j = sum_j / num_tris as f32;
        trace.inverted_triangles = inverted;

        // Compute kinetic energy
        let mut ke = 0.0f32;
        for i in 0..num_verts {
            let vel_x = velocities[i * 2];
            let vel_y = velocities[i * 2 + 1];
            // Assume unit mass per vertex for now
            ke += 0.5 * (vel_x * vel_x + vel_y * vel_y);
        }
        trace.kinetic_energy = ke;

        // Store trace
        if self.traces.len() >= self.max_frames {
            self.traces.pop_front();
        }
        self.traces.push_back(trace);

        self.traces.back()
    }

    /// Get all traces
    pub fn traces(&self) -> &VecDeque<FrameTrace> {
        &self.traces
    }

    /// Get the last N traces
    pub fn last_n(&self, n: usize) -> impl Iterator<Item = &FrameTrace> {
        let skip = self.traces.len().saturating_sub(n);
        self.traces.iter().skip(skip)
    }

    /// Get trace at specific frame (if still in buffer)
    pub fn get_frame(&self, frame: u32) -> Option<&FrameTrace> {
        self.traces.iter().find(|t| t.frame == frame)
    }

    /// Print summary of recent frames
    pub fn print_summary(&self, last_n: usize) {
        println!("\n=== Simulation Trace Summary (last {} frames) ===", last_n);
        println!("{:>6} {:>8} {:>10} {:>10} {:>8} {:>8} {:>8} {:>8} {:>6}",
            "Frame", "Time", "Centroid", "", "MinJ", "MaxJ", "MaxVel", "AngVel", "Inv");
        println!("{:>6} {:>8} {:>10} {:>10} {:>8} {:>8} {:>8} {:>8} {:>6}",
            "", "(s)", "X", "Y", "", "", "(m/s)", "(rad/s)", "Tris");
        println!("{}", "-".repeat(80));

        for trace in self.last_n(last_n) {
            println!("{:>6} {:>8.3} {:>10.3} {:>10.3} {:>8.3} {:>8.3} {:>8.2} {:>8.2} {:>6}",
                trace.frame,
                trace.time,
                trace.centroid.x,
                trace.centroid.y,
                trace.min_j,
                trace.max_j,
                trace.max_velocity,
                trace.angular_velocity,
                trace.inverted_triangles
            );
        }
        println!();
    }

    /// Export to CSV format
    pub fn to_csv(&self) -> String {
        let mut csv = String::new();
        csv.push_str("frame,time,cx,cy,min_x,max_x,min_y,max_y,orientation,");
        csv.push_str("vx,vy,max_vel,angular_vel,min_j,max_j,avg_j,inverted,ke\n");

        for t in &self.traces {
            csv.push_str(&format!(
                "{},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},",
                t.frame, t.time, t.centroid.x, t.centroid.y,
                t.bounding_box[0], t.bounding_box[1], t.bounding_box[2], t.bounding_box[3],
                t.orientation
            ));
            csv.push_str(&format!(
                "{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{},{:.4}\n",
                t.linear_velocity.x, t.linear_velocity.y, t.max_velocity, t.angular_velocity,
                t.min_j, t.max_j, t.avg_j, t.inverted_triangles, t.kinetic_energy
            ));
        }

        csv
    }

    /// Check for anomalies and return descriptions
    pub fn detect_anomalies(&self) -> Vec<String> {
        let mut anomalies = Vec::new();

        for trace in &self.traces {
            let frame = trace.frame;

            // Check for inverted triangles
            if trace.inverted_triangles > 0 {
                anomalies.push(format!(
                    "Frame {}: {} inverted triangles (min_j={:.3})",
                    frame, trace.inverted_triangles, trace.min_j
                ));
            }

            // Check for extreme compression
            if trace.min_j < 0.1 && trace.min_j > 0.0 {
                anomalies.push(format!(
                    "Frame {}: Extreme compression (min_j={:.3})",
                    frame, trace.min_j
                ));
            }

            // Check for extreme stretching
            if trace.max_j > 5.0 {
                anomalies.push(format!(
                    "Frame {}: Extreme stretching (max_j={:.3})",
                    frame, trace.max_j
                ));
            }

            // Check for high angular velocity (spinning)
            if trace.angular_velocity.abs() > 10.0 {
                anomalies.push(format!(
                    "Frame {}: High angular velocity ({:.2} rad/s)",
                    frame, trace.angular_velocity
                ));
            }

            // Check for extreme velocity
            if trace.max_velocity > 100.0 {
                anomalies.push(format!(
                    "Frame {}: Extreme velocity ({:.1} m/s at vertex {})",
                    frame, trace.max_velocity, trace.fastest_vertex.0
                ));
            }
        }

        anomalies
    }

    /// Get statistics over all traced frames
    pub fn statistics(&self) -> TraceStatistics {
        if self.traces.is_empty() {
            return TraceStatistics::default();
        }

        let mut stats = TraceStatistics::default();
        stats.num_frames = self.traces.len();

        let mut min_j_overall = f32::MAX;
        let mut max_j_overall = f32::MIN;
        let mut max_vel_overall = 0.0f32;
        let mut max_angular_overall = 0.0f32;
        let mut total_inverted = 0u32;

        for trace in &self.traces {
            min_j_overall = min_j_overall.min(trace.min_j);
            max_j_overall = max_j_overall.max(trace.max_j);
            max_vel_overall = max_vel_overall.max(trace.max_velocity);
            max_angular_overall = max_angular_overall.max(trace.angular_velocity.abs());
            total_inverted += trace.inverted_triangles;
        }

        stats.min_j_ever = min_j_overall;
        stats.max_j_ever = max_j_overall;
        stats.max_velocity_ever = max_vel_overall;
        stats.max_angular_velocity_ever = max_angular_overall;
        stats.total_inverted_frames = total_inverted;

        // Trajectory: first and last centroid
        if let (Some(first), Some(last)) = (self.traces.front(), self.traces.back()) {
            stats.start_centroid = first.centroid;
            stats.end_centroid = last.centroid;
            stats.total_time = last.time;
        }

        stats
    }
}

#[derive(Clone, Debug, Default)]
pub struct TraceStatistics {
    pub num_frames: usize,
    pub total_time: f32,
    pub min_j_ever: f32,
    pub max_j_ever: f32,
    pub max_velocity_ever: f32,
    pub max_angular_velocity_ever: f32,
    pub total_inverted_frames: u32,
    pub start_centroid: Vec2,
    pub end_centroid: Vec2,
}

impl TraceStatistics {
    pub fn print(&self) {
        println!("\n=== Trace Statistics ===");
        println!("Frames: {}, Time: {:.3}s", self.num_frames, self.total_time);
        println!("J range: [{:.3}, {:.3}]", self.min_j_ever, self.max_j_ever);
        println!("Max velocity: {:.2} m/s", self.max_velocity_ever);
        println!("Max angular velocity: {:.2} rad/s", self.max_angular_velocity_ever);
        println!("Total inverted triangles: {}", self.total_inverted_frames);
        println!("Trajectory: ({:.2}, {:.2}) -> ({:.2}, {:.2})",
            self.start_centroid.x, self.start_centroid.y,
            self.end_centroid.x, self.end_centroid.y
        );

        // Fall distance
        let fall = self.start_centroid.y - self.end_centroid.y;
        if fall > 0.0 && self.total_time > 0.0 {
            let avg_fall_speed = fall / self.total_time;
            println!("Fall: {:.2}m in {:.2}s (avg {:.2} m/s)", fall, self.total_time, avg_fall_speed);
        }
    }

    /// Check if simulation was stable
    pub fn is_stable(&self) -> bool {
        self.min_j_ever > 0.0 &&  // No inversions
        self.max_j_ever < 10.0 && // No extreme stretching
        self.max_velocity_ever < 200.0  // No explosions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracer_basic() {
        let mut tracer = SimulationTracer::new(100);

        // Simple 3-vertex triangle
        let positions = vec![0.0, 0.0, 1.0, 0.0, 0.5, 1.0];
        let velocities = vec![0.0, -1.0, 0.0, -1.0, 0.0, -1.0];
        let triangles = vec![0, 1, 2];
        let rest_areas = vec![0.5];

        tracer.capture_frame(0, 1.0/60.0, &positions, &velocities, &triangles, &rest_areas);

        assert_eq!(tracer.traces().len(), 1);
        let trace = tracer.traces().back().unwrap();
        assert!((trace.centroid.x - 0.5).abs() < 0.01);
        assert!((trace.linear_velocity.y - (-1.0)).abs() < 0.01);
    }
}
