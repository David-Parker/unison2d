//! Unison Lighting — 2D lighting with lightmap compositing and shadow casting.
//!
//! Renders point lights and directional lights to an offscreen FBO (the
//! "lightmap"), then composites it over the scene with multiply blending.
//! Unlit areas are darkened to the ambient color; lit areas are tinted by
//! the light's color and intensity.
//!
//! Shadow casting uses a per-light shadow mask: occluder geometry is projected
//! away from each light and rendered to a shadow FBO, which is then sampled
//! by the lit-sprite shader with optional PCF filtering.

mod light;
pub mod occluder;
pub mod shadow;
mod system;
pub mod gradient;

pub use light::{DirectionalLight, LightId, PointLight};
pub use occluder::{Occluder, OccluderEdge, ShadowFilter};
pub use shadow::{ShadowQuad, compute_boundary_edges};
pub use system::LightingSystem;
