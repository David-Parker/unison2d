//! Occluder types for shadow casting.
//!
//! Occluders are shapes that block light. They are extracted from game objects
//! each frame and passed to the [`LightingSystem`](crate::LightingSystem) for
//! shadow rendering.

/// A single edge of an occluder shape, defined by two endpoints and an outward normal.
#[derive(Debug, Clone)]
pub struct OccluderEdge {
    /// First endpoint (world space).
    pub a: [f32; 2],
    /// Second endpoint (world space).
    pub b: [f32; 2],
    /// Outward-facing normal (unit vector, points away from the shape interior).
    pub normal: [f32; 2],
}

/// A shadow-casting shape, composed of edges.
///
/// Extracted from game objects (rigid bodies, soft bodies, ground plane)
/// and used by the lighting system to generate shadow geometry.
#[derive(Debug, Clone)]
pub struct Occluder {
    /// The edges forming this occluder's boundary.
    pub edges: Vec<OccluderEdge>,
}

impl Occluder {
    /// Create an occluder from a list of edges.
    pub fn new(edges: Vec<OccluderEdge>) -> Self {
        Self { edges }
    }

    /// Create an AABB occluder from a position and half-extents.
    ///
    /// Produces 4 edges with outward-facing normals.
    pub fn from_aabb(cx: f32, cy: f32, hw: f32, hh: f32) -> Self {
        let bl = [cx - hw, cy - hh];
        let br = [cx + hw, cy - hh];
        let tr = [cx + hw, cy + hh];
        let tl = [cx - hw, cy + hh];

        Self {
            edges: vec![
                // Bottom edge: normal points down
                OccluderEdge { a: bl, b: br, normal: [0.0, -1.0] },
                // Right edge: normal points right
                OccluderEdge { a: br, b: tr, normal: [1.0, 0.0] },
                // Top edge: normal points up
                OccluderEdge { a: tr, b: tl, normal: [0.0, 1.0] },
                // Left edge: normal points left
                OccluderEdge { a: tl, b: bl, normal: [-1.0, 0.0] },
            ],
        }
    }

    /// Create a ground plane occluder at the given Y, spanning from `x_min` to `x_max`.
    ///
    /// The normal points downward so that lights above the ground cast shadows
    /// below it (the edge is back-facing relative to overhead lights).
    pub fn from_ground(y: f32, x_min: f32, x_max: f32) -> Self {
        Self {
            edges: vec![OccluderEdge {
                a: [x_min, y],
                b: [x_max, y],
                normal: [0.0, -1.0],
            }],
        }
    }

    /// Create an occluder from soft body boundary edges.
    ///
    /// `positions` is a flat `[x0, y0, x1, y1, ...]` array. `boundary_edges`
    /// is a list of vertex index pairs `(v0, v1)` that form the outer boundary
    /// of the mesh (edges belonging to exactly one triangle).
    pub fn from_boundary_edges(positions: &[f32], boundary_edges: &[(u32, u32)]) -> Self {
        let edges = boundary_edges
            .iter()
            .map(|&(v0, v1)| {
                let ax = positions[v0 as usize * 2];
                let ay = positions[v0 as usize * 2 + 1];
                let bx = positions[v1 as usize * 2];
                let by = positions[v1 as usize * 2 + 1];

                // Compute outward normal (perpendicular to edge, pointing outward).
                // For a counter-clockwise winding, the outward normal is to the right
                // of the edge direction.
                let dx = bx - ax;
                let dy = by - ay;
                let len = (dx * dx + dy * dy).sqrt();
                let normal = if len > 1e-6 {
                    [dy / len, -dx / len]
                } else {
                    [0.0, 1.0]
                };

                OccluderEdge {
                    a: [ax, ay],
                    b: [bx, by],
                    normal,
                }
            })
            .collect();

        Self { edges }
    }
}

/// PCF (Percentage Closer Filtering) mode for shadow edge softness.
///
/// Higher sample counts produce softer shadow edges at higher GPU cost.
/// Mirrors Godot 4's shadow filter options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShadowFilter {
    /// Hard shadows — single sample, crisp edges.
    #[default]
    None,
    /// 5-tap PCF — cardinal directions + center.
    Pcf5,
    /// 13-tap PCF — 3×3 grid + 4 extended samples.
    Pcf13,
}

impl ShadowFilter {
    /// Get the integer value passed to the shader uniform.
    pub fn as_uniform_value(&self) -> u32 {
        match self {
            ShadowFilter::None => 0,
            ShadowFilter::Pcf5 => 5,
            ShadowFilter::Pcf13 => 13,
        }
    }
}
