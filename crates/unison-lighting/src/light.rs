//! Light types and structures for 2D dynamic lighting.

use unison_math::{Vec2, Color};

/// The type of light source and its type-specific parameters.
#[derive(Debug, Clone, PartialEq)]
pub enum LightType {
    /// Omnidirectional light that falls off with distance.
    Point { radius: f32 },
    /// Cone-shaped light with direction and angle.
    Spot {
        radius: f32,
        angle: f32,
        direction: Vec2,
    },
    /// Parallel rays from a distant source (e.g., sun/moon).
    Directional { direction: Vec2 },
    /// Rectangular area light for soft illumination.
    Area { width: f32, height: f32 },
}

/// A 2D light source.
#[derive(Debug, Clone)]
pub struct Light {
    /// The type of light and its parameters.
    pub light_type: LightType,
    /// World position of the light (ignored for directional lights).
    pub position: Vec2,
    /// RGB color of the light (0.0 to 1.0 per channel).
    pub color: Color,
    /// Light intensity multiplier.
    pub intensity: f32,
    /// Whether this light casts shadows.
    pub shadows: bool,
    /// Whether this light is currently active.
    pub enabled: bool,
}

impl Light {
    /// Create a new point light.
    pub fn point(position: Vec2, radius: f32) -> Self {
        Self {
            light_type: LightType::Point { radius },
            position,
            color: Color::WHITE,
            intensity: 1.0,
            shadows: true,
            enabled: true,
        }
    }

    /// Create a new spot light.
    pub fn spot(
        position: Vec2,
        radius: f32,
        angle: f32,
        direction: Vec2,
    ) -> Self {
        Self {
            light_type: LightType::Spot {
                radius,
                angle,
                direction,
            },
            position,
            color: Color::WHITE,
            intensity: 1.0,
            shadows: true,
            enabled: true,
        }
    }

    /// Create a new directional light.
    pub fn directional(direction: Vec2) -> Self {
        Self {
            light_type: LightType::Directional { direction },
            position: Vec2::ZERO,
            color: Color::WHITE,
            intensity: 1.0,
            shadows: true,
            enabled: true,
        }
    }

    /// Create a new area light.
    pub fn area(position: Vec2, width: f32, height: f32) -> Self {
        Self {
            light_type: LightType::Area { width, height },
            position,
            color: Color::WHITE,
            intensity: 1.0,
            shadows: false, // Area lights typically don't cast hard shadows
            enabled: true,
        }
    }

    /// Set the light color.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the light intensity.
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    /// Enable or disable shadows for this light.
    pub fn with_shadows(mut self, shadows: bool) -> Self {
        self.shadows = shadows;
        self
    }

    /// Get the effective radius of the light for culling purposes.
    pub fn effective_radius(&self) -> f32 {
        match &self.light_type {
            LightType::Point { radius } => *radius,
            LightType::Spot { radius, .. } => *radius,
            LightType::Directional { .. } => f32::INFINITY,
            LightType::Area { width, height } => (width * width + height * height).sqrt() * 0.5,
        }
    }

    /// Check if the light affects a given point.
    pub fn affects_point(&self, point: Vec2) -> bool {
        if !self.enabled {
            return false;
        }

        match &self.light_type {
            LightType::Point { radius } => {
                self.position.distance_squared(point) <= radius * radius
            }
            LightType::Spot {
                radius,
                angle,
                direction,
            } => {
                let diff = point - self.position;
                let dist_sq = diff.length_squared();
                if dist_sq > radius * radius {
                    return false;
                }
                // Check if point is within cone
                let dist = dist_sq.sqrt();
                if dist < 0.0001 {
                    return true;
                }
                let to_point = diff / dist;
                let dir_norm = direction.normalized();
                if dir_norm == Vec2::ZERO {
                    return true;
                }
                let dot = to_point.dot(dir_norm);
                dot >= (angle * 0.5).cos()
            }
            LightType::Directional { .. } => true,
            LightType::Area { width, height } => {
                let half_w = width * 0.5;
                let half_h = height * 0.5;
                point.x >= self.position.x - half_w
                    && point.x <= self.position.x + half_w
                    && point.y >= self.position.y - half_h
                    && point.y <= self.position.y + half_h
            }
        }
    }
}

impl Default for Light {
    fn default() -> Self {
        Self::point(Vec2::ZERO, 10.0)
    }
}
