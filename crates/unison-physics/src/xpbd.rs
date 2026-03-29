//! XPBD (Extended Position-Based Dynamics) solver for soft body simulation
//!
//! XPBD is unconditionally stable unlike force-based FEM with explicit integration.
//! Key insight: position-based constraints solved with compliance give implicit-like stability.
//!
//! References:
//! - "XPBD: Position-Based Simulation of Compliant Constrained Dynamics" (Macklin et al. 2016)
//! - Ten Minute Physics XPBD tutorial

use unison_profiler::profile_scope;


#[cfg(feature = "simd")]
use crate::compute::simd::SimdBackend;
#[cfg(feature = "simd")]
use crate::compute::ComputeBackend;
#[cfg(feature = "simd")]
use wide::f32x4;

/// Flat-grid spatial hash for O(1) neighbor queries in collision detection.
/// Uses a dense grid instead of HashMap for cache-friendly lookups.
pub struct SpatialHash {
    cell_size: f32,
    inv_cell_size: f32,
    // Grid bounds (in cell coordinates)
    min_cx: i32,
    min_cy: i32,
    cols: usize,
    rows: usize,
    // Flat grid: each cell holds a range (start, end) into `entries`
    cell_ranges: Vec<(u32, u32)>,
    entries: Vec<(usize, usize)>,  // (body_idx, edge_idx)
    // Temporary buffer used during build
    temp_entries: Vec<(i32, i32, usize, usize)>, // (cx, cy, body_idx, edge_idx)
}

impl SpatialHash {
    pub fn new(cell_size: f32) -> Self {
        SpatialHash {
            cell_size,
            inv_cell_size: 1.0 / cell_size,
            min_cx: 0,
            min_cy: 0,
            cols: 0,
            rows: 0,
            cell_ranges: Vec::new(),
            entries: Vec::with_capacity(512),
            temp_entries: Vec::with_capacity(512),
        }
    }

    #[inline]
    fn hash(&self, x: f32, y: f32) -> (i32, i32) {
        let cx = (x * self.inv_cell_size).floor() as i32;
        let cy = (y * self.inv_cell_size).floor() as i32;
        (cx, cy)
    }

    #[inline]
    fn cell_index(&self, cx: i32, cy: i32) -> Option<usize> {
        let lx = cx - self.min_cx;
        let ly = cy - self.min_cy;
        if lx >= 0 && ly >= 0 && (lx as usize) < self.cols && (ly as usize) < self.rows {
            Some(ly as usize * self.cols + lx as usize)
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.temp_entries.clear();
        self.entries.clear();
    }

    /// Stage an entry for insertion (call `build()` after all inserts)
    pub fn insert(&mut self, body_idx: usize, edge_idx: usize, x: f32, y: f32) {
        let (cx, cy) = self.hash(x, y);
        self.temp_entries.push((cx, cy, body_idx, edge_idx));
    }

    /// Build the flat grid from staged entries. Must be called after all `insert`s.
    pub fn build(&mut self) {
        if self.temp_entries.is_empty() {
            self.cols = 0;
            self.rows = 0;
            return;
        }

        // Find grid bounds
        let mut min_cx = i32::MAX;
        let mut min_cy = i32::MAX;
        let mut max_cx = i32::MIN;
        let mut max_cy = i32::MIN;
        for &(cx, cy, _, _) in &self.temp_entries {
            min_cx = min_cx.min(cx);
            min_cy = min_cy.min(cy);
            max_cx = max_cx.max(cx);
            max_cy = max_cy.max(cy);
        }
        // Expand by 1 for neighbor queries at the boundary
        self.min_cx = min_cx;
        self.min_cy = min_cy;
        self.cols = (max_cx - min_cx + 1) as usize;
        self.rows = (max_cy - min_cy + 1) as usize;

        let num_cells = self.cols * self.rows;

        // Count entries per cell
        self.cell_ranges.clear();
        self.cell_ranges.resize(num_cells, (0, 0));

        for &(cx, cy, _, _) in &self.temp_entries {
            let idx = (cy - self.min_cy) as usize * self.cols + (cx - self.min_cx) as usize;
            self.cell_ranges[idx].1 += 1; // use .1 as count temporarily
        }

        // Compute prefix sums to get ranges
        let mut offset = 0u32;
        for range in self.cell_ranges.iter_mut() {
            let count = range.1;
            range.0 = offset;
            range.1 = offset;
            offset += count;
        }

        // Fill entries
        self.entries.resize(offset as usize, (0, 0));
        for &(cx, cy, body_idx, edge_idx) in &self.temp_entries {
            let cell = (cy - self.min_cy) as usize * self.cols + (cx - self.min_cx) as usize;
            let pos = self.cell_ranges[cell].1 as usize;
            self.entries[pos] = (body_idx, edge_idx);
            self.cell_ranges[cell].1 += 1;
        }
    }

    /// Fill `result` buffer with all entries in cell and its 3x3 neighborhood
    #[inline]
    pub fn query_neighbors_into(&self, x: f32, y: f32, result: &mut Vec<(usize, usize)>) {
        let (cx, cy) = self.hash(x, y);
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if let Some(idx) = self.cell_index(cx + dx, cy + dy) {
                    let (start, end) = self.cell_ranges[idx];
                    result.extend_from_slice(&self.entries[start as usize..end as usize]);
                }
            }
        }
    }
}

/// Cached edge data for collision detection (used during candidate building)
#[derive(Clone)]
struct CachedEdge {
    body_idx: usize,
    v0: usize,
    v1: usize,
    w0: f32,      // inv_mass of v0
    w1: f32,      // inv_mass of v1
}

/// Collision candidate: a vertex-edge pair that may be colliding.
/// Packed as u32s to halve memory bandwidth vs usize on 64-bit.
#[derive(Clone, Copy)]
struct Candidate {
    body_a: u32,
    vert: u32,
    body_b: u32,
    edge_v0: u32,
    edge_v1: u32,
    w0: u32,  // f32 bits stored as u32
    w1: u32,  // f32 bits stored as u32
}

/// Collision system for handling multi-body collisions efficiently
pub struct CollisionSystem {
    edge_hash: SpatialHash,
    min_dist: f32,
    aabbs: Vec<(f32, f32, f32, f32)>,
    overlapping_pairs: Vec<(usize, usize)>,
    cached_edges: Vec<CachedEdge>,  // Precomputed edge data per frame
    body_needs_collision: Vec<bool>, // Reusable buffer to avoid per-call allocation
    query_buf: Vec<(usize, usize)>, // Reusable buffer for spatial hash query results
    // Per-body "danger zone" AABB: union of all overlapping partner AABBs.
    // Vertices outside this zone can't possibly collide with another body.
    danger_zones: Vec<(f32, f32, f32, f32)>,
    // Candidate pairs collected on first narrow phase query, reused across substeps
    candidates: Vec<Candidate>,
    candidates_valid: bool,
    num_bodies_prepared: usize,
    // Diagnostic counters (public for profiling)
    pub stats_candidates: u32,
    pub stats_cached_edges: u32,
    pub stats_overlapping_pairs: u32,
    pub stats_collisions_found: u32,
    pub stats_iterations_run: u32,
    // Collision events: when enabled, body pairs with contacts are recorded
    pub(crate) collision_events_enabled: bool,
    /// Body pair indices (soft body array index, not handles) that had contacts.
    /// May contain duplicates — caller deduplicates via HashSet.
    pub(crate) contact_body_pairs: Vec<(u32, u32)>,
}

impl CollisionSystem {
    pub fn new(min_dist: f32) -> Self {
        // Cell size should be large enough to find edges from any nearby vertex
        // Use larger cells to ensure long edges are found
        let cell_size = 0.8;  // Fixed cell size that works for typical edge lengths
        CollisionSystem {
            edge_hash: SpatialHash::new(cell_size),
            min_dist,
            aabbs: Vec::with_capacity(32),
            overlapping_pairs: Vec::with_capacity(64),
            cached_edges: Vec::with_capacity(256),
            body_needs_collision: Vec::with_capacity(32),
            query_buf: Vec::with_capacity(32),
            danger_zones: Vec::with_capacity(32),
            candidates: Vec::with_capacity(1024),
            candidates_valid: false,
            num_bodies_prepared: 0,
            stats_candidates: 0,
            stats_cached_edges: 0,
            stats_overlapping_pairs: 0,
            stats_collisions_found: 0,
            stats_iterations_run: 0,
            collision_events_enabled: false,
            contact_body_pairs: Vec::new(),
        }
    }

