//! Unison Math — Shared types for the Unison 2D engine.
//!
//! Provides the common `Vec2`, `Color`, and `Rect` types used across all engine crates,
//! eliminating the need for ad-hoc tuples and arrays at crate boundaries.
//!
//! All types provide `From` conversions for `[f32; N]` arrays and `(f32, f32)` tuples
//! so that adoption in existing crates is incremental and non-breaking.

mod vec2;
mod color;
mod rect;

pub use vec2::Vec2;
pub use color::Color;
pub use rect::Rect;
