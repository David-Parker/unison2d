//! Light types and identifiers.

use unison_math::{Color, Vec2};

/// Unique handle to a light in a [`LightingSystem`](crate::LightingSystem).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LightId(pub(crate) u32);

/// A point light that emits in all directions with radial falloff.
#[derive(Debug, Clone)]
pub struct PointLight {
    /// Position in world space.
    pub position: Vec2,
    /// Light color.
    pub color: Color,
    /// Intensity multiplier (applied to color).
    pub intensity: f32,
    /// Radius of the light's influence in world units.
    pub radius: f32,
}

impl PointLight {
    /// Create a new point light.
    pub fn new(position: Vec2, color: Color, intensity: f32, radius: f32) -> Self {
        Self { position, color, intensity, radius }
    }
}

/// A directional light that illuminates the entire scene uniformly.
///
/// Without normal maps, the direction field has no visual effect — the light
/// acts as a uniform color wash. The direction is stored for forward
/// compatibility with Phase 4 (normal maps + per-pixel lighting).
#[derive(Debug, Clone)]
pub struct DirectionalLight {
    /// Direction the light shines FROM (normalized). Stored for Phase 4.
    pub direction: Vec2,
    /// Light color.
    pub color: Color,
    /// Intensity multiplier (applied to color).
    pub intensity: f32,
}

impl DirectionalLight {
    /// Create a new directional light.
    pub fn new(direction: Vec2, color: Color, intensity: f32) -> Self {
        Self { direction, color, intensity }
    }
}