    /// Check if two AABBs overlap (with margin for collision distance)
    #[inline]
    fn aabbs_overlap(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32), margin: f32) -> bool {
        a.2 + margin >= b.0 && b.2 + margin >= a.0 &&  // X overlap
        a.3 + margin >= b.1 && b.3 + margin >= a.1     // Y overlap
    }

    /// Prepare collision data: broad phase, edge cache, spatial hash.
    /// Call once per frame before the substep loop.
    /// Candidates are invalidated and will be rebuilt on next resolve call.
    pub fn prepare(&mut self, bodies: &[XPBDSoftBody]) {
        let num_bodies = bodies.len();
        self.num_bodies_prepared = num_bodies;
        self.candidates_valid = false;

        // Step 1: Compute AABBs for all bodies
        self.aabbs.clear();
        for body in bodies.iter() {
            self.aabbs.push(body.get_aabb());
        }

        // Step 2: Find overlapping body pairs (broad phase)
        {
            profile_scope!("broad_phase");
            self.overlapping_pairs.clear();
            for i in 0..num_bodies {
                for j in (i + 1)..num_bodies {
                    if Self::aabbs_overlap(self.aabbs[i], self.aabbs[j], self.min_dist) {
                        self.overlapping_pairs.push((i, j));
                    }
                }
            }
        }

        if self.overlapping_pairs.is_empty() {
            return;
        }

        // Step 3: Build danger zones, edge cache, and spatial hash
        self.full_rebuild(bodies);

        self.stats_cached_edges = self.cached_edges.len() as u32;
        self.stats_overlapping_pairs = self.overlapping_pairs.len() as u32;
    }

    /// Full rebuild of danger zones, edge cache, and spatial hash.
    fn full_rebuild(&mut self, bodies: &[XPBDSoftBody]) {
        let num_bodies = self.num_bodies_prepared;

        profile_scope!("edge_cache");
        self.cached_edges.clear();
        self.edge_hash.clear();

        self.body_needs_collision.clear();
        self.body_needs_collision.resize(num_bodies, false);
        self.danger_zones.clear();
        self.danger_zones.resize(num_bodies, (f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY));
        let margin = self.min_dist;
        for &(i, j) in &self.overlapping_pairs {
            self.body_needs_collision[i] = true;
            self.body_needs_collision[j] = true;
            let aj = self.aabbs[j];
            let di = &mut self.danger_zones[i];
            di.0 = di.0.min(aj.0 - margin);
            di.1 = di.1.min(aj.1 - margin);
            di.2 = di.2.max(aj.2 + margin);
            di.3 = di.3.max(aj.3 + margin);
            let ai = self.aabbs[i];
            let dj = &mut self.danger_zones[j];
            dj.0 = dj.0.min(ai.0 - margin);
            dj.1 = dj.1.min(ai.1 - margin);
            dj.2 = dj.2.max(ai.2 + margin);
            dj.3 = dj.3.max(ai.3 + margin);
        }

        for (body_idx, body) in bodies.iter().enumerate() {
            if !self.body_needs_collision[body_idx] { continue; }
            let dz = self.danger_zones[body_idx];

            for edge in &body.edge_constraints {
                let w0 = body.inv_mass[edge.v0];
                let w1 = body.inv_mass[edge.v1];

                if w0 == 0.0 && w1 == 0.0 { continue; }

                let e0x = body.pos[edge.v0 * 2];
                let e0y = body.pos[edge.v0 * 2 + 1];
                let e1x = body.pos[edge.v1 * 2];
                let e1y = body.pos[edge.v1 * 2 + 1];

                // Skip edges entirely outside the danger zone
                let edge_min_x = e0x.min(e1x);
                let edge_max_x = e0x.max(e1x);
                let edge_min_y = e0y.min(e1y);
                let edge_max_y = e0y.max(e1y);
                if edge_max_x < dz.0 || edge_min_x > dz.2 ||
                   edge_max_y < dz.1 || edge_min_y > dz.3 { continue; }

                let dx = e1x - e0x;
                let dy = e1y - e0y;
                if dx * dx + dy * dy < 1e-10 { continue; }

                let edge_idx = self.cached_edges.len();
                self.cached_edges.push(CachedEdge {
                    body_idx,
                    v0: edge.v0,
                    v1: edge.v1,
                    w0,
                    w1,
                });

                self.edge_hash.insert(body_idx, edge_idx, e0x, e0y);
                self.edge_hash.insert(body_idx, edge_idx, e1x, e1y);
            }
        }

        self.edge_hash.build();
    }

    /// Legacy single-call API: builds and resolves in one call.
    /// Used by tests that don't call prepare().
    pub fn solve_collisions(&mut self, bodies: &mut [XPBDSoftBody]) -> u32 {
        self.prepare(bodies);
        self.resolve_collisions(bodies)
    }

    /// Resolve collisions using pre-built spatial hash from prepare().
    /// On first call after prepare(), queries the hash to build candidate pairs.
    /// Subsequent calls reuse cached candidates with fresh positions.
    pub fn resolve_collisions(&mut self, bodies: &mut [XPBDSoftBody]) -> u32 {
        self.resolve_collisions_with_kinematic(bodies, &[])
    }

    /// Resolve collisions with a configurable iteration count.
    /// Higher values improve contact quality without increasing substeps.
    pub fn resolve_collisions_n(&mut self, bodies: &mut [XPBDSoftBody], iterations: u32) -> u32 {
        self.resolve_collisions_with_kinematic_n(bodies, &[], iterations)
    }

    /// Resolve collisions with kinematic body support.
    /// Kinematic bodies participate in collisions but don't get moved.
    /// `is_kinematic` slice should match bodies length (or be empty for no kinematic).
    pub fn resolve_collisions_with_kinematic(&mut self, bodies: &mut [XPBDSoftBody], is_kinematic: &[bool]) -> u32 {
        self.resolve_collisions_with_kinematic_n(bodies, is_kinematic, 3)
    }

    /// Resolve collisions with kinematic support and configurable iteration count.
    pub fn resolve_collisions_with_kinematic_n(&mut self, bodies: &mut [XPBDSoftBody], is_kinematic: &[bool], iterations: u32) -> u32 {
        if self.overlapping_pairs.is_empty() {
            return 0;
        }

        // Clear contact pairs for this resolution pass
        if self.collision_events_enabled {
            self.contact_body_pairs.clear();
        }

        // Build candidates on first call after prepare()
        if !self.candidates_valid {
            profile_scope!("build_candidates");
            self.build_candidates(bodies);
            self.candidates_valid = true;
        }

        // Resolve candidates with configurable iteration count
        let mut total = 0;
        let mut iters = 0u32;
        {
            profile_scope!("narrow_phase");
            for _ in 0..iterations {
                iters += 1;
                let found = self.resolve_candidate_collisions_kinematic(bodies, is_kinematic);
                total += found;
                if found == 0 { break; }
            }
        }
        self.stats_candidates = self.candidates.len() as u32;
        self.stats_collisions_found = total;
        self.stats_iterations_run = iters;
        total
    }

    /// Query spatial hash to find all vertex-edge candidate pairs.
    /// Pre-filters with a distance check: only keeps pairs within candidate_radius
    /// of actual collision. This eliminates ~80-90% of false positives from the
    /// spatial hash neighborhood, dramatically reducing work in resolve iterations.
    fn build_candidates(&mut self, bodies: &[XPBDSoftBody]) {
        self.candidates.clear();
        let num_bodies = self.num_bodies_prepared;
        // Generous radius: min_dist × 4 gives margin for inter-substep movement
        let candidate_radius_sq = (self.min_dist * 4.0) * (self.min_dist * 4.0);

        for body_a_idx in 0..num_bodies {
            if !self.body_needs_collision[body_a_idx] { continue; }
            let dz = self.danger_zones[body_a_idx];

            for vert_idx in 0..bodies[body_a_idx].num_verts {
                if bodies[body_a_idx].inv_mass[vert_idx] == 0.0 { continue; }

                let vx = bodies[body_a_idx].pos[vert_idx * 2];
                let vy = bodies[body_a_idx].pos[vert_idx * 2 + 1];

                if vx < dz.0 || vx > dz.2 || vy < dz.1 || vy > dz.3 { continue; }

                self.query_buf.clear();
                self.edge_hash.query_neighbors_into(vx, vy, &mut self.query_buf);

                for i in 0..self.query_buf.len() {
                    let (body_b_idx, edge_idx) = self.query_buf[i];
                    if body_b_idx == body_a_idx { continue; }

                    let edge = &self.cached_edges[edge_idx];

                    // Distance pre-filter: compute vertex-edge distance and reject far pairs.
                    // This is the same math as resolve but without position correction.
                    let e0x = bodies[body_b_idx].pos[edge.v0 * 2];
                    let e0y = bodies[body_b_idx].pos[edge.v0 * 2 + 1];
                    let e1x = bodies[body_b_idx].pos[edge.v1 * 2];
                    let e1y = bodies[body_b_idx].pos[edge.v1 * 2 + 1];

                    let edx = e1x - e0x;
                    let edy = e1y - e0y;
                    let len_sq = edx * edx + edy * edy;
                    if len_sq < 1e-10 { continue; }

                    let t = ((vx - e0x) * edx + (vy - e0y) * edy) / len_sq;
                    let t = t.clamp(0.0, 1.0);
                    let dx = vx - (e0x + t * edx);
                    let dy = vy - (e0y + t * edy);
                    let dist_sq = dx * dx + dy * dy;

                    if dist_sq > candidate_radius_sq { continue; }

                    self.candidates.push(Candidate {
                        body_a: body_a_idx as u32,
                        vert: vert_idx as u32,
                        body_b: body_b_idx as u32,
                        edge_v0: edge.v0 as u32,
                        edge_v1: edge.v1 as u32,
                        w0: edge.w0.to_bits(),
                        w1: edge.w1.to_bits(),
                    });
                }
            }
        }
    }

    /// Resolve collisions for all cached candidates using fresh positions.
    /// SIMD version: batches distance computation for 4 candidates at a time using f32x4,
    /// then applies corrections scalar. The "parallel x, parallel y" layout means all
    /// SIMD ops are vertical (lane-parallel) with no horizontal adds.
    #[cfg(feature = "simd")]
    fn resolve_candidate_collisions(&self, bodies: &mut [XPBDSoftBody]) -> u32 {
        let mut total_collisions = 0u32;
        let min_dist = self.min_dist;
        let min_dist_sq = min_dist * min_dist;
        let epsilon_v = f32x4::splat(1e-10);
        let zero_v = f32x4::splat(0.0);
        let one_v = f32x4::splat(1.0);

        let n = self.candidates.len();
        let chunks = n / 4;

        // Process 4 candidates at a time
        for chunk in 0..chunks {
            let base = chunk * 4;

            // Gather positions for 4 candidates into parallel x/y lanes
            let mut vx_arr = [0.0f32; 4];
            let mut vy_arr = [0.0f32; 4];
            let mut e0x_arr = [0.0f32; 4];
            let mut e0y_arr = [0.0f32; 4];
            let mut e1x_arr = [0.0f32; 4];
            let mut e1y_arr = [0.0f32; 4];

            for j in 0..4 {
                let c = &self.candidates[base + j];
                let ba = c.body_a as usize;
                let vi = c.vert as usize;
                let bb = c.body_b as usize;
                let ev0 = c.edge_v0 as usize;
                let ev1 = c.edge_v1 as usize;

                vx_arr[j] = bodies[ba].pos[vi * 2];
                vy_arr[j] = bodies[ba].pos[vi * 2 + 1];
                e0x_arr[j] = bodies[bb].pos[ev0 * 2];
                e0y_arr[j] = bodies[bb].pos[ev0 * 2 + 1];
                e1x_arr[j] = bodies[bb].pos[ev1 * 2];
                e1y_arr[j] = bodies[bb].pos[ev1 * 2 + 1];
            }

            // SIMD distance computation (all vertical ops, no horizontal adds)
            let vx_v = f32x4::new(vx_arr);
            let vy_v = f32x4::new(vy_arr);
            let e0x_v = f32x4::new(e0x_arr);
            let e0y_v = f32x4::new(e0y_arr);
            let e1x_v = f32x4::new(e1x_arr);
            let e1y_v = f32x4::new(e1y_arr);

            let edx_v = e1x_v - e0x_v;
            let edy_v = e1y_v - e0y_v;
            let len_sq_v = edx_v * edx_v + edy_v * edy_v;

            // Compute t parameter (projection onto edge)
            let rel_x = vx_v - e0x_v;
            let rel_y = vy_v - e0y_v;
            let dot_v = rel_x * edx_v + rel_y * edy_v;
            // Safe divide: if len_sq is tiny, t will be clamped to 0 anyway
            let t_v = (dot_v / len_sq_v.max(epsilon_v)).max(zero_v).min(one_v);

            // Closest point on edge
            let cx_v = e0x_v + t_v * edx_v;
            let cy_v = e0y_v + t_v * edy_v;

            // Distance vector and squared distance
            let dx_v = vx_v - cx_v;
            let dy_v = vy_v - cy_v;
            let dist_sq_v = dx_v * dx_v + dy_v * dy_v;

            // Extract results for scalar collision resolution
            let dist_sqs = dist_sq_v.to_array();
            let ts = t_v.to_array();
            let dxs = dx_v.to_array();
            let dys = dy_v.to_array();
            let len_sqs = len_sq_v.to_array();

            for j in 0..4 {
                if len_sqs[j] < 1e-10 { continue; }
                if dist_sqs[j] >= min_dist_sq || dist_sqs[j] <= 1e-10 { continue; }

                total_collisions += 1;

                let c = &self.candidates[base + j];
                let ba = c.body_a as usize;
                let vi = c.vert as usize;
                let bb = c.body_b as usize;
                let v0 = c.edge_v0 as usize;
                let v1 = c.edge_v1 as usize;
                let edge_w0 = f32::from_bits(c.w0);
                let edge_w1 = f32::from_bits(c.w1);
                let w_vert = bodies[ba].inv_mass[vi];
                let t = ts[j];

                let dist = dist_sqs[j].sqrt();
                let overlap = min_dist - dist;
                let nx = dxs[j] / dist;
                let ny = dys[j] / dist;

                let w_edge = (1.0 - t) * edge_w0 + t * edge_w1;
                let w_total = w_vert + w_edge;
                if w_total < 1e-10 { continue; }

                let vert_corr = overlap * (w_vert / w_total);
                let edge_corr = overlap * (w_edge / w_total);

                bodies[ba].pos[vi * 2] += nx * vert_corr;
                bodies[ba].pos[vi * 2 + 1] += ny * vert_corr;

                let e0_factor = (1.0 - t) * edge_w0 / w_edge.max(1e-10);
                let e1_factor = t * edge_w1 / w_edge.max(1e-10);

                bodies[bb].pos[v0 * 2] -= nx * edge_corr * e0_factor;
                bodies[bb].pos[v0 * 2 + 1] -= ny * edge_corr * e0_factor;
                bodies[bb].pos[v1 * 2] -= nx * edge_corr * e1_factor;
                bodies[bb].pos[v1 * 2 + 1] -= ny * edge_corr * e1_factor;
            }
        }

        // Scalar remainder (0-3 candidates)
        for i in (chunks * 4)..n {
            total_collisions += self.resolve_single_candidate(bodies, i, min_dist, min_dist_sq);
        }

        total_collisions
    }

    /// Resolve collisions for all cached candidates using fresh positions (scalar fallback).
    #[cfg(not(feature = "simd"))]
    fn resolve_candidate_collisions(&self, bodies: &mut [XPBDSoftBody]) -> u32 {
        let mut total_collisions = 0u32;
        let min_dist = self.min_dist;
        let min_dist_sq = min_dist * min_dist;

        for i in 0..self.candidates.len() {
            total_collisions += self.resolve_single_candidate(bodies, i, min_dist, min_dist_sq);
        }

        total_collisions
    }

    /// Resolve a single candidate collision. Used by both SIMD remainder and scalar path.
    #[inline]
    fn resolve_single_candidate(&self, bodies: &mut [XPBDSoftBody], i: usize, min_dist: f32, min_dist_sq: f32) -> u32 {
        let c = &self.candidates[i];
        let ba = c.body_a as usize;
        let vi = c.vert as usize;
        let bb = c.body_b as usize;
        let v0 = c.edge_v0 as usize;
        let v1 = c.edge_v1 as usize;
        let edge_w0 = f32::from_bits(c.w0);
        let edge_w1 = f32::from_bits(c.w1);

        let w_vert = bodies[ba].inv_mass[vi];

        let vx = bodies[ba].pos[vi * 2];
        let vy = bodies[ba].pos[vi * 2 + 1];

        let e0x = bodies[bb].pos[v0 * 2];
        let e0y = bodies[bb].pos[v0 * 2 + 1];
        let e1x = bodies[bb].pos[v1 * 2];
        let e1y = bodies[bb].pos[v1 * 2 + 1];

        let edx = e1x - e0x;
        let edy = e1y - e0y;
        let len_sq = edx * edx + edy * edy;

        if len_sq < 1e-10 { return 0; }

        let t = ((vx - e0x) * edx + (vy - e0y) * edy) / len_sq;
        let t = t.clamp(0.0, 1.0);

        let closest_x = e0x + t * edx;
        let closest_y = e0y + t * edy;

        let dx = vx - closest_x;
        let dy = vy - closest_y;
        let dist_sq = dx * dx + dy * dy;

        if dist_sq < min_dist_sq && dist_sq > 1e-10 {
            let dist = dist_sq.sqrt();
            let overlap = min_dist - dist;

            let nx = dx / dist;
            let ny = dy / dist;

            let w_edge = (1.0 - t) * edge_w0 + t * edge_w1;
            let w_total = w_vert + w_edge;

            if w_total < 1e-10 { return 0; }

            let vert_corr = overlap * (w_vert / w_total);
            let edge_corr = overlap * (w_edge / w_total);

            bodies[ba].pos[vi * 2] += nx * vert_corr;
            bodies[ba].pos[vi * 2 + 1] += ny * vert_corr;

            let e0_factor = (1.0 - t) * edge_w0 / w_edge.max(1e-10);
            let e1_factor = t * edge_w1 / w_edge.max(1e-10);

            bodies[bb].pos[v0 * 2] -= nx * edge_corr * e0_factor;
            bodies[bb].pos[v0 * 2 + 1] -= ny * edge_corr * e0_factor;
            bodies[bb].pos[v1 * 2] -= nx * edge_corr * e1_factor;
            bodies[bb].pos[v1 * 2 + 1] -= ny * edge_corr * e1_factor;

            return 1;
        }

        0
    }

    /// Resolve collisions with kinematic body awareness (SIMD version).
    /// Batches distance computation for 4 candidates at a time, then applies
    /// corrections scalar with kinematic awareness.
    #[cfg(feature = "simd")]
    fn resolve_candidate_collisions_kinematic(&mut self, bodies: &mut [XPBDSoftBody], is_kinematic: &[bool]) -> u32 {
        let mut total_collisions = 0u32;
        let min_dist = self.min_dist;
        let min_dist_sq = min_dist * min_dist;
        let epsilon_v = f32x4::splat(1e-10);
        let zero_v = f32x4::splat(0.0);
        let one_v = f32x4::splat(1.0);

        let n = self.candidates.len();
        let chunks = n / 4;

        for chunk in 0..chunks {
            let base = chunk * 4;

            // Gather positions for 4 candidates
            let mut vx_arr = [0.0f32; 4];
            let mut vy_arr = [0.0f32; 4];
            let mut e0x_arr = [0.0f32; 4];
            let mut e0y_arr = [0.0f32; 4];
            let mut e1x_arr = [0.0f32; 4];
            let mut e1y_arr = [0.0f32; 4];

            for j in 0..4 {
                let c = &self.candidates[base + j];
                let ba = c.body_a as usize;
                let vi = c.vert as usize;
                let bb = c.body_b as usize;
                let ev0 = c.edge_v0 as usize;
                let ev1 = c.edge_v1 as usize;

                vx_arr[j] = bodies[ba].pos[vi * 2];
                vy_arr[j] = bodies[ba].pos[vi * 2 + 1];
                e0x_arr[j] = bodies[bb].pos[ev0 * 2];
                e0y_arr[j] = bodies[bb].pos[ev0 * 2 + 1];
                e1x_arr[j] = bodies[bb].pos[ev1 * 2];
                e1y_arr[j] = bodies[bb].pos[ev1 * 2 + 1];
            }

            // SIMD distance computation
            let vx_v = f32x4::new(vx_arr);
            let vy_v = f32x4::new(vy_arr);
            let e0x_v = f32x4::new(e0x_arr);
            let e0y_v = f32x4::new(e0y_arr);
            let e1x_v = f32x4::new(e1x_arr);
            let e1y_v = f32x4::new(e1y_arr);

            let edx_v = e1x_v - e0x_v;
            let edy_v = e1y_v - e0y_v;
            let len_sq_v = edx_v * edx_v + edy_v * edy_v;

            let rel_x = vx_v - e0x_v;
            let rel_y = vy_v - e0y_v;
            let dot_v = rel_x * edx_v + rel_y * edy_v;
            let t_v = (dot_v / len_sq_v.max(epsilon_v)).max(zero_v).min(one_v);

            let cx_v = e0x_v + t_v * edx_v;
            let cy_v = e0y_v + t_v * edy_v;

            let dx_v = vx_v - cx_v;
            let dy_v = vy_v - cy_v;
            let dist_sq_v = dx_v * dx_v + dy_v * dy_v;

            // Extract for scalar correction with kinematic handling
            let dist_sqs = dist_sq_v.to_array();
            let ts = t_v.to_array();
            let dxs = dx_v.to_array();
            let dys = dy_v.to_array();
            let len_sqs = len_sq_v.to_array();

            for j in 0..4 {
                if len_sqs[j] < 1e-10 { continue; }
                if dist_sqs[j] >= min_dist_sq || dist_sqs[j] <= 1e-10 { continue; }

                let c = &self.candidates[base + j];
                let ba = c.body_a as usize;
                let vi = c.vert as usize;
                let bb = c.body_b as usize;
                let v0 = c.edge_v0 as usize;
                let v1 = c.edge_v1 as usize;

                let a_kinematic = is_kinematic.get(ba).copied().unwrap_or(false);
                let b_kinematic = is_kinematic.get(bb).copied().unwrap_or(false);
                if a_kinematic && b_kinematic { continue; }

                total_collisions += 1;

                if self.collision_events_enabled {
                    let pair = if c.body_a < c.body_b { (c.body_a, c.body_b) } else { (c.body_b, c.body_a) };
                    self.contact_body_pairs.push(pair);
                }

                let t = ts[j];
                let dist = dist_sqs[j].sqrt();
                let overlap = min_dist - dist;
                let nx = dxs[j] / dist;
                let ny = dys[j] / dist;

                if b_kinematic {
                    bodies[ba].pos[vi * 2] += nx * overlap;
                    bodies[ba].pos[vi * 2 + 1] += ny * overlap;
                } else if a_kinematic {
                    let edge_w0 = f32::from_bits(c.w0);
                    let edge_w1 = f32::from_bits(c.w1);
                    let w_edge = (1.0 - t) * edge_w0 + t * edge_w1;
                    if w_edge < 1e-10 { continue; }

                    let e0_factor = (1.0 - t) * edge_w0 / w_edge;
                    let e1_factor = t * edge_w1 / w_edge;

                    bodies[bb].pos[v0 * 2] -= nx * overlap * e0_factor;
                    bodies[bb].pos[v0 * 2 + 1] -= ny * overlap * e0_factor;
                    bodies[bb].pos[v1 * 2] -= nx * overlap * e1_factor;
                    bodies[bb].pos[v1 * 2 + 1] -= ny * overlap * e1_factor;
                } else {
                    let edge_w0 = f32::from_bits(c.w0);
                    let edge_w1 = f32::from_bits(c.w1);
                    let w_vert = bodies[ba].inv_mass[vi];
                    let w_edge = (1.0 - t) * edge_w0 + t * edge_w1;
                    let w_total = w_vert + w_edge;
                    if w_total < 1e-10 { continue; }

                    let vert_corr = overlap * (w_vert / w_total);
                    let edge_corr = overlap * (w_edge / w_total);

                    bodies[ba].pos[vi * 2] += nx * vert_corr;
                    bodies[ba].pos[vi * 2 + 1] += ny * vert_corr;

                    let e0_factor = (1.0 - t) * edge_w0 / w_edge.max(1e-10);
                    let e1_factor = t * edge_w1 / w_edge.max(1e-10);

                    bodies[bb].pos[v0 * 2] -= nx * edge_corr * e0_factor;
                    bodies[bb].pos[v0 * 2 + 1] -= ny * edge_corr * e0_factor;
                    bodies[bb].pos[v1 * 2] -= nx * edge_corr * e1_factor;
                    bodies[bb].pos[v1 * 2 + 1] -= ny * edge_corr * e1_factor;
                }
            }
        }

        // Scalar remainder
        for i in (chunks * 4)..n {
            let found = self.resolve_single_candidate_kinematic(bodies, i, min_dist, min_dist_sq, is_kinematic);
            if found > 0 && self.collision_events_enabled {
                let c = &self.candidates[i];
                let pair = if c.body_a < c.body_b { (c.body_a, c.body_b) } else { (c.body_b, c.body_a) };
                self.contact_body_pairs.push(pair);
            }
            total_collisions += found;
        }

        total_collisions
    }

    /// Resolve collisions with kinematic body awareness (scalar fallback).
    /// Kinematic bodies don't get moved during collision resolution.
    #[cfg(not(feature = "simd"))]
    fn resolve_candidate_collisions_kinematic(&mut self, bodies: &mut [XPBDSoftBody], is_kinematic: &[bool]) -> u32 {
        let mut total_collisions = 0u32;
        let min_dist = self.min_dist;
        let min_dist_sq = min_dist * min_dist;

        for i in 0..self.candidates.len() {
            let found = self.resolve_single_candidate_kinematic(bodies, i, min_dist, min_dist_sq, is_kinematic);
            if found > 0 && self.collision_events_enabled {
                let c = &self.candidates[i];
                let pair = if c.body_a < c.body_b { (c.body_a, c.body_b) } else { (c.body_b, c.body_a) };
                self.contact_body_pairs.push(pair);
            }
            total_collisions += found;
        }

        total_collisions
    }

    /// Resolve a single candidate collision with kinematic awareness.
    #[inline]
    fn resolve_single_candidate_kinematic(&self, bodies: &mut [XPBDSoftBody], i: usize, min_dist: f32, min_dist_sq: f32, is_kinematic: &[bool]) -> u32 {
        let c = &self.candidates[i];
        let ba = c.body_a as usize;
        let vi = c.vert as usize;
        let bb = c.body_b as usize;
        let v0 = c.edge_v0 as usize;
        let v1 = c.edge_v1 as usize;

        // Check kinematic status
        let a_kinematic = is_kinematic.get(ba).copied().unwrap_or(false);
        let b_kinematic = is_kinematic.get(bb).copied().unwrap_or(false);

        // If both are kinematic, skip (no collision response needed)
        if a_kinematic && b_kinematic {
            return 0;
        }

        let vx = bodies[ba].pos[vi * 2];
        let vy = bodies[ba].pos[vi * 2 + 1];

        let e0x = bodies[bb].pos[v0 * 2];
        let e0y = bodies[bb].pos[v0 * 2 + 1];
        let e1x = bodies[bb].pos[v1 * 2];
        let e1y = bodies[bb].pos[v1 * 2 + 1];

        let edx = e1x - e0x;
        let edy = e1y - e0y;
        let len_sq = edx * edx + edy * edy;

        if len_sq < 1e-10 { return 0; }

        let t = ((vx - e0x) * edx + (vy - e0y) * edy) / len_sq;
        let t = t.clamp(0.0, 1.0);

        let closest_x = e0x + t * edx;
        let closest_y = e0y + t * edy;

        let dx = vx - closest_x;
        let dy = vy - closest_y;
        let dist_sq = dx * dx + dy * dy;

        if dist_sq < min_dist_sq && dist_sq > 1e-10 {
            let dist = dist_sq.sqrt();
            let overlap = min_dist - dist;

            let nx = dx / dist;
            let ny = dy / dist;

            // Apply full correction to the non-kinematic body
            if b_kinematic {
                // Edge body is kinematic, move only the vertex body
                bodies[ba].pos[vi * 2] += nx * overlap;
                bodies[ba].pos[vi * 2 + 1] += ny * overlap;
            } else if a_kinematic {
                // Vertex body is kinematic, move only the edge body
                let edge_w0 = f32::from_bits(c.w0);
                let edge_w1 = f32::from_bits(c.w1);
                let w_edge = (1.0 - t) * edge_w0 + t * edge_w1;
                if w_edge < 1e-10 { return 0; }

                let e0_factor = (1.0 - t) * edge_w0 / w_edge;
                let e1_factor = t * edge_w1 / w_edge;

                bodies[bb].pos[v0 * 2] -= nx * overlap * e0_factor;
                bodies[bb].pos[v0 * 2 + 1] -= ny * overlap * e0_factor;
                bodies[bb].pos[v1 * 2] -= nx * overlap * e1_factor;
                bodies[bb].pos[v1 * 2 + 1] -= ny * overlap * e1_factor;
            } else {
                // Neither is kinematic, use standard weighted resolution
                let edge_w0 = f32::from_bits(c.w0);
                let edge_w1 = f32::from_bits(c.w1);
                let w_vert = bodies[ba].inv_mass[vi];
                let w_edge = (1.0 - t) * edge_w0 + t * edge_w1;
                let w_total = w_vert + w_edge;

                if w_total < 1e-10 { return 0; }

                let vert_corr = overlap * (w_vert / w_total);
                let edge_corr = overlap * (w_edge / w_total);

                bodies[ba].pos[vi * 2] += nx * vert_corr;
                bodies[ba].pos[vi * 2 + 1] += ny * vert_corr;

                let e0_factor = (1.0 - t) * edge_w0 / w_edge.max(1e-10);
                let e1_factor = t * edge_w1 / w_edge.max(1e-10);

                bodies[bb].pos[v0 * 2] -= nx * edge_corr * e0_factor;
                bodies[bb].pos[v0 * 2 + 1] -= ny * edge_corr * e0_factor;
                bodies[bb].pos[v1 * 2] -= nx * edge_corr * e1_factor;
                bodies[bb].pos[v1 * 2 + 1] -= ny * edge_corr * e1_factor;
            }

            return 1;
        }

        0
    }
}

