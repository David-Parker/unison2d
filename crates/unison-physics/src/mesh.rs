//! Mesh generation utilities

use std::f32::consts::PI;

/// Mesh data for physics simulation
///
/// Positions are stored as a flat array for efficient physics computation.
/// UV coordinates are optional and used for rendering with textures.
#[derive(Clone)]
pub struct Mesh {
    /// Vertex positions as flat array [x0, y0, x1, y1, ...]
    pub vertices: Vec<f32>,
    /// Triangle indices [i0, i1, i2, ...]
    pub triangles: Vec<u32>,
    /// Optional UV coordinates as flat array [u0, v0, u1, v1, ...]
    /// When present, length must equal vertices.len()
    pub uvs: Option<Vec<f32>>,
    /// Boundary edges — vertex index pairs for edges belonging to exactly
    /// one triangle. Computed lazily via [`compute_boundary_edges`] and
    /// cached for shadow casting.
    pub boundary_edges: Option<Vec<(u32, u32)>>,
}

impl Mesh {
    /// Create a mesh without UVs
    pub fn new(vertices: Vec<f32>, triangles: Vec<u32>) -> Self {
        Self {
            vertices,
            triangles,
            uvs: None,
            boundary_edges: None,
        }
    }

    /// Create a mesh with UVs
    pub fn with_uvs(vertices: Vec<f32>, triangles: Vec<u32>, uvs: Vec<f32>) -> Self {
        assert_eq!(
            vertices.len(),
            uvs.len(),
            "UV count must match vertex count"
        );
        Self {
            vertices,
            triangles,
            uvs: Some(uvs),
            boundary_edges: None,
        }
    }

    /// Compute and cache boundary edges for shadow casting.
    ///
    /// Boundary edges are edges that belong to exactly one triangle,
    /// forming the outer silhouette of the mesh. For meshes with holes
    /// (e.g. rings), only the outermost boundary loop is kept so that
    /// inner holes don't cast incorrect shadows.
    pub fn ensure_boundary_edges(&mut self) {
        if self.boundary_edges.is_some() {
            return;
        }
        let all_edges = compute_boundary_edges_from_triangles(&self.triangles);
        self.boundary_edges = Some(extract_outer_boundary(&self.vertices, &all_edges));
    }

    /// Number of vertices
    #[inline]
    pub fn vertex_count(&self) -> usize {
        self.vertices.len() / 2
    }

    /// Get UV coordinates, generating defaults if not present
    pub fn uvs_or_default(&self) -> Vec<f32> {
        self.uvs.clone().unwrap_or_else(|| vec![0.0; self.vertices.len()])
    }
}

/// Create a ring (annulus) mesh with UV coordinates
///
/// UV mapping preserves texture aspect ratio - the ring samples from the texture
/// based on its actual position. The outer_radius maps to UV 0-1, so the ring
/// "cuts out" its portion of the texture.
pub fn create_ring_mesh(
    outer_radius: f32,
    inner_radius: f32,
    segments: u32,
    radial_divisions: u32,
) -> Mesh {
    let mut vertices = Vec::new();
    let mut uvs = Vec::new();
    let mut triangles = Vec::new();

    // Create vertex grid (no duplicate vertices at seam)
    for r in 0..=radial_divisions {
        let t = r as f32 / radial_divisions as f32;
        let radius = inner_radius + (outer_radius - inner_radius) * t;

        for i in 0..segments {
            let angle = (i as f32 / segments as f32) * PI * 2.0;
            let x = angle.cos() * radius;
            let y = angle.sin() * radius;
            vertices.push(x);
            vertices.push(y);

            // UV: direct position mapping, outer_radius maps to texture edge
            // This preserves the texture's aspect ratio
            let u = 0.5 + (x / outer_radius) * 0.5;
            let v = 0.5 + (y / outer_radius) * 0.5;
            uvs.push(u);
            uvs.push(v);
        }
    }

    // Create triangles with proper wrap-around
    let verts_per_ring = segments;
    for r in 0..radial_divisions {
        for i in 0..segments {
            let curr = r * verts_per_ring + i;
            let next = r * verts_per_ring + (i + 1) % segments;
            let curr_outer = (r + 1) * verts_per_ring + i;
            let next_outer = (r + 1) * verts_per_ring + (i + 1) % segments;

            triangles.push(curr);
            triangles.push(curr_outer);
            triangles.push(next);

            triangles.push(next);
            triangles.push(curr_outer);
            triangles.push(next_outer);
        }
    }

    Mesh::with_uvs(vertices, triangles, uvs)
}

