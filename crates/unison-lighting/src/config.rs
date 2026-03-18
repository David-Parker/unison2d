//! Configuration types for loading lights from scene JSON files.

use serde::{Deserialize, Serialize};

use crate::light::{Light, LightType};
use unison_math::{Vec2, Color};

/// Configuration for a light loaded from a scene file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightConfig {
    /// Light type: "point", "spot", "directional", "area"
    pub light_type: String,
    /// Position in world coordinates (not used for directional lights).
    pub position: Option<[f32; 2]>,
    /// Direction vector (for spot and directional lights).
    pub direction: Option<[f32; 2]>,
    /// RGB color (0.0 to 1.0 per channel).
    pub color: [f32; 3],
    /// Intensity multiplier.
    pub intensity: f32,
    /// Radius for point and spot lights.
    pub radius: Option<f32>,
    /// Cone angle in radians for spot lights.
    pub angle: Option<f32>,
    /// Width for area lights.
    pub width: Option<f32>,
    /// Height for area lights.
    pub height: Option<f32>,
    /// Whether this light casts shadows.
    #[serde(default = "default_shadows")]
    pub shadows: bool,
    /// Whether this light is enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_shadows() -> bool {
    true
}

fn default_enabled() -> bool {
    true
}

impl LightConfig {
    /// Convert this config into a Light instance.
    pub fn to_light(&self) -> Result<Light, String> {
        let position: Vec2 = self.position.unwrap_or([0.0, 0.0]).into();

        let light_type = match self.light_type.as_str() {
            "point" => {
                let radius = self.radius.ok_or("point light requires 'radius'")?;
                LightType::Point { radius }
            }
            "spot" => {
                let radius = self.radius.ok_or("spot light requires 'radius'")?;
                let angle = self.angle.ok_or("spot light requires 'angle'")?;
                let direction = self
                    .direction
                    .ok_or("spot light requires 'direction'")?;
                LightType::Spot {
                    radius,
                    angle,
                    direction: direction.into(),
                }
            }
            "directional" => {
                let direction = self
                    .direction
                    .ok_or("directional light requires 'direction'")?;
                LightType::Directional {
                    direction: direction.into(),
                }
            }
            "area" => {
                let width = self.width.ok_or("area light requires 'width'")?;
                let height = self.height.ok_or("area light requires 'height'")?;
                LightType::Area { width, height }
            }
            other => return Err(format!("unknown light type: {}", other)),
        };

        Ok(Light {
            light_type,
            position,
            color: Color::rgb(self.color[0], self.color[1], self.color[2]),
            intensity: self.intensity,
            shadows: self.shadows,
            enabled: self.enabled,
        })
    }
}

/// Configuration for ambient lighting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmbientConfig {
    /// RGB color for ambient light (0.0 to 1.0 per channel).
    pub color: [f32; 3],
}

impl Default for AmbientConfig {
    fn default() -> Self {
        Self {
            color: [0.1, 0.1, 0.1],
        }
    }
}

/// Full lighting configuration for a scene.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SceneLightingConfig {
    /// Ambient light settings.
    #[serde(default)]
    pub ambient: Option<AmbientConfig>,
    /// List of lights in the scene.
    #[serde(default)]
    pub lights: Vec<LightConfig>,
}