/// Edge constraint data
#[derive(Clone, Debug)]
pub struct EdgeConstraint {
    pub v0: usize,           // First vertex index
    pub v1: usize,           // Second vertex index
    pub rest_length: f32,    // Rest length
}

/// Triangle area constraint data
#[derive(Clone, Debug)]
pub struct AreaConstraint {
    pub v0: usize,
    pub v1: usize,
    pub v2: usize,
    pub rest_area: f32,
}

/// XPBD soft body with position-based constraints
pub struct XPBDSoftBody {
    // Vertex data
    pub pos: Vec<f32>,           // Current positions [x0, y0, x1, y1, ...]
    pub prev_pos: Vec<f32>,      // Previous positions (for velocity computation)
    pub vel: Vec<f32>,           // Velocities (used for external forces)
    pub inv_mass: Vec<f32>,      // Inverse masses (0 = fixed)

    // Force accumulator [fx0, fy0, fx1, fy1, ...] — cleared each step
    pub force_accum: Vec<f32>,

    // Torque accumulator — cleared each step
    pub torque_accum: f32,

    // Constraints
    pub edge_constraints: Vec<EdgeConstraint>,
    pub area_constraints: Vec<AreaConstraint>,

    // Material compliance (inverse of stiffness)
    // Lower compliance = stiffer
    pub edge_compliance: f32,    // For distance constraints
    pub area_compliance: f32,    // For area preservation