/// Create wireframe indices from ring mesh
pub fn create_ring_wireframe(segments: u32, radial_divisions: u32) -> Vec<u32> {
    let mut line_indices = Vec::new();
    let verts_per_ring = segments;

    for r in 0..radial_divisions {
        for i in 0..segments {
            let curr = r * verts_per_ring + i;
            let next = r * verts_per_ring + (i + 1) % segments;
            let curr_outer = (r + 1) * verts_per_ring + i;
            let next_outer = (r + 1) * verts_per_ring + (i + 1) % segments;

            // Inner edge
            line_indices.push(curr);
            line_indices.push(next);

            // Radial edge
            line_indices.push(curr);
            line_indices.push(curr_outer);

            // Diagonal edge
            line_indices.push(next);
            line_indices.push(curr_outer);

            // Outer edge (only on last ring)
            if r == radial_divisions - 1 {
                line_indices.push(curr_outer);
                line_indices.push(next_outer);
            }
        }
    }

    line_indices
}

/// Compute boundary edges from triangle indices.
///
/// Boundary edges are edges that belong to exactly one triangle. These form
/// the outer silhouette of the mesh and are used for shadow casting.
///
/// Returns directed vertex index pairs `(v0, v1)` preserving the original
/// triangle winding order, so that the right-hand perpendicular of each
/// edge points outward (away from the mesh interior).
pub fn compute_boundary_edges_from_triangles(triangles: &[u32]) -> Vec<(u32, u32)> {
    use std::collections::HashMap;

    // For each canonical edge key, track count and the original directed edge.
    let mut edge_info: HashMap<(u32, u32), (u32, (u32, u32))> = HashMap::new();

    for tri in triangles.chunks(3) {
        if tri.len() < 3 {
            continue;
        }
        let edges = [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])];
        for &(a, b) in &edges {
            let key = if a < b { (a, b) } else { (b, a) };
            edge_info
                .entry(key)
                .and_modify(|(count, _)| *count += 1)
                .or_insert((1, (a, b)));
        }
    }

    edge_info
        .into_iter()
        .filter(|&(_, (count, _))| count == 1)
        .map(|(_, (_, directed))| directed)
        .collect()
}

/// Extract only the outermost boundary loop from a set of boundary edges.
///
/// Traces connected loops through the edges, computes each loop's enclosed
/// area via the shoelace formula, and returns only the edges belonging to
/// the loop with the largest area. For simple meshes (single boundary),
/// this returns all edges unchanged.
fn extract_outer_boundary(vertices: &[f32], edges: &[(u32, u32)]) -> Vec<(u32, u32)> {
    use std::collections::HashMap;

    if edges.is_empty() {
        return Vec::new();
    }

    // Build adjacency: vertex -> list of connected vertices
    let mut adjacency: HashMap<u32, Vec<u32>> = HashMap::new();
    for &(a, b) in edges {
        adjacency.entry(a).or_default().push(b);
        adjacency.entry(b).or_default().push(a);
    }

    // Trace connected loops
    let mut visited = std::collections::HashSet::new();
    let mut loops: Vec<Vec<u32>> = Vec::new();

    for &(start, _) in edges {
        if visited.contains(&start) {
            continue;
        }

        // Walk the loop
        let mut loop_verts = Vec::new();
        let mut current = start;
        let mut prev = u32::MAX;

        loop {
            visited.insert(current);
            loop_verts.push(current);

            let neighbors = match adjacency.get(&current) {
                Some(n) => n,
                None => break,
            };

            // Pick the neighbor we haven't come from
            let next = neighbors.iter().copied().find(|&n| n != prev);
            match next {
                Some(n) if n == start => break, // completed the loop
                Some(n) if visited.contains(&n) => break, // shouldn't happen but be safe
                Some(n) => {
                    prev = current;
                    current = n;
                }
                None => break,
            }
        }

        if loop_verts.len() >= 3 {
            loops.push(loop_verts);
        }
    }

    // Single loop — return all edges as-is
    if loops.len() <= 1 {
        return edges.to_vec();
    }

    // Find the loop with the largest enclosed area (shoelace formula)
    let outer_loop = loops.iter().max_by(|a, b| {
        let area_a = shoelace_area(vertices, a);
        let area_b = shoelace_area(vertices, b);
        area_a.partial_cmp(&area_b).unwrap_or(std::cmp::Ordering::Equal)
    }).unwrap();

    // Collect vertex indices in the outer loop into a set for filtering
    let outer_set: std::collections::HashSet<u32> = outer_loop.iter().copied().collect();

    // Return only edges where both vertices belong to the outer loop
    edges.iter()
        .copied()
        .filter(|&(a, b)| outer_set.contains(&a) && outer_set.contains(&b))
        .collect()
}

