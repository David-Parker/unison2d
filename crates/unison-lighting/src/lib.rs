//! Unison Lighting — 2D lighting with lightmap compositing.
//!
//! Renders point lights and directional lights to an offscreen FBO (the
//! "lightmap"), then composites it over the scene with multiply blending.
//! Unlit areas are darkened to the ambient color; lit areas are tinted by
//! the light's color and intensity.

mod light;
mod system;
pub mod gradient;

pub use light::{DirectionalLight, LightId, PointLight};
pub use system::LightingSystem;
