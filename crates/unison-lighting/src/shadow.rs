//! Shadow geometry computation.
//!
//! Given a light source and a set of occluders, computes shadow quads —
//! polygons that represent the shadowed regions behind each occluder edge.

use crate::occluder::{Occluder, OccluderEdge};

/// A projected shadow polygon (2 triangles, 4 vertices).
///
/// Vertices are in world space and should be drawn as a solid black quad
/// to the shadow mask FBO.
#[derive(Debug, Clone)]
pub struct ShadowQuad {
    /// 4 vertices × 2 components = 8 floats: [ax, ay, bx, by, b'x, b'y, a'x, a'y]
    pub positions: [f32; 8],
    /// Triangle indices (always [0,1,2, 0,2,3]).
    pub indices: [u32; 6],
}

impl ShadowQuad {
    fn new(a: [f32; 2], b: [f32; 2], b_proj: [f32; 2], a_proj: [f32; 2]) -> Self {
        Self {
            positions: [a[0], a[1], b[0], b[1], b_proj[0], b_proj[1], a_proj[0], a_proj[1]],
            indices: [0, 1, 2, 0, 2, 3],
        }
    }
}

/// Check if an edge is back-facing relative to a point light.
///
/// An edge is back-facing when its outward normal points away from the light
/// (i.e., the light is "behind" the edge from the exterior's perspective).
/// These are the edges that cast shadows.
pub fn is_back_facing_point(edge: &OccluderEdge, light_pos: [f32; 2]) -> bool {
    // Vector from edge midpoint to light
    let mid_x = (edge.a[0] + edge.b[0]) * 0.5;
    let mid_y = (edge.a[1] + edge.b[1]) * 0.5;
    let to_light_x = light_pos[0] - mid_x;
    let to_light_y = light_pos[1] - mid_y;

    // If normal · to_light < 0, the normal points away from the light → back-facing
    let dot = edge.normal[0] * to_light_x + edge.normal[1] * to_light_y;
    dot < 0.0
}

/// Check if an edge is back-facing relative to a directional light.
///
/// For directional lights, the light direction is the direction light travels
/// (from source toward scene). An edge is back-facing if its normal aligns
/// with the light direction (points in the same general direction the light
/// travels).
pub fn is_back_facing_directional(edge: &OccluderEdge, light_direction: [f32; 2]) -> bool {
    // If normal · direction > 0, the normal points in the light's travel direction
    // → the edge faces away from the light source → back-facing → casts shadow
    let dot = edge.normal[0] * light_direction[0] + edge.normal[1] * light_direction[1];
    dot > 0.0
}

/// Compute shadow quads for a point light.
///
/// For each back-facing edge of each occluder, projects the edge endpoints
/// radially away from the light to form a shadow quad.
pub fn project_point_shadows(
    light_pos: [f32; 2],
    light_radius: f32,
    occluders: &[Occluder],
) -> Vec<ShadowQuad> {
    let mut quads = Vec::new();

    for occluder in occluders {
        for edge in &occluder.edges {
            if !is_back_facing_point(edge, light_pos) {
                continue;
            }

            let a_proj = project_from_point(light_pos, edge.a, light_radius);
            let b_proj = project_from_point(light_pos, edge.b, light_radius);

            quads.push(ShadowQuad::new(edge.a, edge.b, b_proj, a_proj));
        }
    }

    quads
}

/// Compute shadow quads for a directional light.
///
/// For each back-facing edge of each occluder, projects the edge endpoints
/// along the light direction to form a shadow quad.
pub fn project_directional_shadows(
    light_direction: [f32; 2],
    cast_distance: f32,
    occluders: &[Occluder],
) -> Vec<ShadowQuad> {
    let mut quads = Vec::new();

    // Normalize direction
    let len = (light_direction[0] * light_direction[0]
        + light_direction[1] * light_direction[1])
        .sqrt();
    if len < 1e-6 {
        return quads;
    }
    let dx = light_direction[0] / len * cast_distance;
    let dy = light_direction[1] / len * cast_distance;

    for occluder in occluders {
        for edge in &occluder.edges {
            if !is_back_facing_directional(edge, light_direction) {
                continue;
            }

            let a_proj = [edge.a[0] + dx, edge.a[1] + dy];
            let b_proj = [edge.b[0] + dx, edge.b[1] + dy];

            quads.push(ShadowQuad::new(edge.a, edge.b, b_proj, a_proj));
        }
    }

    quads
}

/// Project a point radially away from a light source.
///
/// The projected point is placed at least `min_distance` from the light,
/// and always beyond the original point so the shadow extends outward.
fn project_from_point(light_pos: [f32; 2], point: [f32; 2], min_distance: f32) -> [f32; 2] {
    let dx = point[0] - light_pos[0];
    let dy = point[1] - light_pos[1];
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 {
        return point;
    }
    // Ensure projection always extends past the occluder endpoint.
    // If the endpoint is farther than min_distance, extend beyond it.
    let distance = min_distance.max(len + min_distance);
    let nx = dx / len;
    let ny = dy / len;
    [light_pos[0] + nx * distance, light_pos[1] + ny * distance]
}

/// Compute boundary edges from a triangle mesh.
///
/// Boundary edges are edges that belong to exactly one triangle. These form
/// the outer silhouette of the mesh and are used as occluder edges for
/// shadow casting.
///
/// Returns a list of vertex index pairs `(v0, v1)` preserving the original
/// triangle winding order, so that the right-hand perpendicular of each
/// directed edge points outward (away from the mesh interior).
pub fn compute_boundary_edges(triangles: &[u32]) -> Vec<(u32, u32)> {
    use std::collections::HashMap;

    // For each canonical edge key, track count and the original directed edge
    // from the first triangle that contributed it.
    let mut edge_info: HashMap<(u32, u32), (u32, (u32, u32))> = HashMap::new();

    for tri in triangles.chunks(3) {
        if tri.len() < 3 {
            continue;
        }
        let edges = [
            (tri[0], tri[1]),
            (tri[1], tri[2]),
            (tri[2], tri[0]),
        ];
        for &(a, b) in &edges {
            // Canonical form: smaller index first (for counting)
            let key = if a < b { (a, b) } else { (b, a) };
            edge_info
                .entry(key)
                .and_modify(|(count, _)| *count += 1)
                .or_insert((1, (a, b)));
        }
    }

    // Boundary edges appear exactly once — return with original winding
    edge_info
        .into_iter()
        .filter(|&(_, (count, _))| count == 1)
        .map(|(_, (_, directed))| directed)
        .collect()
}