    // Triangle connectivity (for rendering)
    pub triangles: Vec<u32>,

    // Counts
    pub num_verts: usize,
}

impl XPBDSoftBody {
    /// Create from mesh vertices and triangles
    /// compliance: 0 = infinitely stiff, higher = softer
    pub fn new(
        vertices: &[f32],
        triangles: &[u32],
        density: f32,
        edge_compliance: f32,
        area_compliance: f32,
    ) -> Self {
        let num_verts = vertices.len() / 2;
        let num_tris = triangles.len() / 3;

        // Initialize vertex data
        let pos = vertices.to_vec();
        let prev_pos = vertices.to_vec();
        let vel = vec![0.0; vertices.len()];

        // Compute masses from triangle areas
        let mut mass = vec![0.0f32; num_verts];
        let mut area_constraints = Vec::with_capacity(num_tris);

        for t in 0..num_tris {
            let i0 = triangles[t * 3] as usize;
            let i1 = triangles[t * 3 + 1] as usize;
            let i2 = triangles[t * 3 + 2] as usize;

            let x0 = vertices[i0 * 2];
            let y0 = vertices[i0 * 2 + 1];
            let x1 = vertices[i1 * 2];
            let y1 = vertices[i1 * 2 + 1];
            let x2 = vertices[i2 * 2];
            let y2 = vertices[i2 * 2 + 1];

            // Compute signed area (for winding order)
            let area = 0.5 * ((x1 - x0) * (y2 - y0) - (x2 - x0) * (y1 - y0));
            let tri_area = area.abs();

            // Distribute mass to vertices
            let tri_mass = tri_area * density;
            mass[i0] += tri_mass / 3.0;
            mass[i1] += tri_mass / 3.0;
            mass[i2] += tri_mass / 3.0;

            // Area constraint
            area_constraints.push(AreaConstraint {
                v0: i0,
                v1: i1,
                v2: i2,
                rest_area: tri_area,
            });
        }

        // Compute inverse masses
        let inv_mass: Vec<f32> = mass.iter().map(|&m| {
            if m > 1e-10 { 1.0 / m } else { 0.0 }
        }).collect();

        // Build edge constraints from triangles (avoiding duplicates)
        let mut edge_set = std::collections::HashSet::new();
        let mut edge_constraints = Vec::new();

        for t in 0..num_tris {
            let i0 = triangles[t * 3] as usize;
            let i1 = triangles[t * 3 + 1] as usize;
            let i2 = triangles[t * 3 + 2] as usize;

            // Add edges (using sorted indices for uniqueness)
            for (a, b) in [(i0, i1), (i1, i2), (i2, i0)] {
                let key = if a < b { (a, b) } else { (b, a) };
                if edge_set.insert(key) {
                    let x0 = vertices[a * 2];
                    let y0 = vertices[a * 2 + 1];
                    let x1 = vertices[b * 2];
                    let y1 = vertices[b * 2 + 1];
                    let dx = x1 - x0;
                    let dy = y1 - y0;
                    let rest_length = (dx * dx + dy * dy).sqrt();

                    edge_constraints.push(EdgeConstraint {
                        v0: a,
                        v1: b,
                        rest_length,
                    });
                }
            }
        }

        let force_accum = vec![0.0; vertices.len()];

        XPBDSoftBody {
            pos,
            prev_pos,
            vel,
            inv_mass,
            force_accum,
            torque_accum: 0.0,
            edge_constraints,
            area_constraints,
            edge_compliance,
            area_compliance,
            triangles: triangles.to_vec(),
            num_verts,
        }
    }