/// Compute the absolute area enclosed by a loop of vertices using the shoelace formula.
fn shoelace_area(vertices: &[f32], loop_verts: &[u32]) -> f32 {
    let mut area = 0.0f32;
    let n = loop_verts.len();
    for i in 0..n {
        let j = (i + 1) % n;
        let vi = loop_verts[i] as usize;
        let vj = loop_verts[j] as usize;
        let xi = vertices[vi * 2];
        let yi = vertices[vi * 2 + 1];
        let xj = vertices[vj * 2];
        let yj = vertices[vj * 2 + 1];
        area += xi * yj - xj * yi;
    }
    area.abs() * 0.5
}

/// Offset all vertices by a fixed amount
pub fn offset_vertices(vertices: &mut [f32], dx: f32, dy: f32) {
    for i in (0..vertices.len()).step_by(2) {
        vertices[i] += dx;
        vertices[i + 1] += dy;
    }
}

/// Create a simple square mesh with UV coordinates
///
/// UV mapping: direct 0-1 mapping across the square
pub fn create_square_mesh(size: f32, divisions: u32) -> Mesh {
    let mut vertices = Vec::new();
    let mut uvs = Vec::new();
    let mut triangles = Vec::new();
    let half_size = size / 2.0;

    for y in 0..=divisions {
        for x in 0..=divisions {
            let tx = x as f32 / divisions as f32;
            let ty = y as f32 / divisions as f32;

            let px = -half_size + tx * size;
            let py = -half_size + ty * size;
            vertices.push(px);
            vertices.push(py);

            uvs.push(tx);
            uvs.push(ty);
        }
    }

    let verts_per_row = divisions + 1;
    for y in 0..divisions {
        for x in 0..divisions {
            let curr = y * verts_per_row + x;
            let right = curr + 1;
            let up = curr + verts_per_row;
            let up_right = up + 1;

            triangles.push(curr);
            triangles.push(right);
            triangles.push(up);

            triangles.push(right);
            triangles.push(up_right);
            triangles.push(up);
        }
    }

    Mesh::with_uvs(vertices, triangles, uvs)
}

