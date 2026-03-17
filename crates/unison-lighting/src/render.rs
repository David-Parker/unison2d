//! Rendering interface traits for platform-specific lighting implementations.

use crate::light::Light;
use crate::shadow::ShadowMapId;

/// Data for occluders to be rendered into shadow maps.
#[derive(Debug, Clone, Default)]
pub struct OccluderData {
    /// Line segments that block light.
    /// Each segment is ((start_x, start_y), (end_x, end_y)).
    pub segments: Vec<((f32, f32), (f32, f32))>,
}

impl OccluderData {
    /// Create empty occluder data.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create occluder data from segments.
    pub fn from_segments(segments: Vec<((f32, f32), (f32, f32))>) -> Self {
        Self { segments }
    }

    /// Add a segment to the occluder.
    pub fn add_segment(&mut self, start: (f32, f32), end: (f32, f32)) {
        self.segments.push((start, end));
    }

    /// Add a rectangle as four segments.
    pub fn add_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let x2 = x + width;
        let y2 = y + height;
        self.segments.push(((x, y), (x2, y)));
        self.segments.push(((x2, y), (x2, y2)));
        self.segments.push(((x2, y2), (x, y2)));
        self.segments.push(((x, y2), (x, y)));
    }

    /// Add a polygon as segments connecting consecutive vertices.
    pub fn add_polygon(&mut self, vertices: &[(f32, f32)]) {
        if vertices.len() < 2 {
            return;
        }
        for i in 0..vertices.len() {
            let next = (i + 1) % vertices.len();
            self.segments.push((vertices[i], vertices[next]));
        }
    }
}

/// Lighting data prepared for rendering.
#[derive(Debug, Clone)]
pub struct LightingData {
    /// Visible lights for this frame.
    pub lights: Vec<LightRenderData>,
    /// Ambient light color.
    pub ambient: (f32, f32, f32),
}

/// Per-light data for rendering.
#[derive(Debug, Clone)]
pub struct LightRenderData {
    /// Light position in world space.
    pub position: (f32, f32),
    /// Light color (RGB).
    pub color: (f32, f32, f32),
    /// Light intensity.
    pub intensity: f32,
    /// Light radius (for attenuation).
    pub radius: f32,
    /// Shadow map ID if shadows are enabled.
    pub shadow_map: Option<ShadowMapId>,
    /// Light type identifier for shader selection.
    pub light_type: u32,
    /// Additional parameters (direction, angle, etc).
    pub params: [f32; 4],
}

impl LightRenderData {
    /// Create render data from a Light.
    pub fn from_light(light: &Light, shadow_map: Option<ShadowMapId>) -> Self {
        use crate::light::LightType;

        let (light_type, radius, params) = match &light.light_type {
            LightType::Point { radius } => (0, *radius, [0.0, 0.0, 0.0, 0.0]),
            LightType::Spot {
                radius,
                angle,
                direction,
            } => (1, *radius, [direction.0, direction.1, *angle, 0.0]),
            LightType::Directional { direction } => {
                (2, f32::INFINITY, [direction.0, direction.1, 0.0, 0.0])
            }
            LightType::Area { width, height } => {
                (3, (width * width + height * height).sqrt() * 0.5, [*width, *height, 0.0, 0.0])
            }
        };

        Self {
            position: light.position,
            color: light.color,
            intensity: light.intensity,
            radius,
            shadow_map,
            light_type,
            params,
        }
    }
}

/// Trait for platform-specific lighting renderers.
///
/// Implement this in platform crates to provide GPU-based lighting and shadow rendering.
pub trait LightingRenderer {
    /// Create a new shadow map texture.
    fn create_shadow_map(&mut self, resolution: u32) -> ShadowMapId;

    /// Update a shadow map with new occluder data.
    fn update_shadow_map(&mut self, id: ShadowMapId, light: &Light, occluders: &[OccluderData]);

    /// Destroy a shadow map.
    fn destroy_shadow_map(&mut self, id: ShadowMapId);

    /// Bind lighting data for the current render pass.
    fn bind_lighting(
        &mut self,
        lights: &[&Light],
        ambient: (f32, f32, f32),
        shadow_maps: &[ShadowMapId],
    );

    /// Begin the lighting render pass.
    fn begin_lighting_pass(&mut self);

    /// End the lighting render pass and apply to framebuffer.
    fn end_lighting_pass(&mut self);
}

/// Null implementation of LightingRenderer for testing.
#[derive(Debug, Default)]
pub struct NullLightingRenderer {
    next_id: u32,
}

impl NullLightingRenderer {
    /// Create a new null renderer.
    pub fn new() -> Self {
        Self::default()
    }
}

impl LightingRenderer for NullLightingRenderer {
    fn create_shadow_map(&mut self, _resolution: u32) -> ShadowMapId {
        let id = ShadowMapId(self.next_id);
        self.next_id += 1;
        id
    }

    fn update_shadow_map(&mut self, _id: ShadowMapId, _light: &Light, _occluders: &[OccluderData]) {
        // No-op
    }

    fn destroy_shadow_map(&mut self, _id: ShadowMapId) {
        // No-op
    }

    fn bind_lighting(
        &mut self,
        _lights: &[&Light],
        _ambient: (f32, f32, f32),
        _shadow_maps: &[ShadowMapId],
    ) {
        // No-op
    }

    fn begin_lighting_pass(&mut self) {
        // No-op
    }

    fn end_lighting_pass(&mut self) {
        // No-op
    }
}
