//! Radial gradient texture generation for point lights.

use unison_render::{TextureDescriptor, TextureFormat};

/// Generate a radial gradient texture for point light rendering.
///
/// Produces a `size × size` RGBA texture with white RGB and alpha set to a
/// smooth quadratic falloff: `(1 - dist²)`, where `dist` is the normalized
/// distance from center (0 at center, 1 at edge).
///
/// Used as the sprite texture for point lights drawn with additive blending.
pub fn generate_radial_gradient(size: u32) -> TextureDescriptor {
    let mut data = vec![0u8; (size * size * 4) as usize];
    let center = size as f32 / 2.0;

    for y in 0..size {
        for x in 0..size {
            let dx = (x as f32 + 0.5 - center) / center;
            let dy = (y as f32 + 0.5 - center) / center;
            let dist_sq = (dx * dx + dy * dy).min(1.0);
            let intensity = (1.0 - dist_sq).max(0.0);
            let byte = (intensity * 255.0) as u8;

            let idx = ((y * size + x) * 4) as usize;
            data[idx] = 255;     // R
            data[idx + 1] = 255; // G
            data[idx + 2] = 255; // B
            data[idx + 3] = byte; // A = falloff
        }
    }

    TextureDescriptor::new(size, size, TextureFormat::Rgba8, data)
}