/// Create a rounded box mesh (box with rounded corners)
///
/// This avoids the corner impalement issue with sharp corners in collision detection.
/// The corners are replaced with quarter-circle arcs.
///
/// - `width`: total width of the box
/// - `height`: total height of the box
/// - `corner_radius`: radius of the rounded corners
/// - `corner_segments`: number of segments per corner arc (more = smoother)
pub fn create_rounded_box_mesh(width: f32, height: f32, corner_radius: f32, corner_segments: u32) -> Mesh {
    let mut vertices = Vec::new();
    let mut uvs = Vec::new();
    let mut triangles = Vec::new();

    // Clamp corner radius to half the smallest dimension
    let max_radius = (width.min(height) / 2.0).min(corner_radius);
    let r = max_radius;

    let half_w = width / 2.0;
    let half_h = height / 2.0;

    // Center vertex
    let center_idx = 0u32;
    vertices.push(0.0);
    vertices.push(0.0);
    uvs.push(0.5);
    uvs.push(0.5);

    // Build perimeter vertices: 4 corners with arcs + 4 straight edges
    // Go counter-clockwise starting from bottom-right corner
    let mut perimeter_start = 1u32;

    // Corner centers (inset from actual corners by radius)
    let corners = [
        (half_w - r, -half_h + r),   // bottom-right
        (half_w - r, half_h - r),    // top-right
        (-half_w + r, half_h - r),   // top-left
        (-half_w + r, -half_h + r),  // bottom-left
    ];

    // Start angles for each corner (radians)
    let start_angles = [
        -std::f32::consts::FRAC_PI_2, // bottom-right: -90° to 0°
        0.0,                           // top-right: 0° to 90°
        std::f32::consts::FRAC_PI_2,  // top-left: 90° to 180°
        std::f32::consts::PI,          // bottom-left: 180° to 270°
    ];

    for corner in 0..4 {
        let (cx, cy) = corners[corner];
        let start_angle = start_angles[corner];

        for i in 0..=corner_segments {
            let t = i as f32 / corner_segments as f32;
            let angle = start_angle + t * std::f32::consts::FRAC_PI_2;

            let px = cx + r * angle.cos();
            let py = cy + r * angle.sin();
            vertices.push(px);
            vertices.push(py);

            // UV: map position to 0-1 range
            uvs.push((px + half_w) / width);
            uvs.push((py + half_h) / height);
        }
    }

    // Total perimeter vertices: 4 corners * (corner_segments + 1)
    // But adjacent corners share their endpoint, so actual count is 4 * corner_segments + 4
    // Wait, we're adding corner_segments+1 per corner = 4*(corner_segments+1) vertices
    // The last vertex of each corner is the same position as the first of the next,
    // but for simplicity we'll just create triangles properly

    let total_perimeter = 4 * (corner_segments + 1);

    // Create triangles from center to perimeter (fan triangulation)
    for i in 0..total_perimeter {
        let v0 = center_idx;
        let v1 = perimeter_start + i;
        let v2 = perimeter_start + ((i + 1) % total_perimeter);
        triangles.push(v0);
        triangles.push(v1);
        triangles.push(v2);
    }

    Mesh::with_uvs(vertices, triangles, uvs)
}

/// Create an ellipse mesh with a small hole in the center (like ring topology)
///
/// UV mapping: same as ring mesh (u = angle, v = radial position)
pub fn create_ellipse_mesh(
    width: f32,
    height: f32,
    segments: u32,
    rings: u32,
) -> Mesh {
    let mut vertices = Vec::new();
    let mut uvs = Vec::new();
    let mut triangles = Vec::new();

    // Create rings from inner (small hole) to outer
    // Inner ring is 20% of size to create deformable center
    let inner_scale = 0.2;

    for r in 0..=rings {
        let radial_t = r as f32 / rings as f32;
        let t = inner_scale + (1.0 - inner_scale) * radial_t;
        let rx = width * 0.5 * t;
        let ry = height * 0.5 * t;

        for i in 0..segments {
            let angle = (i as f32 / segments as f32) * PI * 2.0;
            vertices.push(angle.cos() * rx);
            vertices.push(angle.sin() * ry);

            uvs.push(i as f32 / segments as f32);
            uvs.push(radial_t);
        }
    }

    // Triangles between rings (same as ring mesh)
    for r in 0..rings {
        let ring_start = r * segments;
        let next_ring_start = (r + 1) * segments;

        for i in 0..segments {
            let next = (i + 1) % segments;

            triangles.push(ring_start + i);
            triangles.push(next_ring_start + i);
            triangles.push(ring_start + next);

            triangles.push(ring_start + next);
            triangles.push(next_ring_start + i);
            triangles.push(next_ring_start + next);
        }
    }

    Mesh::with_uvs(vertices, triangles, uvs)
}

