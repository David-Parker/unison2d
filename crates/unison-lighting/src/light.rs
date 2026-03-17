//! Light types and structures for 2D dynamic lighting.

/// The type of light source and its type-specific parameters.
#[derive(Debug, Clone, PartialEq)]
pub enum LightType {
    /// Omnidirectional light that falls off with distance.
    Point { radius: f32 },
    /// Cone-shaped light with direction and angle.
    Spot {
        radius: f32,
        angle: f32,
        direction: (f32, f32),
    },
    /// Parallel rays from a distant source (e.g., sun/moon).
    Directional { direction: (f32, f32) },
    /// Rectangular area light for soft illumination.
    Area { width: f32, height: f32 },
}

/// A 2D light source.
#[derive(Debug, Clone)]
pub struct Light {
    /// The type of light and its parameters.
    pub light_type: LightType,
    /// World position of the light (ignored for directional lights).
    pub position: (f32, f32),
    /// RGB color of the light (0.0 to 1.0 per channel).
    pub color: (f32, f32, f32),
    /// Light intensity multiplier.
    pub intensity: f32,
    /// Whether this light casts shadows.
    pub shadows: bool,
    /// Whether this light is currently active.
    pub enabled: bool,
}

impl Light {
    /// Create a new point light.
    pub fn point(position: (f32, f32), radius: f32) -> Self {
        Self {
            light_type: LightType::Point { radius },
            position,
            color: (1.0, 1.0, 1.0),
            intensity: 1.0,
            shadows: true,
            enabled: true,
        }
    }

    /// Create a new spot light.
    pub fn spot(
        position: (f32, f32),
        radius: f32,
        angle: f32,
        direction: (f32, f32),
    ) -> Self {
        Self {
            light_type: LightType::Spot {
                radius,
                angle,
                direction,
            },
            position,
            color: (1.0, 1.0, 1.0),
            intensity: 1.0,
            shadows: true,
            enabled: true,
        }
    }

    /// Create a new directional light.
    pub fn directional(direction: (f32, f32)) -> Self {
        Self {
            light_type: LightType::Directional { direction },
            position: (0.0, 0.0),
            color: (1.0, 1.0, 1.0),
            intensity: 1.0,
            shadows: true,
            enabled: true,
        }
    }

    /// Create a new area light.
    pub fn area(position: (f32, f32), width: f32, height: f32) -> Self {
        Self {
            light_type: LightType::Area { width, height },
            position,
            color: (1.0, 1.0, 1.0),
            intensity: 1.0,
            shadows: false, // Area lights typically don't cast hard shadows
            enabled: true,
        }
    }

    /// Set the light color.
    pub fn with_color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.color = (r, g, b);
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
    pub fn affects_point(&self, point: (f32, f32)) -> bool {
        if !self.enabled {
            return false;
        }

        match &self.light_type {
            LightType::Point { radius } => {
                let dx = point.0 - self.position.0;
                let dy = point.1 - self.position.1;
                dx * dx + dy * dy <= radius * radius
            }
            LightType::Spot {
                radius,
                angle,
                direction,
            } => {
                let dx = point.0 - self.position.0;
                let dy = point.1 - self.position.1;
                let dist_sq = dx * dx + dy * dy;
                if dist_sq > radius * radius {
                    return false;
                }
                // Check if point is within cone
                let dist = dist_sq.sqrt();
                if dist < 0.0001 {
                    return true;
                }
                let to_point = (dx / dist, dy / dist);
                let dir_len = (direction.0 * direction.0 + direction.1 * direction.1).sqrt();
                if dir_len < 0.0001 {
                    return true;
                }
                let dir_norm = (direction.0 / dir_len, direction.1 / dir_len);
                let dot = to_point.0 * dir_norm.0 + to_point.1 * dir_norm.1;
                dot >= (angle * 0.5).cos()
            }
            LightType::Directional { .. } => true,
            LightType::Area { width, height } => {
                let half_w = width * 0.5;
                let half_h = height * 0.5;
                point.0 >= self.position.0 - half_w
                    && point.0 <= self.position.0 + half_w
                    && point.1 >= self.position.1 - half_h
                    && point.1 <= self.position.1 + half_h
            }
        }
    }
}

impl Default for Light {
    fn default() -> Self {
        Self::point((0.0, 0.0), 10.0)
    }
}
