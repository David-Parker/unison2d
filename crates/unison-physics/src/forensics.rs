//! Mesh shape forensics for physics simulation testing
//!
//! Provides advanced metrics to detect permanent deformation, jagged edges,
//! vertex clustering, and other mesh integrity issues that basic KE checks miss.

use crate::xpbd::XPBDSoftBody;

/// Comprehensive snapshot of mesh health at a point in time
#[derive(Clone, Debug)]
pub struct MeshForensics {
    /// Height / original height ratio (1.0 = perfect)
    pub height_ratio: f32,
    /// Width / original width ratio (1.0 = perfect)
    pub width_ratio: f32,
    /// Aspect ratio (width/height)
    pub aspect_ratio: f32,
    /// Original aspect ratio
    pub original_aspect_ratio: f32,
    /// Min edge length / rest length ratio across all edges (< 1.0 = compressed)
    pub min_edge_ratio: f32,
    /// Max edge length / rest length ratio across all edges (> 1.0 = stretched)
    pub max_edge_ratio: f32,
    /// Standard deviation of edge length ratios (high = jagged/uneven)
    pub edge_ratio_stddev: f32,
    /// Number of edges compressed below 50% of rest length
    pub severely_compressed_edges: u32,
    /// Number of edges stretched beyond 150% of rest length
    pub severely_stretched_edges: u32,
    /// Min triangle area / rest area ratio (< 0 = inverted)
    pub min_area_ratio: f32,
    /// Max triangle area / rest area ratio
    pub max_area_ratio: f32,
    /// Number of inverted triangles (area ratio < 0)
    pub inverted_triangles: u32,
    /// Number of triangles compressed below 30% of rest area
    pub collapsed_triangles: u32,
    /// Max angle between adjacent edges on the boundary (detects jagged edges)
    pub max_boundary_angle_deviation: f32,
    /// Ratio of convex hull area to mesh area (< 1.0 = concavities forming)
    pub convexity_ratio: f32,
    /// Vertex clustering metric: min distance between non-adjacent vertices / avg edge length
    /// Low values indicate vertices bunching up
    pub min_vertex_separation: f32,
    /// Center of mass position
    pub center: (f32, f32),
    /// Kinetic energy
    pub kinetic_energy: f32,
    /// Max vertex speed
    pub max_speed: f32,
}

/// Captures the original shape metrics for comparison
#[derive(Clone, Debug)]
pub struct ShapeBaseline {
    pub width: f32,
    pub height: f32,
    pub aspect_ratio: f32,
    pub total_edge_length: f32,
    pub avg_edge_length: f32,
    pub total_area: f32,
    pub num_edges: usize,
    pub num_triangles: usize,
    pub num_verts: usize,
}

impl ShapeBaseline {
    /// Capture baseline from a freshly created body (before any simulation)
    pub fn capture(body: &XPBDSoftBody) -> Self {
        let (min_x, min_y, max_x, max_y) = body.get_aabb();
        let width = max_x - min_x;
        let height = max_y - min_y;

        let total_edge_length: f32 = body.edge_constraints.iter().map(|e| e.rest_length).sum();
        let avg_edge_length = total_edge_length / body.edge_constraints.len().max(1) as f32;
        let total_area: f32 = body.area_constraints.iter().map(|a| a.rest_area).sum();

        ShapeBaseline {
            width,
            height,
            aspect_ratio: if height > 1e-6 { width / height } else { f32::INFINITY },
            total_edge_length,
            avg_edge_length,
            total_area,
            num_edges: body.edge_constraints.len(),
            num_triangles: body.area_constraints.len(),
            num_verts: body.num_verts,
        }
    }
}