    /// Create from existing FEM material parameters
    /// Note: XPBD compliance is different from FEM stiffness
    /// We scale to give stable behavior with 8 substeps at 60Hz
    pub fn from_material(
        vertices: &[f32],
        triangles: &[u32],
        young_modulus: f32,
        density: f32,
    ) -> Self {
        // For XPBD, compliance = 1/stiffness
        // Lower compliance = stiffer material
        // Scale appropriately for simulation timestep
        let base_compliance = 1.0 / young_modulus;

        // These multipliers are tuned for 8 substeps at 60Hz
        let edge_compliance = base_compliance * 10.0;
        let area_compliance = base_compliance * 100.0;

        Self::new(vertices, triangles, density, edge_compliance, area_compliance)
    }

    /// Pre-solve: apply external forces and predict positions
    #[cfg(feature = "simd")]
    pub fn pre_solve(&mut self, dt: f32, gravity: f32) {
        // Compute center of mass for torque application
        let (cx, cy) = if self.torque_accum != 0.0 {
            let mut sx = 0.0f32;
            let mut sy = 0.0f32;
            for i in 0..self.num_verts {
                sx += self.pos[i * 2];
                sy += self.pos[i * 2 + 1];
            }
            (sx / self.num_verts as f32, sy / self.num_verts as f32)
        } else {
            (0.0, 0.0)
        };
        let omega = self.torque_accum * dt;

        // Apply accumulated forces and torque before SIMD integration
        for i in 0..self.num_verts {
            if self.inv_mass[i] > 0.0 {
                self.vel[i * 2] += self.force_accum[i * 2] * self.inv_mass[i] * dt;
                self.vel[i * 2 + 1] += self.force_accum[i * 2 + 1] * self.inv_mass[i] * dt;

                if omega != 0.0 {
                    let rx = self.pos[i * 2] - cx;
                    let ry = self.pos[i * 2 + 1] - cy;
                    self.vel[i * 2] += -ry * omega;
                    self.vel[i * 2 + 1] += rx * omega;
                }
            }
        }
        // Note: accumulators are cleared by PhysicsWorld::step() AFTER all substeps.

        SimdBackend::integrate_gravity(
            &mut self.pos,
            &mut self.vel,
            &mut self.prev_pos,
            gravity,
            dt,
            &self.inv_mass,
        );
    }

