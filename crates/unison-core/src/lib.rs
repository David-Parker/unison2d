//! Unison Core — Shared primitives for the Unison 2D engine.
//!
//! Provides the common `Vec2`, `Color`, `Rect`, and `EventSink` types used across all
//! engine crates. This is the lowest-level crate in the dependency graph — every other
//! engine crate depends on it.

mod vec2;
mod color;
mod rect;
mod rng;
mod event_sink;

pub use vec2::Vec2;
pub use color::Color;
pub use rect::Rect;
pub use rng::Rng;
pub use event_sink::EventSink;

/// Linear interpolation between two f32 values.
/// `t=0` returns `a`, `t=1` returns `b`.
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Hermite smoothstep: smooth ease-in/ease-out for `t` in \[0, 1\].
/// Returns 0 at `t=0`, 1 at `t=1`, with zero derivative at both endpoints.
#[inline]
pub fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lerp_f32() {
        assert!((lerp(0.0, 10.0, 0.0)).abs() < 1e-6);
        assert!((lerp(0.0, 10.0, 1.0) - 10.0).abs() < 1e-6);
        assert!((lerp(0.0, 10.0, 0.5) - 5.0).abs() < 1e-6);
        assert!((lerp(-5.0, 5.0, 0.5)).abs() < 1e-6);
    }

    #[test]
    fn test_smoothstep() {
        assert!((smoothstep(0.0)).abs() < 1e-6);
        assert!((smoothstep(1.0) - 1.0).abs() < 1e-6);
        assert!((smoothstep(0.5) - 0.5).abs() < 1e-6);
        // Verify it's not linear — 0.25 should not map to 0.25
        assert!((smoothstep(0.25) - 0.15625).abs() < 1e-6);
    }
}
