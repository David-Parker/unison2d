//! Unison 2D Lighting System
//!
//! This crate provides 2D dynamic lighting with soft shadows for the Unison 2D engine.
//! It is designed for cross-platform use (WebGL, mobile) with good performance.
//!
//! # Features
//!
//! - **Multiple light types**: Point, Spot, Directional, and Area lights
//! - **Soft shadows**: 2D shadow mapping with PCF filtering
//! - **Performance optimized**: Frustum culling, dirty tracking, shadow map caching
//! - **Configurable quality**: Multiple quality presets for different hardware
//! - **Easy integration**: Simple API for adding/removing lights
//!
//! # Usage
//!
//! ```ignore
//! use unison_lighting::{LightingSystem, Light, ShadowQuality};
//! use unison_math::{Vec2, Color, Rect};
//!
//! // Create lighting manager
//! let mut lighting = LightingSystem::new();
//! lighting.set_shadow_quality(ShadowQuality::Medium);
//! lighting.set_ambient(Color::rgb(0.1, 0.1, 0.15));
//!
//! // Add lights
//! let sun = lighting.add_light(
//!     Light::directional(Vec2::new(0.5, -1.0))
//!         .with_color(Color::rgb(1.0, 0.95, 0.8))
//!         .with_intensity(0.8)
//! );
//!
//! let torch = lighting.add_light(
//!     Light::point(Vec2::new(10.0, 5.0), 8.0)
//!         .with_color(Color::rgb(1.0, 0.6, 0.2))
//!         .with_intensity(1.2)
//! );
//!
//! // Update shadows when occluders change
//! lighting.mark_all_dirty();
//! lighting.update_shadows(&occluders);
//!
//! // Get visible lights for rendering
//! let camera = Rect::from_center(Vec2::ZERO, Vec2::new(20.0, 15.0));
//! let visible = lighting.get_visible_lights(&camera);
//! ```

pub mod light;
pub mod shadow;
pub mod manager;
pub mod render;

// Re-export commonly used types
pub use light::{Light, LightType};
pub use shadow::{ShadowQuality, ShadowMap, ShadowMapId, LightHandle, ShadowCaster, ShadowMapCache};
pub use manager::LightingSystem;
pub use render::{LightingRenderer, OccluderData, LightingData, LightRenderData, NullLightingRenderer};
