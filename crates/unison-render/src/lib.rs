//! Unison Render - Rendering traits and abstractions
//!
//! This crate defines platform-agnostic rendering interfaces.
//! Platform crates provide concrete implementations.

mod color;
mod texture;
mod sprite;
mod camera;
mod renderer;
mod image;

pub use color::Color;
pub use texture::{TextureId, TextureFormat, TextureFilter, TextureWrap, TextureDescriptor};
pub use sprite::{Sprite, SpriteSheet};
pub use camera::Camera;
pub use renderer::{Renderer, RenderCommand, DrawSprite, DrawMesh, DrawLitSprite, RenderTargetId, BlendMode, AntiAliasing};
pub use self::image::decode_image;
