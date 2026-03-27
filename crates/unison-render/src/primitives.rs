//! Geometry factory functions for common shapes.
//!
//! These produce [`RenderCommand::Mesh`] values built from [`DrawMesh`].
//! No new renderer-side code is needed — the existing mesh path handles everything.

use crate::color::Color;
use crate::renderer::{DrawMesh, RenderCommand};
use crate::texture::TextureId;
use unison_math::Vec2;

/// Default segment count for circles.
const DEFAULT_SEGMENTS: usize = 32;

/// Build a filled circle as a triangle-fan mesh.
pub fn circle(center: Vec2, radius: f32, color: Color) -> RenderCommand {
    circle_with_segments(center, radius, color, DEFAULT_SEGMENTS)
}

/// Build a filled circle with a custom segment count.
pub fn circle_with_segments(center: Vec2, radius: f32, color: Color, segments: usize) -> RenderCommand {
    let n = segments;
    let mut positions = Vec::with_capacity((n + 1) * 2);
    let mut uvs = Vec::with_capacity((n + 1) * 2);
    let mut indices = Vec::with_capacity(n * 3);

    // Center vertex
    positions.push(center.x);
    positions.push(center.y);
    uvs.push(0.5);
    uvs.push(0.5);

    for i in 0..n {
        let theta = (i as f32 / n as f32) * std::f32::consts::TAU;
        positions.push(center.x + radius * theta.cos());
        positions.push(center.y + radius * theta.sin());
        uvs.push(0.0);
        uvs.push(0.0);

        let next = if i + 1 < n { i + 2 } else { 1 };
        indices.push(0);
        indices.push((i + 1) as u32);
        indices.push(next as u32);
    }

    RenderCommand::Mesh(DrawMesh {
        positions,
        uvs,
        indices,
        texture: TextureId::NONE,
        color,
        vertex_colors: None,
    })
}

/// Build a filled circle with a radial gradient (opaque center, transparent edge).
///
/// Useful for glow effects and soft light discs. The `color` tint is applied
/// at full alpha in the center; rim vertices have alpha = 0.
pub fn gradient_circle(center: Vec2, radius: f32, color: Color) -> RenderCommand {
    gradient_circle_with_segments(center, radius, color, DEFAULT_SEGMENTS)
}

/// Build a gradient circle with a custom segment count.
pub fn gradient_circle_with_segments(center: Vec2, radius: f32, color: Color, segments: usize) -> RenderCommand {
    let n = segments;
    let mut positions = Vec::with_capacity((n + 1) * 2);
    let mut uvs = Vec::with_capacity((n + 1) * 2);
    let mut vertex_colors = Vec::with_capacity((n + 1) * 4);
    let mut indices = Vec::with_capacity(n * 3);

    // Center vertex — full alpha
    positions.push(center.x);
    positions.push(center.y);
    uvs.push(0.5);
    uvs.push(0.5);
    vertex_colors.extend_from_slice(&[1.0, 1.0, 1.0, 1.0]);

    // Rim vertices — zero alpha
    for i in 0..n {
        let theta = (i as f32 / n as f32) * std::f32::consts::TAU;
        positions.push(center.x + radius * theta.cos());
        positions.push(center.y + radius * theta.sin());
        uvs.push(0.0);
        uvs.push(0.0);
        vertex_colors.extend_from_slice(&[1.0, 1.0, 1.0, 0.0]);

        let next = if i + 1 < n { i + 2 } else { 1 };
        indices.push(0);
        indices.push((i + 1) as u32);
        indices.push(next as u32);
    }

    RenderCommand::Mesh(DrawMesh {
        positions,
        uvs,
        indices,
        texture: TextureId::NONE,
        color,
        vertex_colors: Some(vertex_colors),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circle_produces_correct_vertex_count() {
        let cmd = circle_with_segments(Vec2::ZERO, 1.0, Color::RED, 16);
        if let RenderCommand::Mesh(mesh) = cmd {
            // 1 center + 16 rim = 17 vertices, 2 floats each
            assert_eq!(mesh.positions.len(), 34);
            assert_eq!(mesh.indices.len(), 48); // 16 triangles * 3
            assert!(mesh.vertex_colors.is_none());
        } else {
            panic!("expected Mesh");
        }
    }

    #[test]
    fn gradient_circle_has_vertex_colors() {
        let cmd = gradient_circle_with_segments(Vec2::ZERO, 1.0, Color::WHITE, 8);
        if let RenderCommand::Mesh(mesh) = cmd {
            assert!(mesh.vertex_colors.is_some());
            let vc = mesh.vertex_colors.unwrap();
            // Center vertex alpha = 1.0, rim vertex alpha = 0.0
            assert_eq!(vc[3], 1.0); // center alpha
            assert_eq!(vc[7], 0.0); // first rim alpha
        } else {
            panic!("expected Mesh");
        }
    }
}