    /// Pre-solve: apply external forces and predict positions (scalar fallback)
    #[cfg(not(feature = "simd"))]
    pub fn pre_solve(&mut self, dt: f32, gravity: f32) {
        // Compute center of mass for torque application
        let (cx, cy) = if self.torque_accum != 0.0 {
            let mut sx = 0.0f32;
            let mut sy = 0.0f32;
            for i in 0..self.num_verts {
                sx += self.pos[i * 2];
                sy += self.pos[i * 2 + 1];
            }
            (sx / self.num_verts as f32, sy / self.num_verts as f32)
        } else {
            (0.0, 0.0)
        };
        let omega = self.torque_accum * dt;

        // Max velocity per substep: prevents tunneling and energy explosion.
        // A vertex should not move more than ~2x the average edge length per substep.
        // We use a fixed cap that works for typical mesh sizes (edges ~0.1-1.0 units).
        let max_vel = 25.0; // units/sec — at dt=1/240 this is ~0.10 units/substep

        for i in 0..self.num_verts {
            if self.inv_mass[i] == 0.0 {
                continue; // Fixed vertex
            }

            // Store previous position
            self.prev_pos[i * 2] = self.pos[i * 2];
            self.prev_pos[i * 2 + 1] = self.pos[i * 2 + 1];

            // Apply accumulated forces: a = F * inv_mass, v += a * dt
            self.vel[i * 2] += self.force_accum[i * 2] * self.inv_mass[i] * dt;
            self.vel[i * 2 + 1] += self.force_accum[i * 2 + 1] * self.inv_mass[i] * dt;

            // Apply accumulated torque as tangential velocity
            if omega != 0.0 {
                let rx = self.pos[i * 2] - cx;
                let ry = self.pos[i * 2 + 1] - cy;
                self.vel[i * 2] += -ry * omega;
                self.vel[i * 2 + 1] += rx * omega;
            }

            // Apply gravity to velocity
            self.vel[i * 2 + 1] += gravity * dt;

            // Clamp velocity to prevent tunneling and energy explosion
            let vx = self.vel[i * 2];
            let vy = self.vel[i * 2 + 1];
            let speed_sq = vx * vx + vy * vy;
            if speed_sq > max_vel * max_vel {
                let scale = max_vel / speed_sq.sqrt();
                self.vel[i * 2] *= scale;
                self.vel[i * 2 + 1] *= scale;
            }

            // Predict position
            self.pos[i * 2] += self.vel[i * 2] * dt;
            self.pos[i * 2 + 1] += self.vel[i * 2 + 1] * dt;
        }

        // Note: accumulators are cleared by PhysicsWorld::step() AFTER all substeps,
        // so the same force/torque applies across every substep within a frame.
    }

    /// Clear force and torque accumulators. Called by PhysicsWorld after all substeps complete.
    pub fn clear_accumulators(&mut self) {
        self.force_accum.iter_mut().for_each(|f| *f = 0.0);
        self.torque_accum = 0.0;
    }

    /// Solve distance (edge length) constraint using XPBD
    /// Returns constraint violation before solve.
    ///
    /// Uses adaptive compliance: when an edge is severely compressed or stretched
    /// (beyond 50% or 200% of rest length), compliance drops to zero for aggressive
    /// correction. This prevents jagged mesh edges from forming at contact zones.
    fn solve_edge_constraint(&mut self, edge: &EdgeConstraint, alpha: f32) -> f32 {
        let i0 = edge.v0;
        let i1 = edge.v1;

        let w0 = self.inv_mass[i0];
        let w1 = self.inv_mass[i1];
        let w_sum = w0 + w1;

        if w_sum < 1e-10 {
            return 0.0; // Both vertices fixed
        }

        // Current edge vector
        let dx = self.pos[i1 * 2] - self.pos[i0 * 2];
        let dy = self.pos[i1 * 2 + 1] - self.pos[i0 * 2 + 1];
        let len = (dx * dx + dy * dy).sqrt();

        if len < 1e-10 {
            return 0.0; // Degenerate edge
        }

        // Constraint: C = len - rest_length
        let c = len - edge.rest_length;

        // Adaptive compliance: severely deformed edges get reduced compliance
        // to aggressively restore them. This prevents the "jagged contact zone" bug
        // and ensures soft materials recover from extreme deformation.
        let ratio = len / edge.rest_length;
        let effective_alpha = if ratio < 0.4 || ratio > 2.5 {
            alpha.min(0.1)
        } else {
            alpha
        };

        // XPBD: λ = -C / (w_sum + α/dt²)
        let lambda = -c / (w_sum + effective_alpha);

        // Position corrections
        let nx = dx / len;
        let ny = dy / len;

        let corr0 = -lambda * w0;
        let corr1 = lambda * w1;

        self.pos[i0 * 2] += corr0 * nx;
        self.pos[i0 * 2 + 1] += corr0 * ny;
        self.pos[i1 * 2] += corr1 * nx;
        self.pos[i1 * 2 + 1] += corr1 * ny;

        c.abs()
    }

    /// Solve area constraint using XPBD
    /// Preserves triangle area (2D volume) and prevents triangle inversion.
    ///
    /// Key insight: when a triangle's signed area goes negative (inverted),
    /// we use zero compliance (infinite stiffness) to aggressively restore it.
    /// This prevents the mesh from "folding through itself" which causes
    /// permanent deformation artifacts.
    fn solve_area_constraint(&mut self, area: &AreaConstraint, alpha: f32) -> f32 {
        let i0 = area.v0;
        let i1 = area.v1;
        let i2 = area.v2;

        let w0 = self.inv_mass[i0];
        let w1 = self.inv_mass[i1];
        let w2 = self.inv_mass[i2];

        // Get positions
        let x0 = self.pos[i0 * 2];
        let y0 = self.pos[i0 * 2 + 1];
        let x1 = self.pos[i1 * 2];
        let y1 = self.pos[i1 * 2 + 1];
        let x2 = self.pos[i2 * 2];
        let y2 = self.pos[i2 * 2 + 1];

        // Current signed area (2 * area = cross product)
        let current_area_2x = (x1 - x0) * (y2 - y0) - (x2 - x0) * (y1 - y0);
        let current_area = current_area_2x * 0.5;

        // Constraint: C = current_area - rest_area
        let c = current_area - area.rest_area;

        // Adaptive compliance for triangle health:
        // 1. Inverted triangles (signed area flipped): zero compliance for maximum
        //    correction. This is a hard constraint — fold-through must be prevented.
        // 2. Near-collapse (area < 25% of rest): zero compliance with a boosted
        //    target to push area back to 25% of rest. Prevents triangles from
        //    settling into a degenerate equilibrium.
        // 3. Normal: use material compliance as-is.
        let area_ratio = current_area / area.rest_area.max(1e-10);
        let is_inverted = (current_area < 0.0 && area.rest_area > 0.0)
            || (current_area > 0.0 && area.rest_area < 0.0);

        let (effective_alpha, c) = if is_inverted {
            if alpha < 1.0 {
                (0.0, c)
            } else {
                (alpha * 0.01, c)
            }
        } else {
            (alpha, c)
        };

        // Gradients of area w.r.t. vertex positions
        let grad0_x = 0.5 * (y1 - y2);
        let grad0_y = 0.5 * (x2 - x1);
        let grad1_x = 0.5 * (y2 - y0);
        let grad1_y = 0.5 * (x0 - x2);
        let grad2_x = 0.5 * (y0 - y1);
        let grad2_y = 0.5 * (x1 - x0);

        // Sum of weighted squared gradient magnitudes
        let grad0_sq = grad0_x * grad0_x + grad0_y * grad0_y;
        let grad1_sq = grad1_x * grad1_x + grad1_y * grad1_y;
        let grad2_sq = grad2_x * grad2_x + grad2_y * grad2_y;

        let w_grad_sum = w0 * grad0_sq + w1 * grad1_sq + w2 * grad2_sq;

        if w_grad_sum < 1e-10 {
            return c.abs();
        }

        // XPBD lambda
        let lambda = -c / (w_grad_sum + effective_alpha);

        // Apply corrections
        self.pos[i0 * 2] += lambda * w0 * grad0_x;
        self.pos[i0 * 2 + 1] += lambda * w0 * grad0_y;
        self.pos[i1 * 2] += lambda * w1 * grad1_x;
        self.pos[i1 * 2 + 1] += lambda * w1 * grad1_y;
        self.pos[i2 * 2] += lambda * w2 * grad2_x;
        self.pos[i2 * 2 + 1] += lambda * w2 * grad2_y;

        c.abs()
    }

    /// Solve ground collision with Coulomb friction for realistic rolling
    /// The contact point sticks to the ground (no slip) up to the friction limit
    pub fn solve_ground_collision(&mut self, ground_y: f32, _dt: f32) {
        self.solve_ground_collision_with_friction(ground_y, 0.8, 0.3);
    }