impl MeshForensics {
    /// Run full forensic analysis on a body
    pub fn analyze(body: &XPBDSoftBody, baseline: &ShapeBaseline) -> Self {
        let (min_x, min_y, max_x, max_y) = body.get_aabb();
        let width = max_x - min_x;
        let height = max_y - min_y;
        let (cx, cy) = body.get_center();

        // Rotation-aware dimension ratios: compare sorted dimensions so that
        // a rigid body rotation doesn't register as "collapse".
        // We compare (min_dim / original_min_dim) and (max_dim / original_max_dim).
        let current_min_dim = width.min(height);
        let current_max_dim = width.max(height);
        let baseline_min_dim = baseline.width.min(baseline.height);
        let baseline_max_dim = baseline.width.max(baseline.height);

        let width_ratio = current_min_dim / baseline_min_dim.max(1e-6);
        let height_ratio = current_max_dim / baseline_max_dim.max(1e-6);
        let aspect_ratio = if height > 1e-6 { width / height } else { f32::INFINITY };

        // Edge analysis
        let mut min_edge_ratio = f32::MAX;
        let mut max_edge_ratio = f32::MIN;
        let mut edge_ratios = Vec::with_capacity(body.edge_constraints.len());
        let mut severely_compressed = 0u32;
        let mut severely_stretched = 0u32;

        for edge in &body.edge_constraints {
            let dx = body.pos[edge.v1 * 2] - body.pos[edge.v0 * 2];
            let dy = body.pos[edge.v1 * 2 + 1] - body.pos[edge.v0 * 2 + 1];
            let len = (dx * dx + dy * dy).sqrt();
            let ratio = len / edge.rest_length.max(1e-10);

            min_edge_ratio = min_edge_ratio.min(ratio);
            max_edge_ratio = max_edge_ratio.max(ratio);
            edge_ratios.push(ratio);

            if ratio < 0.5 { severely_compressed += 1; }
            if ratio > 1.5 { severely_stretched += 1; }
        }

        // Edge ratio standard deviation
        let mean_ratio: f32 = edge_ratios.iter().sum::<f32>() / edge_ratios.len().max(1) as f32;
        let variance: f32 = edge_ratios.iter()
            .map(|r| (r - mean_ratio) * (r - mean_ratio))
            .sum::<f32>() / edge_ratios.len().max(1) as f32;
        let edge_ratio_stddev = variance.sqrt();

        // Triangle area analysis
        let mut min_area_ratio = f32::MAX;
        let mut max_area_ratio = f32::MIN;
        let mut inverted = 0u32;
        let mut collapsed = 0u32;

        for area_c in &body.area_constraints {
            let x0 = body.pos[area_c.v0 * 2];
            let y0 = body.pos[area_c.v0 * 2 + 1];
            let x1 = body.pos[area_c.v1 * 2];
            let y1 = body.pos[area_c.v1 * 2 + 1];
            let x2 = body.pos[area_c.v2 * 2];
            let y2 = body.pos[area_c.v2 * 2 + 1];

            let signed_area = 0.5 * ((x1 - x0) * (y2 - y0) - (x2 - x0) * (y1 - y0));
            let ratio = signed_area / area_c.rest_area.max(1e-10);

            min_area_ratio = min_area_ratio.min(ratio);
            max_area_ratio = max_area_ratio.max(ratio);

            if ratio < 0.0 { inverted += 1; }
            if ratio.abs() < 0.3 { collapsed += 1; }
        }

        // Boundary angle analysis (detect jagged edges)
        let max_boundary_angle_deviation = compute_boundary_jaggedness(body);

        // Convexity ratio (approximate)
        let convexity_ratio = compute_convexity_ratio(body);

        // Vertex separation (detect clustering)
        let min_vertex_separation = compute_min_vertex_separation(body, baseline.avg_edge_length);

        // Kinetic energy and max speed
        let ke = body.get_kinetic_energy();
        let max_speed = body.get_max_velocity();

        MeshForensics {
            height_ratio,
            width_ratio,
            aspect_ratio,
            original_aspect_ratio: baseline.aspect_ratio,
            min_edge_ratio,
            max_edge_ratio,
            edge_ratio_stddev,
            severely_compressed_edges: severely_compressed,
            severely_stretched_edges: severely_stretched,
            min_area_ratio,
            max_area_ratio,
            inverted_triangles: inverted,
            collapsed_triangles: collapsed,
            max_boundary_angle_deviation,
            convexity_ratio,
            min_vertex_separation,
            center: (cx, cy),
            kinetic_energy: ke,
            max_speed,
        }
    }

