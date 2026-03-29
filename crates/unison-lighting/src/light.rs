//! Light types and identifiers.

use unison_core::{Color, Vec2};

use crate::occluder::ShadowFilter;

/// Unique handle to a light in a [`LightingSystem`](crate::LightingSystem).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LightId(pub(crate) u32);

/// Shadow casting configuration shared by all light types.
///
/// Controls the appearance of shadows cast by a light. The light must also
/// have `casts_shadows: true` for these settings to take effect.
#[derive(Debug, Clone)]
pub struct ShadowSettings {
    /// PCF filter mode for shadow edges.
    pub filter: ShadowFilter,
    /// How dark shadows are (0.0 = no shadow, 1.0 = full black). Default 1.0.
    pub strength: f32,
    /// Maximum distance in world units that shadows extend from the occluder.
    /// At 0.0, shadows extend to the full light radius. Default 0.0.
    pub distance: f32,
    /// Controls how quickly shadows fade within the distance.
    /// At 0.0, the shadow is solid black (no fade). At higher values,
    /// the shadow fades faster. The fade follows `(1-t)^attenuation`.
    /// Try 0.5 for a gentle fade, 2.0+ for aggressive fade. Default 1.0.
    pub attenuation: f32,
}

impl Default for ShadowSettings {
    fn default() -> Self {
        Self {
            filter: ShadowFilter::None,
            strength: 1.0,
            distance: 0.0,
            attenuation: 1.0,
        }
    }
}

impl ShadowSettings {
    /// Hard shadows with default settings.
    pub fn hard() -> Self {
        Self::default()
    }

    /// Soft shadows with PCF5 filtering.
    pub fn soft() -> Self {
        Self {
            filter: ShadowFilter::Pcf5,
            ..Self::default()
        }
    }
}

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
    /// Shadow appearance settings (filter, strength, distance, attenuation).
    pub shadow: ShadowSettings,
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
            shadow: ShadowSettings::default(),
        }
    }
}

/// A directional light that illuminates the entire scene uniformly.
///
/// Without normal maps, the direction field has no visual effect on shading —
/// the light acts as a uniform color wash. However, the direction IS used for
/// shadow casting: shadows are projected along this direction.
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
    /// Shadow appearance settings (filter, strength, distance, attenuation).
    pub shadow: ShadowSettings,
}

impl DirectionalLight {
    /// Create a new directional light (shadows disabled by default).
    pub fn new(direction: Vec2, color: Color, intensity: f32) -> Self {
        Self {
            direction,
            color,
            intensity,
            casts_shadows: false,
            shadow: ShadowSettings::default(),
        }
    }
}
