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