    /// Check if the mesh is in a healthy state (no permanent deformation)
    pub fn is_healthy(&self, tolerance: &HealthTolerance) -> Vec<String> {
        let mut issues = Vec::new();

        if self.height_ratio < tolerance.min_dimension_ratio {
            issues.push(format!(
                "Height collapsed to {:.1}% (min: {:.1}%)",
                self.height_ratio * 100.0, tolerance.min_dimension_ratio * 100.0
            ));
        }
        if self.width_ratio < tolerance.min_dimension_ratio {
            issues.push(format!(
                "Width collapsed to {:.1}% (min: {:.1}%)",
                self.width_ratio * 100.0, tolerance.min_dimension_ratio * 100.0
            ));
        }
        if self.min_edge_ratio < tolerance.min_edge_ratio {
            issues.push(format!(
                "Edge over-compressed: {:.1}% of rest (min: {:.1}%)",
                self.min_edge_ratio * 100.0, tolerance.min_edge_ratio * 100.0
            ));
        }
        if self.max_edge_ratio > tolerance.max_edge_ratio {
            issues.push(format!(
                "Edge over-stretched: {:.1}% of rest (max: {:.1}%)",
                self.max_edge_ratio * 100.0, tolerance.max_edge_ratio * 100.0
            ));
        }
        if self.edge_ratio_stddev > tolerance.max_edge_stddev {
            issues.push(format!(
                "Jagged edges: stddev={:.3} (max: {:.3})",
                self.edge_ratio_stddev, tolerance.max_edge_stddev
            ));
        }
        if self.severely_compressed_edges > tolerance.max_severely_compressed {
            issues.push(format!(
                "{} severely compressed edges (max: {})",
                self.severely_compressed_edges, tolerance.max_severely_compressed
            ));
        }
        if self.inverted_triangles > 0 {
            issues.push(format!("{} inverted triangles", self.inverted_triangles));
        }
        if self.collapsed_triangles > tolerance.max_collapsed_triangles {
            issues.push(format!(
                "{} collapsed triangles (max: {})",
                self.collapsed_triangles, tolerance.max_collapsed_triangles
            ));
        }
        if self.min_area_ratio < tolerance.min_area_ratio {
            issues.push(format!(
                "Triangle area ratio {:.3} below minimum {:.3}",
                self.min_area_ratio, tolerance.min_area_ratio
            ));
        }
        if self.min_vertex_separation < tolerance.min_vertex_separation {
            issues.push(format!(
                "Vertex clustering: min separation {:.3} (min: {:.3})",
                self.min_vertex_separation, tolerance.min_vertex_separation
            ));
        }

        issues
    }

    /// One-line summary for logging
    pub fn summary(&self) -> String {
        format!(
            "dim={:.0}%x{:.0}% edge=[{:.2},{:.2}]±{:.3} area=[{:.2},{:.2}] inv={} col={} KE={:.1}",
            self.width_ratio * 100.0, self.height_ratio * 100.0,
            self.min_edge_ratio, self.max_edge_ratio, self.edge_ratio_stddev,
            self.min_area_ratio, self.max_area_ratio,
            self.inverted_triangles, self.collapsed_triangles,
            self.kinetic_energy,
        )
    }
}

/// Configurable health thresholds
#[derive(Clone, Debug)]
pub struct HealthTolerance {
    pub min_dimension_ratio: f32,
    pub min_edge_ratio: f32,
    pub max_edge_ratio: f32,
    pub max_edge_stddev: f32,
    pub max_severely_compressed: u32,
    pub min_area_ratio: f32,
    pub max_collapsed_triangles: u32,
    pub min_vertex_separation: f32,
}