    /// Solve ground collision with configurable friction and restitution
    /// - friction: Coulomb friction coefficient (0 = ice, 1 = sticky rubber)
    /// - restitution: bounciness (0 = no bounce, 1 = perfect bounce)
    pub fn solve_ground_collision_with_friction(&mut self, ground_y: f32, friction: f32, restitution: f32) {
        // Threshold: vertices within this distance of the ground are considered "in contact"
        let contact_threshold = 0.05;

        for i in 0..self.num_verts {
            if self.inv_mass[i] == 0.0 {
                continue;
            }

            let y = self.pos[i * 2 + 1];

            if y < ground_y {
                let prev_x = self.prev_pos[i * 2];
                let prev_y = self.prev_pos[i * 2 + 1];
                let curr_x = self.pos[i * 2];

                // Penetration depth (normal direction)
                let penetration = ground_y - y;

                // Tangent displacement this substep
                let dx = curr_x - prev_x;
                let dy = y - prev_y;

                // Project out of ground (normal constraint)
                self.pos[i * 2 + 1] = ground_y;

                // Apply restitution if moving into ground
                if dy < 0.0 {
                    self.pos[i * 2 + 1] = ground_y + penetration * restitution;
                }

                // Coulomb friction model:
                // The friction force opposes sliding and is proportional to normal force.
                // In PBD, we reduce tangent displacement by friction coefficient.
                //
                // For rolling to work, high friction means the contact point "sticks" to ground,
                // which creates the torque needed for rolling motion.
                //
                // friction = 1.0: contact point doesn't move (perfect stick -> rolling)
                // friction = 0.0: contact point slides freely (no rolling)

                // Clamp the tangent displacement - this removes energy, never adds it
                let friction_factor = 1.0 - friction;
                self.pos[i * 2] = prev_x + dx * friction_factor;
            } else if y < ground_y + contact_threshold {
                // Near-ground friction: damp horizontal movement of vertices close to ground.
                // This simulates rolling friction — the body decelerates when in contact.
                let prev_x = self.prev_pos[i * 2];
                let curr_x = self.pos[i * 2];
                let dx = curr_x - prev_x;
                let friction_factor = 1.0 - friction * 0.3;
                self.pos[i * 2] = prev_x + dx * friction_factor;
            }
        }
    }

    /// Solve terrain collision with variable height
    /// - height_at: function that returns terrain height at x position
    /// - normal_at: function that returns terrain normal (nx, ny) at x position
    /// - friction: Coulomb friction coefficient
    /// - restitution: bounciness
    pub fn solve_terrain_collision<F, G>(
        &mut self,
        height_at: F,
        normal_at: G,
        friction: f32,
        restitution: f32,
    )
    where
        F: Fn(f32) -> f32,
        G: Fn(f32) -> (f32, f32),
    {
        for i in 0..self.num_verts {
            if self.inv_mass[i] == 0.0 {
                continue;
            }

            let x = self.pos[i * 2];
            let y = self.pos[i * 2 + 1];
            let terrain_y = height_at(x);

            if y < terrain_y {
                let prev_x = self.prev_pos[i * 2];
                let prev_y = self.prev_pos[i * 2 + 1];

                // Get terrain normal at this point
                let (nx, ny) = normal_at(x);

                // Penetration depth along normal
                let penetration = terrain_y - y;

                // Velocity this substep
                let dx = x - prev_x;
                let dy = y - prev_y;

                // Project velocity onto normal and tangent
                let vel_normal = dx * nx + dy * ny;
                let vel_tangent_x = dx - vel_normal * nx;
                let vel_tangent_y = dy - vel_normal * ny;

                // Project out of terrain along normal
                self.pos[i * 2] = x + nx * penetration;
                self.pos[i * 2 + 1] = y + ny * penetration;

                // Apply restitution if moving into terrain
                if vel_normal < 0.0 {
                    self.pos[i * 2] += nx * penetration * restitution;
                    self.pos[i * 2 + 1] += ny * penetration * restitution;
                }

                // Apply friction to tangent motion
                let friction_factor = 1.0 - friction;
                self.pos[i * 2] = prev_x + vel_tangent_x * friction_factor + nx * penetration;
                self.pos[i * 2 + 1] = prev_y + vel_tangent_y * friction_factor + ny * penetration;
            } else if y < terrain_y + 0.05 {
                // Near-terrain friction: damp tangential movement of vertices close to surface
                let prev_x = self.prev_pos[i * 2];
                let prev_y = self.prev_pos[i * 2 + 1];
                let (nx, ny) = normal_at(x);
                let dx = x - prev_x;
                let dy = y - prev_y;
                let vel_tangent_x = dx - (dx * nx + dy * ny) * nx;
                let vel_tangent_y = dy - (dx * nx + dy * ny) * ny;
                let friction_factor = 1.0 - friction * 0.3;
                self.pos[i * 2] = prev_x + (dx - vel_tangent_x) + vel_tangent_x * friction_factor;
                self.pos[i * 2 + 1] = prev_y + (dy - vel_tangent_y) + vel_tangent_y * friction_factor;
            }
        }
    }

    /// Solve all constraints for one iteration
    /// Returns max constraint violation
    pub fn solve_constraints(&mut self, dt: f32) -> f32 {
        let dt_sq = dt * dt;
        let mut max_violation: f32 = 0.0;

        // Edge compliance scaled by dt²
        let edge_alpha = self.edge_compliance / dt_sq;

        // Solve edge (distance) constraints
        for i in 0..self.edge_constraints.len() {
            let edge = self.edge_constraints[i].clone();
            let violation = self.solve_edge_constraint(&edge, edge_alpha);
            max_violation = max_violation.max(violation);
        }

        // Area compliance scaled by dt²
        let area_alpha = self.area_compliance / dt_sq;

        // Solve area constraints
        for i in 0..self.area_constraints.len() {
            let area = self.area_constraints[i].clone();
            let violation = self.solve_area_constraint(&area, area_alpha);
            max_violation = max_violation.max(violation);
        }

        max_violation
    }

    /// Post-solve: compute velocities from position change
    #[cfg(feature = "simd")]
    pub fn post_solve(&mut self, dt: f32) {
        SimdBackend::derive_velocities(&self.pos, &self.prev_pos, &mut self.vel, dt);
    }

    /// Post-solve: compute velocities from position change (scalar fallback)
    #[cfg(not(feature = "simd"))]
    pub fn post_solve(&mut self, dt: f32) {
        let inv_dt = 1.0 / dt;

        for i in 0..self.num_verts {
            self.vel[i * 2] = (self.pos[i * 2] - self.prev_pos[i * 2]) * inv_dt;
            self.vel[i * 2 + 1] = (self.pos[i * 2 + 1] - self.prev_pos[i * 2 + 1]) * inv_dt;
        }
    }

    /// Apply velocity damping
    pub fn apply_damping(&mut self, damping: f32) {
        let factor = 1.0 - damping;
        for i in 0..self.vel.len() {
            self.vel[i] *= factor;
        }
    }

    /// Damp only internal deformation velocity, preserving bulk motion and rotation.
    /// This kills internal oscillation/bouncing without affecting fall speed,
    /// movement, or rolling.
    pub fn apply_internal_damping(&mut self, damping: f32) {
        // Compute mass-weighted center of mass and velocity
        let mut cx = 0.0f32;
        let mut cy = 0.0f32;
        let mut avg_vx = 0.0f32;
        let mut avg_vy = 0.0f32;
        let mut total_mass = 0.0f32;
        for i in 0..self.num_verts {
            if self.inv_mass[i] > 0.0 {
                let m = 1.0 / self.inv_mass[i];
                cx += self.pos[i * 2] * m;
                cy += self.pos[i * 2 + 1] * m;
                avg_vx += self.vel[i * 2] * m;
                avg_vy += self.vel[i * 2 + 1] * m;
                total_mass += m;
            }
        }
        if total_mass < 1e-10 { return; }
        cx /= total_mass;
        cy /= total_mass;
        avg_vx /= total_mass;
        avg_vy /= total_mass;

        // Compute angular velocity (omega) around center of mass
        let mut omega_num = 0.0f32;
        let mut omega_den = 0.0f32;
        for i in 0..self.num_verts {
            if self.inv_mass[i] > 0.0 {
                let m = 1.0 / self.inv_mass[i];
                let rx = self.pos[i * 2] - cx;
                let ry = self.pos[i * 2 + 1] - cy;
                let rel_vx = self.vel[i * 2] - avg_vx;
                let rel_vy = self.vel[i * 2 + 1] - avg_vy;
                // omega = sum(m * r x v) / sum(m * r²)
                omega_num += m * (rx * rel_vy - ry * rel_vx);
                omega_den += m * (rx * rx + ry * ry);
            }
        }
        let omega = if omega_den > 1e-10 { omega_num / omega_den } else { 0.0 };

        // Damp only the deformation component: subtract rigid body motion
        // (linear + angular), damp the remainder, add rigid motion back.
        let factor = 1.0 - damping;
        for i in 0..self.num_verts {
            if self.inv_mass[i] > 0.0 {
                let rx = self.pos[i * 2] - cx;
                let ry = self.pos[i * 2 + 1] - cy;
                // Rigid body velocity at this vertex = linear + omega × r
                let rigid_vx = avg_vx + (-ry * omega);
                let rigid_vy = avg_vy + (rx * omega);
                // Deformation = actual - rigid
                let def_vx = self.vel[i * 2] - rigid_vx;
                let def_vy = self.vel[i * 2 + 1] - rigid_vy;
                // Damp deformation, preserve rigid
                self.vel[i * 2] = rigid_vx + def_vx * factor;
                self.vel[i * 2 + 1] = rigid_vy + def_vy * factor;
            }
        }
    }

