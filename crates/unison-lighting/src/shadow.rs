//! Shadow mapping system for 2D soft shadows.

use crate::light::Light;
use unison_math::Vec2;
use unison_profiler::profile_scope;

/// Quality settings for shadow rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShadowQuality {
    /// No shadows rendered.
    Off,
    /// 128 resolution, 2 PCF samples.
    Low,
    /// 256 resolution, 4 PCF samples.
    #[default]
    Medium,
    /// 512 resolution, 8 PCF samples.
    High,
}

impl ShadowQuality {
    /// Get the shadow map resolution for this quality level.
    pub fn resolution(&self) -> u32 {
        match self {
            ShadowQuality::Off => 0,
            ShadowQuality::Low => 128,
            ShadowQuality::Medium => 256,
            ShadowQuality::High => 512,
        }
    }

    /// Get the number of PCF samples for this quality level.
    pub fn pcf_samples(&self) -> u32 {
        match self {
            ShadowQuality::Off => 0,
            ShadowQuality::Low => 2,
            ShadowQuality::Medium => 4,
            ShadowQuality::High => 8,
        }
    }
}

/// Identifier for a shadow map resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShadowMapId(pub u32);

/// A 1D shadow map storing distance to nearest occluder per angle.
///
/// The shadow map stores distances from the light source to the nearest
/// occluder for each angle around the light. This is used for point and
/// spot lights. Directional lights use a different approach.
#[derive(Debug, Clone)]
pub struct ShadowMap {
    /// Unique identifier for this shadow map.
    pub id: ShadowMapId,
    /// Resolution (number of angles sampled).
    pub resolution: u32,
    /// Distance values for each angle (0 to 2*PI).
    pub distances: Vec<f32>,
    /// Whether the shadow map needs to be regenerated.
    pub dirty: bool,
    /// The light handle this shadow map belongs to.
    pub light_handle: Option<LightHandle>,
}

impl ShadowMap {
    /// Create a new shadow map with the given resolution.
    pub fn new(id: ShadowMapId, resolution: u32) -> Self {
        Self {
            id,
            resolution,
            distances: vec![f32::INFINITY; resolution as usize],
            dirty: true,
            light_handle: None,
        }
    }

    /// Clear the shadow map to maximum distance.
    pub fn clear(&mut self) {
        for d in &mut self.distances {
            *d = f32::INFINITY;
        }
    }

    /// Mark the shadow map as needing regeneration.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Get the distance at a given angle (in radians).
    pub fn sample(&self, angle: f32) -> f32 {
        if self.resolution == 0 {
            return f32::INFINITY;
        }
        let normalized = (angle / (2.0 * std::f32::consts::PI)).rem_euclid(1.0);
        let index = (normalized * self.resolution as f32) as usize;
        self.distances[index.min(self.resolution as usize - 1)]
    }

    /// Sample with PCF filtering for soft shadows.
    pub fn sample_pcf(&self, angle: f32, samples: u32, spread: f32) -> f32 {
        if samples <= 1 {
            return self.sample(angle);
        }

        let mut total = 0.0;
        let step = spread / (samples - 1) as f32;
        let start = angle - spread * 0.5;

        for i in 0..samples {
            total += self.sample(start + step * i as f32);
        }

        total / samples as f32
    }
}

/// Handle to a light in the lighting manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LightHandle(pub u32);

/// Trait for objects that can cast shadows.
pub trait ShadowCaster {
    /// Get the line segments that block light.
    fn get_occluder_segments(&self) -> Vec<(Vec2, Vec2)>;
}

/// Cache for managing shadow map allocation and dirty tracking.
#[derive(Debug, Default)]
pub struct ShadowMapCache {
    /// All shadow maps.
    shadow_maps: Vec<ShadowMap>,
    /// Next ID to assign.
    next_id: u32,
    /// Free shadow map slots for reuse.
    free_slots: Vec<usize>,
}

impl ShadowMapCache {
    /// Create a new shadow map cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocate a new shadow map with the given resolution.
    pub fn allocate(&mut self, resolution: u32) -> ShadowMapId {
        let id = ShadowMapId(self.next_id);
        self.next_id += 1;

        if let Some(slot) = self.free_slots.pop() {
            self.shadow_maps[slot] = ShadowMap::new(id, resolution);
        } else {
            self.shadow_maps.push(ShadowMap::new(id, resolution));
        }

        id
    }

    /// Free a shadow map for reuse.
    pub fn free(&mut self, id: ShadowMapId) {
        if let Some(pos) = self.shadow_maps.iter().position(|sm| sm.id == id) {
            self.free_slots.push(pos);
        }
    }

    /// Get a shadow map by ID.
    pub fn get(&self, id: ShadowMapId) -> Option<&ShadowMap> {
        self.shadow_maps.iter().find(|sm| sm.id == id)
    }

    /// Get a mutable shadow map by ID.
    pub fn get_mut(&mut self, id: ShadowMapId) -> Option<&mut ShadowMap> {
        self.shadow_maps.iter_mut().find(|sm| sm.id == id)
    }

    /// Mark all shadow maps as dirty.
    pub fn mark_all_dirty(&mut self) {
        for sm in &mut self.shadow_maps {
            sm.dirty = true;
        }
    }

    /// Get all shadow maps that need regeneration.
    pub fn get_dirty(&self) -> Vec<ShadowMapId> {
        self.shadow_maps
            .iter()
            .filter(|sm| sm.dirty)
            .map(|sm| sm.id)
            .collect()
    }
}

/// Compute shadow map distances for a point/spot light.
pub fn compute_shadow_map(
    shadow_map: &mut ShadowMap,
    light: &Light,
    occluders: &[(Vec2, Vec2)],
) {
    profile_scope!("lighting.compute_shadow_map");

    shadow_map.clear();

    let resolution = shadow_map.resolution;
    if resolution == 0 {
        return;
    }

    let two_pi = 2.0 * std::f32::consts::PI;

    for i in 0..resolution {
        let angle = (i as f32 / resolution as f32) * two_pi;
        let dir = Vec2::new(angle.cos(), angle.sin());

        let mut min_dist = f32::INFINITY;

        for (p1, p2) in occluders {
            if let Some(dist) = ray_segment_intersection(light.position, dir, *p1, *p2) {
                min_dist = min_dist.min(dist);
            }
        }

        shadow_map.distances[i as usize] = min_dist;
    }

    shadow_map.dirty = false;
}

/// Ray-segment intersection test.
/// Returns the distance along the ray if there's an intersection.
fn ray_segment_intersection(
    ray_origin: Vec2,
    ray_dir: Vec2,
    p1: Vec2,
    p2: Vec2,
) -> Option<f32> {
    let v1 = ray_origin - p1;
    let v2 = p2 - p1;
    let v3 = Vec2::new(-ray_dir.y, ray_dir.x);

    let dot = v2.dot(v3);
    if dot.abs() < 0.0001 {
        return None; // Parallel
    }

    let t1 = v2.cross(v1) / dot;
    let t2 = v1.dot(v3) / dot;

    if t1 >= 0.0 && t2 >= 0.0 && t2 <= 1.0 {
        Some(t1)
    } else {
        None
    }
}