impl HealthTolerance {
    /// Strict tolerance for settled bodies (post-simulation)
    pub fn strict() -> Self {
        Self {
            min_dimension_ratio: 0.70,
            min_edge_ratio: 0.30,
            max_edge_ratio: 2.0,
            max_edge_stddev: 0.25,
            max_severely_compressed: 2,
            min_area_ratio: 0.05,
            max_collapsed_triangles: 3,
            min_vertex_separation: 0.10,
        }
    }

    /// Relaxed tolerance for mid-collision snapshots
    pub fn during_collision() -> Self {
        Self {
            min_dimension_ratio: 0.40,
            min_edge_ratio: 0.15,
            max_edge_ratio: 3.0,
            max_edge_stddev: 0.40,
            max_severely_compressed: 10,
            min_area_ratio: -0.05,
            max_collapsed_triangles: 10,
            min_vertex_separation: 0.03,
        }
    }

    /// Very soft material tolerance (more deformation expected)
    pub fn soft_material() -> Self {
        Self {
            min_dimension_ratio: 0.55,
            min_edge_ratio: 0.20,
            max_edge_ratio: 2.5,
            max_edge_stddev: 0.35,
            max_severely_compressed: 5,
            min_area_ratio: 0.0,
            max_collapsed_triangles: 5,
            min_vertex_separation: 0.05,
        }
    }
}

/// Compute boundary jaggedness by looking at angle changes along the outer ring
fn compute_boundary_jaggedness(body: &XPBDSoftBody) -> f32 {
    // Find boundary edges (edges that belong to only one triangle)
    let mut edge_tri_count = std::collections::HashMap::new();
    for area_c in &body.area_constraints {
        let verts = [area_c.v0, area_c.v1, area_c.v2];
        for k in 0..3 {
            let a = verts[k].min(verts[(k + 1) % 3]);
            let b = verts[k].max(verts[(k + 1) % 3]);
            *edge_tri_count.entry((a, b)).or_insert(0u32) += 1;
        }
    }

    let boundary_edges: Vec<(usize, usize)> = edge_tri_count.iter()
        .filter(|(_, &count)| count == 1)
        .map(|(&(a, b), _)| (a, b))
        .collect();

    if boundary_edges.len() < 3 {
        return 0.0;
    }

    // Build adjacency for boundary vertices
    let mut adj: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    for &(a, b) in &boundary_edges {
        adj.entry(a).or_default().push(b);
        adj.entry(b).or_default().push(a);
    }

    // Walk the boundary and compute angle deviations
    let mut max_deviation = 0.0f32;

    for (&v, neighbors) in &adj {
        if neighbors.len() != 2 { continue; }
        let prev = neighbors[0];
        let next = neighbors[1];

        let px = body.pos[prev * 2];
        let py = body.pos[prev * 2 + 1];
        let vx = body.pos[v * 2];
        let vy = body.pos[v * 2 + 1];
        let nx = body.pos[next * 2];
        let ny = body.pos[next * 2 + 1];

        let d1x = vx - px;
        let d1y = vy - py;
        let d2x = nx - vx;
        let d2y = ny - vy;

        let len1 = (d1x * d1x + d1y * d1y).sqrt();
        let len2 = (d2x * d2x + d2y * d2y).sqrt();
        if len1 < 1e-8 || len2 < 1e-8 { continue; }

        let dot = (d1x * d2x + d1y * d2y) / (len1 * len2);
        let angle = dot.clamp(-1.0, 1.0).acos();
        // Deviation from straight (π radians)
        let deviation = (std::f32::consts::PI - angle).abs();
        max_deviation = max_deviation.max(deviation);
    }

    max_deviation
}

