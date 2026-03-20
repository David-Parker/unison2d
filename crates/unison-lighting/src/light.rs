//! Light types and identifiers.

use unison_math::{Color, Vec2};

use crate::occluder::ShadowFilter;

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
    /// Whether this light casts shadows from occluders.
    pub casts_shadows: bool,
    /// PCF filter mode for shadow edges.
    pub shadow_filter: ShadowFilter,
}

impl PointLight {
    /// Create a new point light (shadows disabled by default).
    pub fn new(position: Vec2, color: Color, intensity: f32, radius: f32) -> Self {
        Self {
            position,
            color,
            intensity,
            radius,
            casts_shadows: false,
            shadow_filter: ShadowFilter::None,
        }
    }
}

/// A directional light that illuminates the entire scene uniformly.
///
/// Without normal maps, the direction field has no visual effect on shading —
/// the light acts as a uniform color wash. However, the direction IS used for
/// shadow casting (Phase 3): shadows are projected along this direction.
#[derive(Debug, Clone)]
pub struct DirectionalLight {
    /// Direction the light shines FROM (normalized).
    pub direction: Vec2,
    /// Light color.
    pub color: Color,
    /// Intensity multiplier (applied to color).
    pub intensity: f32,
    /// Whether this light casts shadows from occluders.
    pub casts_shadows: bool,
    /// PCF filter mode for shadow edges.
    pub shadow_filter: ShadowFilter,
}

impl DirectionalLight {
    /// Create a new directional light (shadows disabled by default).
    pub fn new(direction: Vec2, color: Color, intensity: f32) -> Self {
        Self {
            direction,
            color,
            intensity,
            casts_shadows: false,
            shadow_filter: ShadowFilter::None,
        }
    }
}