    /// Pre-solve and constraint solving (call collide_with_body after this, then finalize_substep)
    pub fn substep_pre(&mut self, dt: f32, gravity: f32, ground_y: Option<f32>) {
        self.substep_pre_with_friction(dt, gravity, ground_y, 0.8, 0.3);
    }

    /// Pre-solve with configurable ground friction and restitution
    pub fn substep_pre_with_friction(
        &mut self,
        dt: f32,
        gravity: f32,
        ground_y: Option<f32>,
        friction: f32,
        restitution: f32,
    ) {
        self.substep_pre_with_friction_iters(dt, gravity, ground_y, friction, restitution, 3, 2);
    }

    /// Pre-solve with configurable iteration counts
    pub fn substep_pre_with_friction_iters(
        &mut self,
        dt: f32,
        gravity: f32,
        ground_y: Option<f32>,
        friction: f32,
        restitution: f32,
        pre_iters: u32,
        post_iters: u32,
    ) {
        {
            profile_scope!("integrate");
            self.pre_solve(dt, gravity);
        }

        // Solve constraints before collision
        {
            profile_scope!("constraints_pre");
            for _ in 0..pre_iters {
                self.solve_constraints(dt);
            }
        }

        // Ground collision with friction
        if let Some(gy) = ground_y {
            {
                profile_scope!("ground_collision");
                self.solve_ground_collision_with_friction(gy, friction, restitution);
            }
            // Re-solve constraints after ground collision to restore shape
            {
                profile_scope!("constraints_post");
                for _ in 0..post_iters {
                    self.solve_constraints(dt);
                }
            }
        }
    }

    /// Pre-solve with terrain collision (variable height ground)
    pub fn substep_pre_with_terrain<F, G>(
        &mut self,
        dt: f32,
        gravity: f32,
        height_at: F,
        normal_at: G,
        friction: f32,
        restitution: f32,
    )
    where
        F: Fn(f32) -> f32,
        G: Fn(f32) -> (f32, f32),
    {
        self.substep_pre_with_terrain_iters(dt, gravity, height_at, normal_at, friction, restitution, 3, 2);
    }

    /// Pre-solve with terrain collision and configurable iteration counts
    pub fn substep_pre_with_terrain_iters<F, G>(
        &mut self,
        dt: f32,
        gravity: f32,
        height_at: F,
        normal_at: G,
        friction: f32,
        restitution: f32,
        pre_iters: u32,
        post_iters: u32,
    )
    where
        F: Fn(f32) -> f32,
        G: Fn(f32) -> (f32, f32),
    {
        {
            profile_scope!("integrate");
            self.pre_solve(dt, gravity);
        }

        // Solve constraints before collision
        {
            profile_scope!("constraints_pre");
            for _ in 0..pre_iters {
                self.solve_constraints(dt);
            }
        }

        // Terrain collision with friction
        {
            profile_scope!("terrain_collision");
            self.solve_terrain_collision(&height_at, &normal_at, friction, restitution);
        }

        // Re-solve constraints after terrain collision to restore shape
        {
            profile_scope!("constraints_post");
            for _ in 0..post_iters {
                self.solve_constraints(dt);
            }
        }
    }

    /// Finalize substep: compute velocities from position change
    pub fn substep_post(&mut self, dt: f32) {
        self.post_solve(dt);
    }

    /// Complete substep: pre-solve, solve constraints, post-solve (no inter-body collision)
    pub fn substep(&mut self, dt: f32, gravity: f32, ground_y: Option<f32>) {
        self.substep_pre(dt, gravity, ground_y);
        self.substep_post(dt);
    }

    /// Get kinetic energy
    pub fn get_kinetic_energy(&self) -> f32 {
        let mut ke = 0.0;
        for i in 0..self.num_verts {
            if self.inv_mass[i] > 0.0 {
                let m = 1.0 / self.inv_mass[i];
                let vx = self.vel[i * 2];
                let vy = self.vel[i * 2 + 1];
                ke += 0.5 * m * (vx * vx + vy * vy);
            }
        }
        ke
    }

    /// Get lowest Y position
    pub fn get_lowest_y(&self) -> f32 {
        let mut lowest = f32::INFINITY;
        for i in 0..self.num_verts {
            lowest = lowest.min(self.pos[i * 2 + 1]);
        }
        lowest
    }

    /// Get AABB
    pub fn get_aabb(&self) -> (f32, f32, f32, f32) {
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for i in 0..self.num_verts {
            let x = self.pos[i * 2];
            let y = self.pos[i * 2 + 1];
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }

        (min_x, min_y, max_x, max_y)
    }

    /// Get center of mass (average position)
    pub fn get_center(&self) -> (f32, f32) {
        let mut cx = 0.0;
        let mut cy = 0.0;
        for i in 0..self.num_verts {
            cx += self.pos[i * 2];
            cy += self.pos[i * 2 + 1];
        }
        let n = self.num_verts as f32;
        (cx / n, cy / n)
    }

    /// Collide with another XPBD body - position-based separation
    /// Call this BEFORE post_solve so velocities are derived correctly
    pub fn collide_with_body(&mut self, other: &mut XPBDSoftBody, min_dist: f32) -> u32 {
        // Broad phase AABB check
        let self_aabb = self.get_aabb();
        let other_aabb = other.get_aabb();

        if self_aabb.2 + min_dist < other_aabb.0 || other_aabb.2 + min_dist < self_aabb.0 ||
           self_aabb.3 + min_dist < other_aabb.1 || other_aabb.3 + min_dist < self_aabb.1 {
            return 0;
        }

        let mut collisions = 0u32;

        for i in 0..self.num_verts {
            let w1 = self.inv_mass[i];
            if w1 == 0.0 { continue; }

            for j in 0..other.num_verts {
                let w2 = other.inv_mass[j];
                if w2 == 0.0 { continue; }

                let x1 = self.pos[i * 2];
                let y1 = self.pos[i * 2 + 1];
                let x2 = other.pos[j * 2];
                let y2 = other.pos[j * 2 + 1];

                let dx = x2 - x1;
                let dy = y2 - y1;
                let dist_sq = dx * dx + dy * dy;

                if dist_sq < min_dist * min_dist && dist_sq > 1e-10 {
                    collisions += 1;

                    let dist = dist_sq.sqrt();
                    let overlap = min_dist - dist;

                    // Normal from self to other
                    let nx = dx / dist;
                    let ny = dy / dist;

                    // Push vertices apart proportional to inverse mass
                    let w_sum = w1 + w2;
                    let corr1 = overlap * (w1 / w_sum);
                    let corr2 = overlap * (w2 / w_sum);

                    // Apply position corrections only
                    self.pos[i * 2] -= nx * corr1;
                    self.pos[i * 2 + 1] -= ny * corr1;
                    other.pos[j * 2] += nx * corr2;
                    other.pos[j * 2 + 1] += ny * corr2;
                }
            }
        }

        collisions
    }

    /// Sleep if kinetic energy is below threshold
    pub fn sleep_if_resting(&mut self, ke_threshold: f32) -> bool {
        let ke = self.get_kinetic_energy();
        if ke < ke_threshold {
            self.vel.fill(0.0);
            true
        } else {
            false
        }
    }

    /// Get max velocity
    pub fn get_max_velocity(&self) -> f32 {
        let mut max_vel_sq: f32 = 0.0;
        for i in 0..self.num_verts {
            let vx = self.vel[i * 2];
            let vy = self.vel[i * 2 + 1];
            max_vel_sq = max_vel_sq.max(vx * vx + vy * vy);
        }
        max_vel_sq.sqrt()
    }

    /// Get aspect ratio (width / height) - for detecting pancaking
    pub fn get_aspect_ratio(&self) -> f32 {
        let (min_x, min_y, max_x, max_y) = self.get_aabb();
        let width = max_x - min_x;
        let height = max_y - min_y;
        if height < 1e-6 { return f32::INFINITY; }
        width / height
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::create_square_mesh;

    #[test]
    fn test_xpbd_creation() {
        let mesh = create_square_mesh(1.0, 2);
        let body = XPBDSoftBody::new(&mesh.vertices, &mesh.triangles, 1000.0, 1e-6, 1e-5);

        assert_eq!(body.num_verts, 9);
        assert!(!body.edge_constraints.is_empty());
        assert!(!body.area_constraints.is_empty());
    }

    #[test]
    fn test_xpbd_freefall() {
        let mesh = create_square_mesh(1.0, 2);
        let mut body = XPBDSoftBody::new(&mesh.vertices, &mesh.triangles, 1000.0, 1e-6, 1e-5);

        let initial_lowest = body.get_lowest_y();

        // Run 8 substeps for one frame
        let dt = 1.0 / 60.0 / 8.0;
        for _ in 0..8 {
            body.substep(dt, -9.8, None);
        }

        let final_lowest = body.get_lowest_y();

        // Should have fallen
        assert!(final_lowest < initial_lowest, "Body should fall under gravity");
    }

    // Simulation / e2e tests live in the `unison-tests` crate.
}