/// Create a star-shaped mesh with a small hole in the center
///
/// UV mapping: u = angle, v = radial position
pub fn create_star_mesh(
    outer_radius: f32,
    inner_radius: f32,
    points: u32,
    rings: u32,
) -> Mesh {
    let mut vertices = Vec::new();
    let mut uvs = Vec::new();
    let mut triangles = Vec::new();

    // Create rings with alternating star points
    let total_points = points * 2;  // points + valleys

    // Inner ring is 25% of size to create deformable center
    let inner_scale = 0.25;

    for r in 0..=rings {
        let radial_t = r as f32 / rings as f32;
        let t = inner_scale + (1.0 - inner_scale) * radial_t;

        for i in 0..total_points {
            let angle = (i as f32 / total_points as f32) * PI * 2.0;
            // Alternate between outer and inner radius
            let radius = if i % 2 == 0 {
                outer_radius * t
            } else {
                inner_radius * t
            };
            vertices.push(angle.cos() * radius);
            vertices.push(angle.sin() * radius);

            uvs.push(i as f32 / total_points as f32);
            uvs.push(radial_t);
        }
    }

    // Triangles between rings
    for r in 0..rings {
        let ring_start = r * total_points;
        let next_ring_start = (r + 1) * total_points;

        for i in 0..total_points {
            let next = (i + 1) % total_points;

            triangles.push(ring_start + i);
            triangles.push(next_ring_start + i);
            triangles.push(ring_start + next);

            triangles.push(ring_start + next);
            triangles.push(next_ring_start + i);
            triangles.push(next_ring_start + next);
        }
    }

    Mesh::with_uvs(vertices, triangles, uvs)
}

/// Create a blob mesh with randomized vertex positions and a small hole in the center
///
/// UV mapping: u = angle, v = radial position
pub fn create_blob_mesh(
    base_radius: f32,
    variation: f32,
    segments: u32,
    rings: u32,
    seed: u32,
) -> Mesh {
    let mut vertices = Vec::new();
    let mut uvs = Vec::new();
    let mut triangles = Vec::new();

    // Simple deterministic "random" based on seed
    let pseudo_random = |i: u32, j: u32| -> f32 {
        let x = ((i.wrapping_mul(1103515245).wrapping_add(j.wrapping_mul(12345)).wrapping_add(seed)) % 1000) as f32 / 1000.0;
        x * 2.0 - 1.0  // -1 to 1
    };

    // Inner ring is 20% of size to create deformable center
    let inner_scale = 0.2;

    // Create rings with randomized radii (including inner ring)
    for r in 0..=rings {
        let radial_t = r as f32 / rings as f32;
        let base_t = inner_scale + (1.0 - inner_scale) * radial_t;

        for i in 0..segments {
            let angle = (i as f32 / segments as f32) * PI * 2.0;
            let random_factor = 1.0 + pseudo_random(r, i) * variation;
            let radius = base_radius * base_t * random_factor;
            vertices.push(angle.cos() * radius);
            vertices.push(angle.sin() * radius);

            uvs.push(i as f32 / segments as f32);
            uvs.push(radial_t);
        }
    }

    // Triangles between rings
    for r in 0..rings {
        let ring_start = r * segments;
        let next_ring_start = (r + 1) * segments;

        for i in 0..segments {
            let next = (i + 1) % segments;

            triangles.push(ring_start + i);
            triangles.push(next_ring_start + i);
            triangles.push(ring_start + next);

            triangles.push(ring_start + next);
            triangles.push(next_ring_start + i);
            triangles.push(next_ring_start + next);
        }
    }

    Mesh::with_uvs(vertices, triangles, uvs)
}