/// Approximate convexity ratio using bounding box area vs actual mesh area
fn compute_convexity_ratio(body: &XPBDSoftBody) -> f32 {
    // Compute actual mesh area from triangles
    let mut mesh_area = 0.0f32;
    for area_c in &body.area_constraints {
        let x0 = body.pos[area_c.v0 * 2];
        let y0 = body.pos[area_c.v0 * 2 + 1];
        let x1 = body.pos[area_c.v1 * 2];
        let y1 = body.pos[area_c.v1 * 2 + 1];
        let x2 = body.pos[area_c.v2 * 2];
        let y2 = body.pos[area_c.v2 * 2 + 1];
        let area = 0.5 * ((x1 - x0) * (y2 - y0) - (x2 - x0) * (y1 - y0));
        mesh_area += area.abs();
    }

    let (min_x, min_y, max_x, max_y) = body.get_aabb();
    let bbox_area = (max_x - min_x) * (max_y - min_y);

    if bbox_area < 1e-10 { return 0.0; }
    (mesh_area / bbox_area).min(1.0)
}

/// Find minimum distance between non-adjacent vertices, normalized by avg edge length
fn compute_min_vertex_separation(body: &XPBDSoftBody, avg_edge_len: f32) -> f32 {
    if body.num_verts < 2 || avg_edge_len < 1e-10 { return 1.0; }

    // Build adjacency set
    let mut adjacent = std::collections::HashSet::new();
    for edge in &body.edge_constraints {
        let a = edge.v0.min(edge.v1);
        let b = edge.v0.max(edge.v1);
        adjacent.insert((a, b));
    }

    let mut min_dist = f32::MAX;

    // Sample pairs (full O(n²) is fine for test meshes)
    for i in 0..body.num_verts {
        for j in (i + 1)..body.num_verts {
            let a = i.min(j);
            let b = i.max(j);
            if adjacent.contains(&(a, b)) { continue; }

            let dx = body.pos[j * 2] - body.pos[i * 2];
            let dy = body.pos[j * 2 + 1] - body.pos[i * 2 + 1];
            let dist = (dx * dx + dy * dy).sqrt();
            min_dist = min_dist.min(dist);
        }
    }

    if min_dist == f32::MAX { return 1.0; }
    min_dist / avg_edge_len
}

/// Run a simulation and collect forensics at regular intervals
pub struct ForensicSimulation {
    pub snapshots: Vec<(u32, MeshForensics)>,
    pub baseline: ShapeBaseline,
}

impl ForensicSimulation {
    /// Run a single-body simulation with forensic capture
    pub fn run_single(
        body: &mut XPBDSoftBody,
        baseline: &ShapeBaseline,
        frames: u32,
        substeps: u32,
        dt: f32,
        gravity: f32,
        ground_y: Option<f32>,
        friction: f32,
        restitution: f32,
        capture_interval: u32,
    ) -> Self {
        let substep_dt = dt / substeps as f32;
        let mut snapshots = Vec::new();

        for frame in 0..frames {
            for _ in 0..substeps {
                body.substep_pre_with_friction(substep_dt, gravity, ground_y, friction, restitution);
                body.substep_post(substep_dt);
            }
            body.apply_damping(0.005);

            if frame % capture_interval == 0 || frame == frames - 1 {
                let forensics = MeshForensics::analyze(body, baseline);
                snapshots.push((frame, forensics));
            }
        }

        ForensicSimulation {
            snapshots,
            baseline: baseline.clone(),
        }
    }

    /// Get the final snapshot
    pub fn final_snapshot(&self) -> Option<&MeshForensics> {
        self.snapshots.last().map(|(_, f)| f)
    }

    /// Check if any snapshot violated health during simulation
    pub fn worst_issues(&self, tolerance: &HealthTolerance) -> Vec<(u32, Vec<String>)> {
        self.snapshots.iter()
            .filter_map(|(frame, forensics)| {
                let issues = forensics.is_healthy(tolerance);
                if issues.is_empty() { None } else { Some((*frame, issues)) }
            })
            .collect()
    }

    /// Print a summary table
    pub fn print_summary(&self) {
        println!("{:>6} | {}", "Frame", "Forensics");
        println!("{}", "-".repeat(90));
        for (frame, f) in &self.snapshots {
            println!("{:>6} | {}", frame, f.summary());
        }
    }
}