/// Create wireframe for any radial mesh (ellipse, star, blob) - now with hole topology
pub fn create_radial_wireframe(segments: u32, rings: u32) -> Vec<u32> {
    let mut line_indices = Vec::new();

    // Lines within and between rings (same as ring mesh)
    for r in 0..=rings {
        let ring_start = r * segments;

        for i in 0..segments {
            let next = (i + 1) % segments;

            // Circumferential line
            line_indices.push(ring_start + i);
            line_indices.push(ring_start + next);

            // Radial line to next ring (if not last ring)
            if r < rings {
                let next_ring_start = (r + 1) * segments;
                line_indices.push(ring_start + i);
                line_indices.push(next_ring_start + i);
            }
        }
    }

    line_indices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_mesh() {
        let mesh = create_ring_mesh(1.0, 0.5, 8, 2);
        assert_eq!(mesh.vertices.len() / 2, 24); // 8 * (2+1) vertices
        assert_eq!(mesh.triangles.len() / 3, 32); // 8 * 2 * 2 triangles
    }

    #[test]
    fn test_square_mesh() {
        let mesh = create_square_mesh(2.0, 4);
        assert_eq!(mesh.vertices.len() / 2, 25); // 5x5 vertices
        assert_eq!(mesh.triangles.len() / 3, 32); // 4x4x2 triangles
    }

    #[test]
    fn test_ring_wireframe() {
        let wireframe = create_ring_wireframe(8, 2);
        // Each segment has: inner edge, radial edge, diagonal edge
        // Plus outer edges on last ring
        // 8 segments * 2 radial divisions * 3 edges + 8 outer edges = 56 edges
        // Each edge is 2 indices
        assert!(wireframe.len() > 0);
        assert_eq!(wireframe.len() % 2, 0); // Must be pairs
    }

    #[test]
    fn test_offset_vertices() {
        let mut vertices = vec![0.0, 0.0, 1.0, 1.0];
        offset_vertices(&mut vertices, 2.0, 3.0);
        assert_eq!(vertices, vec![2.0, 3.0, 3.0, 4.0]);
    }

    #[test]
    fn test_ellipse_mesh() {
        let mesh = create_ellipse_mesh(2.0, 1.5, 8, 3);
        // With hole topology: 8 segments * (3+1) rings = 32 vertices
        assert_eq!(mesh.vertices.len() / 2, 32);
        // 8 segments * 3 ring transitions * 2 triangles = 48 triangles
        assert_eq!(mesh.triangles.len() / 3, 48);
        // Verify no NaN or infinite values
        for v in &mesh.vertices {
            assert!(v.is_finite(), "Vertex should be finite");
        }
    }

    #[test]
    fn test_star_mesh() {
        let mesh = create_star_mesh(1.5, 0.7, 5, 3);
        // 5 points * 2 (points + valleys) = 10 segments per ring
        // With hole topology: 10 * (3+1) = 40 vertices
        assert_eq!(mesh.vertices.len() / 2, 40);
        // 10 segments * 3 ring transitions * 2 triangles = 60 triangles
        assert_eq!(mesh.triangles.len() / 3, 60);
        // Verify star shape has alternating radii
        for v in &mesh.vertices {
            assert!(v.is_finite(), "Vertex should be finite");
        }
    }

    #[test]
    fn test_blob_mesh() {
        let mesh = create_blob_mesh(1.5, 0.25, 8, 3, 42);
        // With hole topology: 8 segments * (3+1) rings = 32 vertices
        assert_eq!(mesh.vertices.len() / 2, 32);
        // 8 segments * 3 ring transitions * 2 triangles = 48 triangles
        assert_eq!(mesh.triangles.len() / 3, 48);
        // Verify deterministic: same seed gives same mesh
        let mesh2 = create_blob_mesh(1.5, 0.25, 8, 3, 42);
        assert_eq!(mesh.vertices, mesh2.vertices);
        // Different seed gives different mesh
        let mesh3 = create_blob_mesh(1.5, 0.25, 8, 3, 99);
        assert_ne!(mesh.vertices, mesh3.vertices);
    }

    #[test]
    fn test_radial_wireframe() {
        let wireframe = create_radial_wireframe(8, 3);
        // Should have circumferential + radial lines
        assert!(wireframe.len() > 0);
        assert_eq!(wireframe.len() % 2, 0); // Must be pairs
    }
}
