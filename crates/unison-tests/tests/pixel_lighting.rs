#![allow(dead_code)]
//! Pixel-level lighting tests.
//!
//! These tests simulate the full GPU rendering pipeline in software:
//! lightmap rendering (ambient + additive lights + shadows) → multiply composite
//! over a scene buffer. This catches bugs that the MockRenderer-based tests miss
//! because those only verify call sequences, not actual pixel values.
//!
//! Two approaches:
//!
//! **Approach A (standalone simulator):** Reimplements the rendering math in pure
//! Rust to test the expected output given known inputs. Catches math bugs.
//!
//! **Approach B (PixelRenderer):** A `Renderer` trait implementation that actually
//! rasterizes to pixel buffers. The real `LightingSystem` code drives it, so we
//! exercise the actual orchestration: FBO binds, shadow mask rendering, UV mapping,
//! blend modes, composite pass. Catches integration/wiring bugs.

use std::collections::HashMap;
use unison_lighting::gradient::generate_radial_gradient;
use unison_lighting::shadow::{project_directional_shadows, project_point_shadows};
use unison_lighting::{DirectionalLight, LightingSystem, Occluder, PointLight, ShadowFilter, ShadowSettings};
use unison_math::{Color, Vec2};
use unison_render::{
    BlendMode, Camera, RenderCommand, RenderTargetId, Renderer, TextureDescriptor, TextureId,
};

// ── Pixel buffer ──

/// A simple RGBA pixel buffer for software rasterization.
#[derive(Clone)]
struct PixelBuffer {
    width: u32,
    height: u32,
    /// RGBA pixels, row-major, bottom-to-top (matches OpenGL FBO orientation).
    /// Each pixel is [r, g, b, a] as f32 in 0.0..=1.0+
    data: Vec<[f32; 4]>,
}

impl PixelBuffer {
    fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![[0.0, 0.0, 0.0, 0.0]; (width * height) as usize],
        }
    }

    fn clear(&mut self, color: [f32; 4]) {
        for pixel in &mut self.data {
            *pixel = color;
        }
    }

    fn get(&self, x: u32, y: u32) -> [f32; 4] {
        self.data[(y * self.width + x) as usize]
    }

    fn set(&mut self, x: u32, y: u32, color: [f32; 4]) {
        self.data[(y * self.width + x) as usize] = color;
    }

    /// Sample at UV coordinates (0..1, 0..1), nearest neighbor.
    fn sample_uv(&self, u: f32, v: f32) -> [f32; 4] {
        let x = ((u * self.width as f32) as u32).min(self.width - 1);
        let y = ((v * self.height as f32) as u32).min(self.height - 1);
        self.get(x, y)
    }

    /// Sample brightness (average of RGB) at a pixel.
    fn brightness(&self, x: u32, y: u32) -> f32 {
        let p = self.get(x, y);
        (p[0] + p[1] + p[2]) / 3.0
    }
}

// ── Rasterization helpers ──

/// Rasterize a filled triangle into a pixel buffer with the given color and blend mode.
fn rasterize_triangle(
    buf: &mut PixelBuffer,
    v0: [f32; 2],
    v1: [f32; 2],
    v2: [f32; 2],
    color: [f32; 4],
    blend: BlendFn,
) {
    // Bounding box
    let min_x = v0[0].min(v1[0]).min(v2[0]).max(0.0) as u32;
    let max_x = (v0[0].max(v1[0]).max(v2[0]).ceil() as u32).min(buf.width - 1);
    let min_y = v0[1].min(v1[1]).min(v2[1]).max(0.0) as u32;
    let max_y = (v0[1].max(v1[1]).max(v2[1]).ceil() as u32).min(buf.height - 1);

    for py in min_y..=max_y {
        for px in min_x..=max_x {
            let p = [px as f32 + 0.5, py as f32 + 0.5];
            if point_in_triangle(p, v0, v1, v2) {
                let dst = buf.get(px, py);
                buf.set(px, py, blend(color, dst));
            }
        }
    }
}

/// Rasterize a filled triangle with per-vertex colors (barycentric interpolation).
/// The vertex color is multiplied component-wise with the solid color.
fn rasterize_triangle_vc(
    buf: &mut PixelBuffer,
    v0: [f32; 2],
    v1: [f32; 2],
    v2: [f32; 2],
    color: [f32; 4],
    vc0: [f32; 4],
    vc1: [f32; 4],
    vc2: [f32; 4],
    blend: BlendFn,
) {
    let min_x = v0[0].min(v1[0]).min(v2[0]).max(0.0) as u32;
    let max_x = (v0[0].max(v1[0]).max(v2[0]).ceil() as u32).min(buf.width - 1);
    let min_y = v0[1].min(v1[1]).min(v2[1]).max(0.0) as u32;
    let max_y = (v0[1].max(v1[1]).max(v2[1]).ceil() as u32).min(buf.height - 1);

    // Precompute denominator for barycentric coordinates
    let denom = (v1[1] - v2[1]) * (v0[0] - v2[0]) + (v2[0] - v1[0]) * (v0[1] - v2[1]);
    if denom.abs() < 1e-10 {
        return; // degenerate triangle
    }
    let inv_denom = 1.0 / denom;

    for py in min_y..=max_y {
        for px in min_x..=max_x {
            let p = [px as f32 + 0.5, py as f32 + 0.5];
            if !point_in_triangle(p, v0, v1, v2) {
                continue;
            }

            // Barycentric coordinates
            let w0 = ((v1[1] - v2[1]) * (p[0] - v2[0]) + (v2[0] - v1[0]) * (p[1] - v2[1])) * inv_denom;
            let w1 = ((v2[1] - v0[1]) * (p[0] - v2[0]) + (v0[0] - v2[0]) * (p[1] - v2[1])) * inv_denom;
            let w2 = 1.0 - w0 - w1;

            // Interpolate vertex color
            let interp = [
                vc0[0] * w0 + vc1[0] * w1 + vc2[0] * w2,
                vc0[1] * w0 + vc1[1] * w1 + vc2[1] * w2,
                vc0[2] * w0 + vc1[2] * w1 + vc2[2] * w2,
                vc0[3] * w0 + vc1[3] * w1 + vc2[3] * w2,
            ];

            // Multiply vertex color with solid color
            let src = [
                color[0] * interp[0],
                color[1] * interp[1],
                color[2] * interp[2],
                color[3] * interp[3],
            ];

            let dst = buf.get(px, py);
            buf.set(px, py, blend(src, dst));
        }
    }
}

/// Rasterize a quad (two triangles) given 4 vertices in order.
fn rasterize_quad(
    buf: &mut PixelBuffer,
    verts: [[f32; 2]; 4],
    color: [f32; 4],
    blend: BlendFn,
) {
    rasterize_triangle(buf, verts[0], verts[1], verts[2], color, blend);
    rasterize_triangle(buf, verts[0], verts[2], verts[3], color, blend);
}

/// Rasterize an axis-aligned sprite (position = center, size = full extent).
fn rasterize_sprite(
    buf: &mut PixelBuffer,
    center_px: [f32; 2],
    size_px: [f32; 2],
    color: [f32; 4],
    blend: BlendFn,
) {
    let half_w = size_px[0] / 2.0;
    let half_h = size_px[1] / 2.0;
    let verts = [
        [center_px[0] - half_w, center_px[1] - half_h],
        [center_px[0] + half_w, center_px[1] - half_h],
        [center_px[0] + half_w, center_px[1] + half_h],
        [center_px[0] - half_w, center_px[1] + half_h],
    ];
    rasterize_quad(buf, verts, color, blend);
}

fn point_in_triangle(p: [f32; 2], v0: [f32; 2], v1: [f32; 2], v2: [f32; 2]) -> bool {
    let d1 = sign(p, v0, v1);
    let d2 = sign(p, v1, v2);
    let d3 = sign(p, v2, v0);
    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);
    !(has_neg && has_pos)
}

fn sign(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    (p[0] - b[0]) * (a[1] - b[1]) - (a[0] - b[0]) * (p[1] - b[1])
}

// ── Blend functions ──

type BlendFn = fn([f32; 4], [f32; 4]) -> [f32; 4];

/// Standard alpha blending: src * srcA + dst * (1 - srcA)
fn blend_alpha(src: [f32; 4], dst: [f32; 4]) -> [f32; 4] {
    let a = src[3];
    [
        src[0] * a + dst[0] * (1.0 - a),
        src[1] * a + dst[1] * (1.0 - a),
        src[2] * a + dst[2] * (1.0 - a),
        a + dst[3] * (1.0 - a),
    ]
}

/// Additive blending: src * srcA + dst
fn blend_additive(src: [f32; 4], dst: [f32; 4]) -> [f32; 4] {
    let a = src[3];
    [
        src[0] * a + dst[0],
        src[1] * a + dst[1],
        src[2] * a + dst[2],
        1.0, // lightmap alpha doesn't matter for compositing
    ]
}

/// Multiply blending: src * dst
fn blend_multiply(src: [f32; 4], dst: [f32; 4]) -> [f32; 4] {
    [
        src[0] * dst[0],
        src[1] * dst[1],
        src[2] * dst[2],
        src[3] * dst[3],
    ]
}

// ── Coordinate conversion ──

/// Convert world position to pixel coordinates in the buffer.
fn world_to_pixel(camera: &Camera, buf_w: u32, buf_h: u32, world_x: f32, world_y: f32) -> [f32; 2] {
    let (min_x, min_y, max_x, max_y) = camera.bounds();
    let cam_w = max_x - min_x;
    let cam_h = max_y - min_y;
    // Map world -> 0..1 -> pixel
    let u = (world_x - min_x) / cam_w;
    // In FBO, Y=0 is bottom, so world min_y maps to pixel row 0
    let v = (world_y - min_y) / cam_h;
    [u * buf_w as f32, v * buf_h as f32]
}

/// Convert pixel coordinates to world position.
fn pixel_to_world(camera: &Camera, buf_w: u32, buf_h: u32, px: u32, py: u32) -> [f32; 2] {
    let (min_x, min_y, max_x, max_y) = camera.bounds();
    let cam_w = max_x - min_x;
    let cam_h = max_y - min_y;
    let u = (px as f32 + 0.5) / buf_w as f32;
    let v = (py as f32 + 0.5) / buf_h as f32;
    [min_x + u * cam_w, min_y + v * cam_h]
}

/// Sample the buffer at a world position (nearest pixel).
fn sample_at_world(
    buf: &PixelBuffer,
    camera: &Camera,
    world_x: f32,
    world_y: f32,
) -> [f32; 4] {
    let px = world_to_pixel(camera, buf.width, buf.height, world_x, world_y);
    let x = (px[0] as u32).min(buf.width - 1);
    let y = (px[1] as u32).min(buf.height - 1);
    buf.get(x, y)
}

fn brightness_at_world(buf: &PixelBuffer, camera: &Camera, world_x: f32, world_y: f32) -> f32 {
    let p = sample_at_world(buf, camera, world_x, world_y);
    (p[0] + p[1] + p[2]) / 3.0
}

// ── Gradient texture ──

/// Build the radial gradient as a PixelBuffer (matches generate_radial_gradient).
fn build_gradient_buffer(size: u32) -> PixelBuffer {
    let desc = generate_radial_gradient(size);
    let mut buf = PixelBuffer::new(size, size);
    for y in 0..size {
        for x in 0..size {
            let idx = ((y * size + x) * 4) as usize;
            buf.set(x, y, [
                desc.data[idx] as f32 / 255.0,
                desc.data[idx + 1] as f32 / 255.0,
                desc.data[idx + 2] as f32 / 255.0,
                desc.data[idx + 3] as f32 / 255.0,
            ]);
        }
    }
    buf
}

/// Sample the gradient at a normalized distance from center (0=center, 1=edge).
fn gradient_alpha_at_distance(normalized_dist: f32) -> f32 {
    if normalized_dist >= 1.0 {
        return 0.0;
    }
    (1.0 - normalized_dist * normalized_dist).max(0.0)
}

// ── Shadow mask rendering ──

/// Render the shadow mask for a point light: white = lit, black = shadowed.
fn render_shadow_mask_point(
    camera: &Camera,
    buf_w: u32,
    buf_h: u32,
    light: &PointLight,
    occluders: &[Occluder],
) -> PixelBuffer {
    let mut mask = PixelBuffer::new(buf_w, buf_h);
    mask.clear([1.0, 1.0, 1.0, 1.0]); // white = fully lit

    let quads = project_point_shadows(
        [light.position.x, light.position.y],
        light.radius,
        occluders,
        light.shadow.distance,
        light.shadow.attenuation,
    );

    for quad in &quads {
        // Convert quad vertices from world to pixel space
        let verts = [
            world_to_pixel(camera, buf_w, buf_h, quad.positions[0], quad.positions[1]),
            world_to_pixel(camera, buf_w, buf_h, quad.positions[2], quad.positions[3]),
            world_to_pixel(camera, buf_w, buf_h, quad.positions[4], quad.positions[5]),
            world_to_pixel(camera, buf_w, buf_h, quad.positions[6], quad.positions[7]),
        ];
        rasterize_quad(&mut mask, verts, [0.0, 0.0, 0.0, 1.0], blend_alpha);
    }

    mask
}

/// Render the shadow mask for a directional light.
fn render_shadow_mask_directional(
    camera: &Camera,
    buf_w: u32,
    buf_h: u32,
    light: &DirectionalLight,
    occluders: &[Occluder],
) -> PixelBuffer {
    let mut mask = PixelBuffer::new(buf_w, buf_h);
    mask.clear([1.0, 1.0, 1.0, 1.0]);

    let (min_x, min_y, max_x, max_y) = camera.bounds();
    let cam_w = max_x - min_x;
    let cam_h = max_y - min_y;
    let cast_distance = (cam_w * cam_w + cam_h * cam_h).sqrt();

    let quads = project_directional_shadows(
        [light.direction.x, light.direction.y],
        cast_distance,
        occluders,
        light.shadow.distance,
        light.shadow.attenuation,
    );

    for quad in &quads {
        let verts = [
            world_to_pixel(camera, buf_w, buf_h, quad.positions[0], quad.positions[1]),
            world_to_pixel(camera, buf_w, buf_h, quad.positions[2], quad.positions[3]),
            world_to_pixel(camera, buf_w, buf_h, quad.positions[4], quad.positions[5]),
            world_to_pixel(camera, buf_w, buf_h, quad.positions[6], quad.positions[7]),
        ];
        rasterize_quad(&mut mask, verts, [0.0, 0.0, 0.0, 1.0], blend_alpha);
    }

    mask
}

// ── Full pipeline simulation ──

/// Render the full lightmap for a scene, returning the lightmap pixel buffer.
///
/// This simulates the GPU pipeline:
/// 1. For each shadow-casting light, render a shadow mask FBO
/// 2. Clear lightmap to ambient color
/// 3. For each light, draw additively (sampling shadow mask if applicable)
fn render_lightmap(
    camera: &Camera,
    buf_w: u32,
    buf_h: u32,
    ambient: Color,
    point_lights: &[PointLight],
    directional_lights: &[DirectionalLight],
    occluders: &[Occluder],
    ground_shadow_y: Option<f32>,
) -> PixelBuffer {
    let mut lightmap = PixelBuffer::new(buf_w, buf_h);
    lightmap.clear([ambient.r, ambient.g, ambient.b, ambient.a]);

    // Build combined occluders (object occluders + ground)
    let mut all_occluders = occluders.to_vec();
    if let Some(ground_y) = ground_shadow_y {
        let (min_x, _, max_x, _) = camera.bounds();
        let margin = (max_x - min_x) * 0.5;
        all_occluders.push(Occluder::from_ground(ground_y, min_x - margin, max_x + margin));
    }

    let has_occluders = !all_occluders.is_empty();

    // ── Point lights ──
    for light in point_lights {
        let shadow_mask = if light.casts_shadows && has_occluders {
            Some(render_shadow_mask_point(camera, buf_w, buf_h, light, &all_occluders))
        } else {
            None
        };

        let color = [
            light.color.r * light.intensity,
            light.color.g * light.intensity,
            light.color.b * light.intensity,
            1.0,
        ];

        // For each pixel, compute the light contribution
        for py in 0..buf_h {
            for px in 0..buf_w {
                let world_pos = pixel_to_world(camera, buf_w, buf_h, px, py);

                // Distance from light center
                let dx = world_pos[0] - light.position.x;
                let dy = world_pos[1] - light.position.y;
                let dist = (dx * dx + dy * dy).sqrt();
                let normalized_dist = dist / light.radius;

                let gradient_alpha = gradient_alpha_at_distance(normalized_dist);
                if gradient_alpha <= 0.0 {
                    continue;
                }

                // Shadow sampling
                let shadow = if let Some(ref mask) = shadow_mask {
                    // Map pixel to shadow mask UV (same as shader: gl_FragCoord.xy / u_screen_size)
                    let u = (px as f32 + 0.5) / buf_w as f32;
                    let v = (py as f32 + 0.5) / buf_h as f32;
                    let raw_shadow = mask.sample_uv(u, v)[0]; // .r channel
                    // Apply shadow strength: mix(1.0, shadow, strength)
                    1.0 - light.shadow.strength * (1.0 - raw_shadow)
                } else {
                    1.0
                };

                // Lit shader: frag_color = vec4(light.rgb * shadow, light.a)
                // where light = gradient_texture * u_color
                // gradient_texture = (1, 1, 1, gradient_alpha)
                // so light = (color.r, color.g, color.b, gradient_alpha * color.a)
                let frag = [
                    color[0] * shadow,
                    color[1] * shadow,
                    color[2] * shadow,
                    gradient_alpha * color[3], // alpha NOT affected by shadow
                ];

                // Additive blend onto lightmap
                let dst = lightmap.get(px, py);
                lightmap.set(px, py, blend_additive(frag, dst));
            }
        }
    }

    // ── Directional lights ──
    for light in directional_lights {
        let shadow_mask = if light.casts_shadows && has_occluders {
            Some(render_shadow_mask_directional(camera, buf_w, buf_h, light, &all_occluders))
        } else {
            None
        };

        let color = [
            light.color.r * light.intensity,
            light.color.g * light.intensity,
            light.color.b * light.intensity,
            1.0,
        ];

        // Directional lights cover the entire viewport
        for py in 0..buf_h {
            for px in 0..buf_w {
                let shadow = if let Some(ref mask) = shadow_mask {
                    let u = (px as f32 + 0.5) / buf_w as f32;
                    let v = (py as f32 + 0.5) / buf_h as f32;
                    let raw_shadow = mask.sample_uv(u, v)[0];
                    // Apply shadow strength: mix(1.0, shadow, strength)
                    1.0 - light.shadow.strength * (1.0 - raw_shadow)
                } else {
                    1.0
                };

                // No texture for directional: light = u_color
                let frag = [
                    color[0] * shadow,
                    color[1] * shadow,
                    color[2] * shadow,
                    color[3],
                ];

                let dst = lightmap.get(px, py);
                lightmap.set(px, py, blend_additive(frag, dst));
            }
        }
    }

    lightmap
}

/// Composite a lightmap over a scene buffer using multiply blending.
/// Returns the final composited buffer.
fn composite(scene: &PixelBuffer, lightmap: &PixelBuffer) -> PixelBuffer {
    assert_eq!(scene.width, lightmap.width);
    assert_eq!(scene.height, lightmap.height);

    let mut result = scene.clone();
    for i in 0..result.data.len() {
        result.data[i] = blend_multiply(lightmap.data[i], scene.data[i]);
    }
    result
}

/// Helper: render a full lit scene (scene fill + lightmap + composite).
fn render_lit_scene(
    camera: &Camera,
    buf_w: u32,
    buf_h: u32,
    scene_color: Color,
    ambient: Color,
    point_lights: &[PointLight],
    directional_lights: &[DirectionalLight],
    occluders: &[Occluder],
    ground_shadow_y: Option<f32>,
) -> PixelBuffer {
    // Scene pass: solid color fill
    let mut scene = PixelBuffer::new(buf_w, buf_h);
    scene.clear([scene_color.r, scene_color.g, scene_color.b, scene_color.a]);

    let lightmap = render_lightmap(
        camera,
        buf_w,
        buf_h,
        ambient,
        point_lights,
        directional_lights,
        occluders,
        ground_shadow_y,
    );

    composite(&scene, &lightmap)
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

// ── Lightmap-only tests (no composite) ──

#[test]
fn lightmap_ambient_only_is_uniform() {
    let camera = Camera::new(20.0, 15.0);
    let ambient = Color::new(0.1, 0.1, 0.15, 1.0);

    let lightmap = render_lightmap(&camera, 80, 60, ambient, &[], &[], &[], None);

    // Every pixel should be the ambient color
    for py in 0..60 {
        for px in 0..80 {
            let p = lightmap.get(px, py);
            assert!(
                (p[0] - 0.1).abs() < 0.01 && (p[1] - 0.1).abs() < 0.01 && (p[2] - 0.15).abs() < 0.01,
                "pixel ({},{}) should be ambient, got {:?}",
                px, py, p
            );
        }
    }
}

#[test]
fn lightmap_point_light_bright_at_center() {
    let camera = Camera::new(20.0, 15.0);
    let ambient = Color::new(0.05, 0.05, 0.08, 1.0);

    let light = PointLight {
        position: Vec2::new(0.0, 0.0), // camera center
        color: Color::WHITE,
        intensity: 1.0,
        radius: 5.0,
        casts_shadows: false,
        shadow: ShadowSettings::default(),
    };

    let lightmap = render_lightmap(&camera, 100, 75, ambient, &[light], &[], &[], None);

    // Center of buffer (50, 37) maps to world (0, 0) — light center
    let center = lightmap.brightness(50, 37);
    // Edge of buffer maps to world (10, 7.5) — outside radius 5
    let edge = lightmap.brightness(0, 0);

    eprintln!("Center brightness: {}", center);
    eprintln!("Edge brightness: {}", edge);

    assert!(center > 0.5, "center should be bright, got {}", center);
    assert!(edge < 0.15, "edge should be dim (ambient only), got {}", edge);
    assert!(center > edge * 3.0, "center should be much brighter than edge");
}

#[test]
fn lightmap_point_light_gradual_falloff() {
    let camera = Camera::new(20.0, 15.0);
    let ambient = Color::BLACK;

    let light = PointLight {
        position: Vec2::new(0.0, 0.0),
        color: Color::WHITE,
        intensity: 1.0,
        radius: 8.0,
        casts_shadows: false,
        shadow: ShadowSettings::default(),
    };

    let lightmap = render_lightmap(&camera, 200, 150, ambient, &[light], &[], &[], None);

    // Sample brightness at increasing distances from center (along x-axis, y=center)
    let center_y = 75;
    let mut prev_brightness = f32::MAX;
    for dist_px in (0..90).step_by(10) {
        let px = 100 + dist_px; // center_x + offset
        let b = lightmap.brightness(px as u32, center_y);
        assert!(
            b <= prev_brightness + 0.01, // allow tiny floating point noise
            "brightness should decrease with distance: at px={}, b={}, prev={}",
            px, b, prev_brightness
        );
        prev_brightness = b;
    }
}

#[test]
fn lightmap_shadow_blocks_light_behind_box() {
    let camera = Camera::new(20.0, 15.0);
    let ambient = Color::BLACK;

    // Light above, box in the middle, shadow below
    let light = PointLight {
        position: Vec2::new(0.0, 5.0),
        color: Color::WHITE,
        intensity: 1.0,
        radius: 12.0,
        casts_shadows: true,
        shadow: ShadowSettings::default(),
    };
    let occluders = vec![Occluder::from_aabb(0.0, 0.0, 1.0, 1.0)];

    let lightmap = render_lightmap(&camera, 200, 150, ambient, &[light], &[], &occluders, None);

    // Point above box (y=3, within light radius, not blocked) — should be lit
    let above = brightness_at_world(&lightmap, &camera, 0.0, 3.0);
    // Point below box (y=-3, behind box from light's POV) — should be in shadow
    let below = brightness_at_world(&lightmap, &camera, 0.0, -3.0);
    // Point to the side (x=5, y=0, not blocked) — should be lit
    let side = brightness_at_world(&lightmap, &camera, 5.0, 0.0);

    eprintln!("Above box: {}", above);
    eprintln!("Below box (shadow): {}", below);
    eprintln!("Side (not blocked): {}", side);

    assert!(above > 0.1, "above box should be lit, got {}", above);
    assert!(below < 0.01, "below box should be in shadow, got {}", below);
    assert!(side > 0.05, "side should be lit, got {}", side);
}

#[test]
fn lightmap_shadow_does_not_block_light_in_front_of_box() {
    let camera = Camera::new(20.0, 15.0);
    let ambient = Color::BLACK;

    // Light at left, box to the right, shadow should be to the RIGHT of box
    let light = PointLight {
        position: Vec2::new(-5.0, 0.0),
        color: Color::WHITE,
        intensity: 1.0,
        radius: 15.0,
        casts_shadows: true,
        shadow: ShadowSettings::default(),
    };
    let occluders = vec![Occluder::from_aabb(0.0, 0.0, 1.0, 1.0)];

    let lightmap = render_lightmap(&camera, 200, 150, ambient, &[light], &[], &occluders, None);

    // Between light and box (x=-3) — should be lit
    let in_front = brightness_at_world(&lightmap, &camera, -3.0, 0.0);
    // Behind box (x=3) — should be in shadow
    let behind = brightness_at_world(&lightmap, &camera, 3.0, 0.0);

    eprintln!("In front of box: {}", in_front);
    eprintln!("Behind box: {}", behind);

    assert!(in_front > 0.1, "in front should be lit, got {}", in_front);
    assert!(behind < 0.01, "behind should be shadow, got {}", behind);
}

#[test]
fn lightmap_directional_light_uniform_without_shadows() {
    let camera = Camera::new(20.0, 15.0);
    let ambient = Color::BLACK;

    let dir_light = DirectionalLight::new(
        Vec2::new(0.3, -1.0),
        Color::new(0.2, 0.2, 0.35, 1.0),
        1.0,
    );

    let lightmap = render_lightmap(
        &camera, 100, 75, ambient, &[], &[dir_light], &[], None,
    );

    // All pixels should have uniform directional light contribution
    let center = lightmap.get(50, 37);
    let corner = lightmap.get(5, 5);

    eprintln!("Center: {:?}", center);
    eprintln!("Corner: {:?}", corner);

    // Should be close to the directional light color
    assert!((center[0] - 0.2).abs() < 0.02, "R channel");
    assert!((center[2] - 0.35).abs() < 0.02, "B channel");
    // Should be uniform
    assert!(
        (center[0] - corner[0]).abs() < 0.02,
        "directional light should be uniform across viewport"
    );
}

#[test]
fn lightmap_directional_shadow_creates_stripe() {
    let camera = Camera::new(20.0, 15.0);
    let ambient = Color::BLACK;

    // Directional light shining straight down
    let dir_light = DirectionalLight {
        direction: Vec2::new(0.0, -1.0),
        color: Color::WHITE,
        intensity: 1.0,
        casts_shadows: true,
        shadow: ShadowSettings::default(),
    };

    // Wide platform
    let occluders = vec![Occluder::from_aabb(0.0, 2.0, 5.0, 0.5)];

    let lightmap = render_lightmap(
        &camera, 200, 150, ambient, &[], &[dir_light], &occluders, None,
    );

    // Above the platform (y=5) — should be lit (light shines down, hits the top face)
    let above = brightness_at_world(&lightmap, &camera, 0.0, 5.0);
    // Below the platform (y=-2) — should be in shadow
    let below = brightness_at_world(&lightmap, &camera, 0.0, -2.0);
    // Far to the side (x=8, y=-2) — should be lit (not under the platform)
    let side = brightness_at_world(&lightmap, &camera, 8.0, -2.0);

    eprintln!("Above platform: {}", above);
    eprintln!("Below platform (shadow): {}", below);
    eprintln!("Side of platform: {}", side);

    assert!(above > 0.5, "above should be lit, got {}", above);
    assert!(below < 0.01, "below should be in shadow, got {}", below);
    assert!(side > 0.5, "side should be lit, got {}", side);
}

// ── Full composite tests (scene + lightmap) ──

#[test]
fn composite_darkens_unlit_areas() {
    let camera = Camera::new(20.0, 15.0);
    let scene_color = Color::new(0.8, 0.6, 0.4, 1.0); // tan color
    let ambient = Color::new(0.1, 0.1, 0.1, 1.0);

    // No lights, just ambient
    let result = render_lit_scene(
        &camera, 100, 75, scene_color, ambient, &[], &[], &[], None,
    );

    // Every pixel should be scene * ambient (much darker)
    let p = result.get(50, 37);
    eprintln!("Composited center: {:?}", p);

    assert!((p[0] - 0.08).abs() < 0.02, "R should be scene.r * ambient.r = 0.08, got {}", p[0]);
    assert!((p[1] - 0.06).abs() < 0.02, "G should be scene.g * ambient.g = 0.06, got {}", p[1]);
}

#[test]
fn composite_preserves_lit_areas() {
    let camera = Camera::new(20.0, 15.0);
    let scene_color = Color::new(0.8, 0.8, 0.8, 1.0);
    let ambient = Color::BLACK;

    // Bright white light at center
    let light = PointLight {
        position: Vec2::new(0.0, 0.0),
        color: Color::WHITE,
        intensity: 1.0,
        radius: 5.0,
        casts_shadows: false,
        shadow: ShadowSettings::default(),
    };

    let result = render_lit_scene(
        &camera, 100, 75, scene_color, ambient, &[light], &[], &[], None,
    );

    // Center should be bright (lightmap ≈ white, multiply preserves scene)
    let center = result.brightness(50, 37);
    // Far corner should be dark (lightmap ≈ black from ambient=BLACK, multiply → 0)
    let corner = result.brightness(0, 0);

    eprintln!("Center: {}, Corner: {}", center, corner);

    assert!(center > 0.3, "lit center should be bright, got {}", center);
    assert!(corner < 0.05, "unlit corner should be dark, got {}", corner);
}

#[test]
fn composite_shadow_creates_visible_dark_patch() {
    let camera = Camera::new(20.0, 15.0);
    let scene_color = Color::new(0.7, 0.7, 0.7, 1.0);
    let ambient = Color::new(0.05, 0.05, 0.05, 1.0);

    let light = PointLight {
        position: Vec2::new(0.0, 5.0),
        color: Color::WHITE,
        intensity: 1.0,
        radius: 12.0,
        casts_shadows: true,
        shadow: ShadowSettings::default(),
    };
    let occluders = vec![Occluder::from_aabb(0.0, 0.0, 1.5, 1.5)];

    let result = render_lit_scene(
        &camera, 200, 150, scene_color, ambient, &[light], &[], &occluders, None,
    );

    // Above box — lit
    let above = brightness_at_world(&result, &camera, 0.0, 3.5);
    // Below box — shadowed (only ambient)
    let below = brightness_at_world(&result, &camera, 0.0, -3.0);
    // Side — lit
    let side = brightness_at_world(&result, &camera, 5.0, -1.0);

    eprintln!("Above box: {}", above);
    eprintln!("Below box (shadow): {}", below);
    eprintln!("Side: {}", side);

    assert!(above > 0.2, "above box should be visibly lit, got {}", above);
    assert!(below < above * 0.3, "shadow should be much darker than lit area: shadow={}, lit={}", below, above);
    assert!(side > below * 2.0, "side (lit) should be brighter than shadow: side={}, shadow={}", side, below);
}

// ── Game scenario reproduction ──

#[test]
fn game_scenario_donut_light_with_ground_shadow() {
    // Reproduces the exact game setup from main_level.rs / shared.rs
    let camera = Camera::new(20.0, 15.0);
    let scene_color = Color::from_hex(0x1a1a2e);
    let ambient = Color::new(0.05, 0.05, 0.08, 1.0);

    // Donut point light
    let point_light = PointLight {
        position: Vec2::new(0.0, 3.0),
        color: Color::new(1.0, 0.9, 0.7, 1.0),
        intensity: 1.0,
        radius: 6.0,
        casts_shadows: true,
        shadow: ShadowSettings::soft(),
    };

    // Moonlight
    let dir_light = DirectionalLight::new(
        Vec2::new(0.3, -1.0),
        Color::new(0.2, 0.2, 0.35, 1.0),
        1.0,
    );

    // Ground platform
    let occluders = vec![
        Occluder::from_aabb(0.0, -5.5, 15.0, 1.0),
        Occluder::from_aabb(6.0, -3.0, 1.0, 1.0), // trigger box
    ];

    let result = render_lit_scene(
        &camera,
        200,
        150,
        scene_color,
        ambient,
        &[point_light],
        &[dir_light],
        &occluders,
        Some(-4.5),
    );

    // ── Key assertions ──

    // 1. Near the donut (light center) should be the brightest area
    let near_donut = brightness_at_world(&result, &camera, 0.0, 3.0);
    eprintln!("Near donut: {}", near_donut);
    assert!(near_donut > 0.05, "near donut should be brightly lit, got {}", near_donut);

    // 2. Above the ground but below the donut should still get some light
    let mid_scene = brightness_at_world(&result, &camera, 0.0, 0.0);
    eprintln!("Mid scene: {}", mid_scene);
    assert!(mid_scene > 0.01, "mid scene should have some light, got {}", mid_scene);

    // 3. Far from the donut light (outside radius) should be dim
    let far_right = brightness_at_world(&result, &camera, 8.0, 3.0);
    eprintln!("Far right: {}", far_right);
    assert!(
        far_right < near_donut * 0.5,
        "far right should be dimmer than near donut: far={}, near={}",
        far_right, near_donut
    );

    // 4. Ground shadow should block light below ground.
    //    Compare a point well within light radius (y=0, 3 units from light)
    //    to a point just below ground (y=-5, still within radius at 8 units... just outside).
    //    The key test: with ground shadow at -4.5, light shouldn't bleed below.
    let above_ground = brightness_at_world(&result, &camera, 0.0, -2.0);
    let at_ground = brightness_at_world(&result, &camera, 0.0, -4.5);
    eprintln!("Above ground (y=-2): {}, At ground (y=-4.5): {}", above_ground, at_ground);
    // above_ground should get both point light (within radius) and directional
    // at_ground is right at the ground shadow edge
    assert!(
        above_ground > at_ground,
        "above ground should be brighter than at ground level: above={}, at={}",
        above_ground, at_ground
    );

    // 5. The scene should have visible variation (not uniform)
    let brightnesses: Vec<f32> = [
        (0.0, 3.0), (0.0, 0.0), (5.0, 3.0), (-5.0, 0.0),
        (0.0, -3.0), (8.0, 0.0),
    ].iter()
        .map(|&(x, y)| brightness_at_world(&result, &camera, x, y))
        .collect();

    let min_b = brightnesses.iter().cloned().fold(f32::MAX, f32::min);
    let max_b = brightnesses.iter().cloned().fold(f32::MIN, f32::max);
    eprintln!("Brightness range: {:.4} to {:.4}", min_b, max_b);
    assert!(
        max_b - min_b > 0.02,
        "scene should have visible brightness variation, got range {:.4}",
        max_b - min_b
    );
}

#[test]
fn shadow_edge_is_sharp_without_pcf() {
    let camera = Camera::new(20.0, 15.0);
    let ambient = Color::BLACK;

    let light = PointLight {
        position: Vec2::new(0.0, 5.0),
        color: Color::WHITE,
        intensity: 1.0,
        radius: 15.0,
        casts_shadows: true,
        shadow: ShadowSettings::hard(),
    };
    let occluders = vec![Occluder::from_aabb(3.0, 0.0, 1.0, 1.0)];

    let lightmap = render_lightmap(&camera, 400, 300, ambient, &[light], &[], &occluders, None);

    // Scan horizontally at y=-2 (below the box). There should be an abrupt
    // transition from lit to dark at the shadow edge.
    let scan_y = -2.0;
    let mut transitions = 0;
    let mut prev_lit = false;
    for ix in -80..80 {
        let x = ix as f32 * 0.1;
        let b = brightness_at_world(&lightmap, &camera, x, scan_y);
        let lit = b > 0.05;
        if ix > -80 && lit != prev_lit {
            transitions += 1;
        }
        prev_lit = lit;
    }

    eprintln!("Shadow edge transitions at y={}: {}", scan_y, transitions);
    // There should be exactly 2 transitions (lit→dark, dark→lit) for a single box
    assert!(
        transitions == 2,
        "hard shadow should have exactly 2 transitions (enter/exit shadow), got {}",
        transitions
    );
}

#[test]
fn multiple_lights_accumulate_additively() {
    let camera = Camera::new(20.0, 15.0);
    let ambient = Color::BLACK;

    let light1 = PointLight {
        position: Vec2::new(-3.0, 0.0),
        color: Color::new(1.0, 0.0, 0.0, 1.0), // red
        intensity: 1.0,
        radius: 5.0,
        casts_shadows: false,
        shadow: ShadowSettings::default(),
    };
    let light2 = PointLight {
        position: Vec2::new(3.0, 0.0),
        color: Color::new(0.0, 0.0, 1.0, 1.0), // blue
        intensity: 1.0,
        radius: 5.0,
        casts_shadows: false,
        shadow: ShadowSettings::default(),
    };

    let lightmap = render_lightmap(
        &camera, 200, 150, ambient, &[light1, light2], &[], &[], None,
    );

    // At the red light center — should be mostly red
    let at_red = sample_at_world(&lightmap, &camera, -3.0, 0.0);
    // At the blue light center — should be mostly blue
    let at_blue = sample_at_world(&lightmap, &camera, 3.0, 0.0);
    // In between (x=0) — should have contributions from both
    let between = sample_at_world(&lightmap, &camera, 0.0, 0.0);

    eprintln!("At red light: {:?}", at_red);
    eprintln!("At blue light: {:?}", at_blue);
    eprintln!("Between: {:?}", between);

    assert!(at_red[0] > 0.5, "red center should have high R");
    assert!(at_red[2] < 0.1, "red center should have low B");
    assert!(at_blue[2] > 0.5, "blue center should have high B");
    assert!(at_blue[0] < 0.1, "blue center should have low R");
    assert!(between[0] > 0.01, "between should have some R from red light");
    assert!(between[2] > 0.01, "between should have some B from blue light");
}

#[test]
fn ground_shadow_prevents_light_bleed_below() {
    let camera = Camera::new(20.0, 15.0);
    let ambient = Color::BLACK;

    let light = PointLight {
        position: Vec2::new(0.0, 3.0),
        color: Color::WHITE,
        intensity: 1.0,
        radius: 12.0,
        casts_shadows: true,
        shadow: ShadowSettings::default(),
    };

    // Test WITH ground shadow
    let with_ground = render_lightmap(
        &camera, 200, 150, ambient, &[light.clone()], &[], &[], Some(-4.5),
    );
    let below_with = brightness_at_world(&with_ground, &camera, 0.0, -6.0);

    // Test WITHOUT ground shadow
    let without_ground = render_lightmap(
        &camera, 200, 150, ambient, &[light.clone()], &[], &[], None,
    );
    let below_without = brightness_at_world(&without_ground, &camera, 0.0, -6.0);

    eprintln!("Below ground WITH shadow: {}", below_with);
    eprintln!("Below ground WITHOUT shadow: {}", below_without);

    assert!(
        below_with < below_without,
        "ground shadow should reduce light below ground: with={}, without={}",
        below_with, below_without
    );
    assert!(below_with < 0.01, "with ground shadow, below should be dark");
}

#[test]
fn shadow_from_two_occluders_both_cast() {
    let camera = Camera::new(20.0, 15.0);
    let ambient = Color::BLACK;

    let light = PointLight {
        position: Vec2::new(0.0, 5.0),
        color: Color::WHITE,
        intensity: 1.0,
        radius: 15.0,
        casts_shadows: true,
        shadow: ShadowSettings::default(),
    };

    // Two boxes side by side
    let occluders = vec![
        Occluder::from_aabb(-3.0, 0.0, 1.0, 1.0),
        Occluder::from_aabb(3.0, 0.0, 1.0, 1.0),
    ];

    let lightmap = render_lightmap(
        &camera, 200, 150, ambient, &[light], &[], &occluders, None,
    );

    // Below left box — in shadow
    let below_left = brightness_at_world(&lightmap, &camera, -3.0, -3.0);
    // Below right box — in shadow
    let below_right = brightness_at_world(&lightmap, &camera, 3.0, -3.0);
    // Between the boxes at y=-3 — should be lit (not blocked by either)
    let between = brightness_at_world(&lightmap, &camera, 0.0, -3.0);

    eprintln!("Below left box: {}", below_left);
    eprintln!("Below right box: {}", below_right);
    eprintln!("Between boxes: {}", between);

    assert!(below_left < 0.01, "below left box should be shadowed");
    assert!(below_right < 0.01, "below right box should be shadowed");
    assert!(between > 0.05, "between boxes should be lit, got {}", between);
}

// ── Regression tests for known bugs ──

#[test]
fn shadow_not_applied_twice_through_alpha() {
    // Regression: if shadow multiplies both RGB and alpha, additive blending
    // applies shadow² instead of shadow¹. This test catches that by checking
    // that a half-shadowed pixel is exactly half the brightness of a fully-lit one.

    // We'll manually verify the math for a single pixel
    let light_color = [1.0f32, 0.9, 0.7, 1.0];
    let gradient_alpha = 0.8;
    let shadow = 0.5;

    // Correct shader: frag_color = vec4(light.rgb * shadow, light.a)
    // where light = (color.r, color.g, color.b, gradient_alpha * color.a)
    let correct_frag = [
        light_color[0] * shadow,
        light_color[1] * shadow,
        light_color[2] * shadow,
        gradient_alpha * light_color[3],
    ];
    let correct_contribution = additive_blend_contribution(correct_frag);

    // Bug shader: frag_color = light * shadow (multiplies alpha too)
    let bug_frag = [
        light_color[0] * shadow,
        light_color[1] * shadow,
        light_color[2] * shadow,
        gradient_alpha * light_color[3] * shadow, // BUG: shadow applied to alpha
    ];
    let bug_contribution = additive_blend_contribution(bug_frag);

    // Fully lit (shadow=1.0) for reference
    let full_frag = [
        light_color[0],
        light_color[1],
        light_color[2],
        gradient_alpha * light_color[3],
    ];
    let full_contribution = additive_blend_contribution(full_frag);

    eprintln!("Full contribution: {:?}", full_contribution);
    eprintln!("Correct half-shadow: {:?}", correct_contribution);
    eprintln!("Bug half-shadow: {:?}", bug_contribution);

    // Correct: contribution = full * shadow (linear)
    let correct_ratio = correct_contribution[0] / full_contribution[0];
    assert!(
        (correct_ratio - 0.5).abs() < 0.01,
        "correct shader should give 0.5x at shadow=0.5, got {}",
        correct_ratio
    );

    // Bug: contribution = full * shadow² (quadratic)
    let bug_ratio = bug_contribution[0] / full_contribution[0];
    assert!(
        (bug_ratio - 0.25).abs() < 0.01,
        "buggy shader would give 0.25x at shadow=0.5 (shadow²), got {}",
        bug_ratio
    );

    // The correct ratio should NOT equal the bug ratio
    assert!(
        (correct_ratio - bug_ratio).abs() > 0.1,
        "correct and bug paths should produce different results"
    );
}

/// Helper for the regression test above.
fn additive_blend_contribution(frag: [f32; 4]) -> [f32; 3] {
    [frag[0] * frag[3], frag[1] * frag[3], frag[2] * frag[3]]
}

#[test]
fn lightmap_composite_uv_not_flipped() {
    // Regression: if the lightmap composite pass flips V incorrectly,
    // the top of the scene gets the bottom of the lightmap.
    // We detect this by putting a light at the TOP of the scene and checking
    // that the composite result is bright at the TOP, not the bottom.
    let camera = Camera::new(20.0, 15.0);
    let scene_color = Color::new(0.5, 0.5, 0.5, 1.0);
    let ambient = Color::BLACK;

    let light = PointLight {
        position: Vec2::new(0.0, 6.0), // near top of camera view
        color: Color::WHITE,
        intensity: 1.0,
        radius: 3.0,
        casts_shadows: false,
        shadow: ShadowSettings::default(),
    };

    let result = render_lit_scene(
        &camera, 100, 75, scene_color, ambient, &[light], &[], &[], None,
    );

    // Top of scene (y=6) should be bright
    let top = brightness_at_world(&result, &camera, 0.0, 6.0);
    // Bottom of scene (y=-6) should be dark
    let bottom = brightness_at_world(&result, &camera, 0.0, -6.0);

    eprintln!("Top (near light): {}", top);
    eprintln!("Bottom (far from light): {}", bottom);

    assert!(top > bottom * 5.0, "light at top should make top bright, not bottom: top={}, bottom={}", top, bottom);
}

// ═══════════════════════════════════════════════════════════════════════════
// APPROACH B: PixelRenderer — exercises the REAL LightingSystem code
// ═══════════════════════════════════════════════════════════════════════════

/// A software Renderer that rasterizes into PixelBuffers.
///
/// When `LightingSystem::render_lightmap` calls bind_render_target, begin_frame,
/// draw(Mesh/Sprite/LitSprite), end_frame — this renderer performs actual
/// rasterization so we can inspect the resulting pixel values.
struct PixelRenderer {
    screen_w: f32,
    screen_h: f32,
    /// Screen buffer (target 0 = SCREEN)
    screen: PixelBuffer,
    /// Off-screen render targets (FBOs)
    targets: HashMap<u32, PixelBuffer>,
    /// Textures (gradient, FBO color attachments, etc.)
    textures: HashMap<u32, PixelBuffer>,
    /// Which target IDs correspond to which texture IDs
    target_textures: HashMap<u32, u32>,
    next_texture_id: u32,
    next_target_id: u32,
    /// Currently bound render target (0 = screen)
    current_target: u32,
    /// Current blend mode
    current_blend: BlendMode,
    /// Current camera (set by begin_frame)
    current_camera: Option<Camera>,
}

impl PixelRenderer {
    fn new(w: f32, h: f32) -> Self {
        Self {
            screen_w: w,
            screen_h: h,
            screen: PixelBuffer::new(w as u32, h as u32),
            targets: HashMap::new(),
            textures: HashMap::new(),
            target_textures: HashMap::new(),
            next_texture_id: 1,
            next_target_id: 1,
            current_target: 0,
            current_blend: BlendMode::Alpha,
            current_camera: None,
        }
    }

    /// Get the currently active pixel buffer (screen or FBO).
    fn active_buf(&self) -> &PixelBuffer {
        if self.current_target == 0 {
            &self.screen
        } else {
            &self.targets[&self.current_target]
        }
    }

    fn active_buf_mut(&mut self) -> &mut PixelBuffer {
        if self.current_target == 0 {
            &mut self.screen
        } else {
            self.targets.get_mut(&self.current_target).unwrap()
        }
    }

    /// Get the blend function for the current blend mode.
    fn blend_fn(&self) -> BlendFn {
        match self.current_blend {
            BlendMode::Alpha => blend_alpha,
            BlendMode::Additive => blend_additive,
            BlendMode::Multiply => blend_multiply,
        }
    }

    /// Convert world to pixel coords using the current camera.
    fn world_to_px(&self, wx: f32, wy: f32) -> [f32; 2] {
        let cam = self.current_camera.as_ref().unwrap();
        let buf = self.active_buf();
        world_to_pixel(cam, buf.width, buf.height, wx, wy)
    }

    /// Rasterize a quad (from world-space positions) into the active buffer.
    fn rasterize_world_quad(
        &mut self,
        positions: &[f32],   // 8 floats: 4 vertices × 2
        color: [f32; 4],
    ) {
        let v0 = self.world_to_px(positions[0], positions[1]);
        let v1 = self.world_to_px(positions[2], positions[3]);
        let v2 = self.world_to_px(positions[4], positions[5]);
        let v3 = self.world_to_px(positions[6], positions[7]);

        let blend = self.blend_fn();
        let buf = self.active_buf_mut();
        rasterize_triangle(buf, v0, v1, v2, color, blend);
        rasterize_triangle(buf, v0, v2, v3, color, blend);
    }

    /// Rasterize a mesh (triangles) into the active buffer.
    fn rasterize_mesh(
        &mut self,
        positions: &[f32],
        indices: &[u32],
        color: [f32; 4],
        vertex_colors: Option<&[f32]>,
    ) {
        let cam = self.current_camera.as_ref().unwrap().clone();
        let buf_w = self.active_buf().width;
        let buf_h = self.active_buf().height;

        for tri in indices.chunks(3) {
            if tri.len() < 3 {
                continue;
            }
            let i0 = tri[0] as usize;
            let i1 = tri[1] as usize;
            let i2 = tri[2] as usize;

            let v0 = world_to_pixel(&cam, buf_w, buf_h,
                positions[i0 * 2], positions[i0 * 2 + 1]);
            let v1 = world_to_pixel(&cam, buf_w, buf_h,
                positions[i1 * 2], positions[i1 * 2 + 1]);
            let v2 = world_to_pixel(&cam, buf_w, buf_h,
                positions[i2 * 2], positions[i2 * 2 + 1]);

            if let Some(vc) = vertex_colors {
                // Per-vertex colors: rasterize with barycentric interpolation
                let vc0 = [vc[i0*4], vc[i0*4+1], vc[i0*4+2], vc[i0*4+3]];
                let vc1 = [vc[i1*4], vc[i1*4+1], vc[i1*4+2], vc[i1*4+3]];
                let vc2 = [vc[i2*4], vc[i2*4+1], vc[i2*4+2], vc[i2*4+3]];

                let blend = self.blend_fn();
                let buf = self.active_buf_mut();
                rasterize_triangle_vc(buf, v0, v1, v2, color, vc0, vc1, vc2, blend);
            } else {
                let blend = self.blend_fn();
                let buf = self.active_buf_mut();
                rasterize_triangle(buf, v0, v1, v2, color, blend);
            }
        }
    }

    /// Draw a sprite: rasterize a textured quad.
    fn rasterize_sprite(
        &mut self,
        position: [f32; 2],
        size: [f32; 2],
        uv: [f32; 4],
        color: Color,
        texture: TextureId,
    ) {
        let hw = size[0] / 2.0;
        let hh = size[1] / 2.0;
        // World-space quad corners (same as make_quad with rotation=0)
        let world_bl = [position[0] - hw, position[1] - hh];
        let world_br = [position[0] + hw, position[1] - hh];
        let world_tr = [position[0] + hw, position[1] + hh];
        let world_tl = [position[0] - hw, position[1] + hh];

        let cam = self.current_camera.as_ref().unwrap().clone();
        let buf_w = self.active_buf().width;
        let buf_h = self.active_buf().height;

        let pbl = world_to_pixel(&cam, buf_w, buf_h, world_bl[0], world_bl[1]);
        let pbr = world_to_pixel(&cam, buf_w, buf_h, world_br[0], world_br[1]);
        let ptr = world_to_pixel(&cam, buf_w, buf_h, world_tr[0], world_tr[1]);
        let ptl = world_to_pixel(&cam, buf_w, buf_h, world_tl[0], world_tl[1]);

        // UV mapping: match make_quad convention
        // uv = [u0, v0, u1, v1]
        // make_quad assigns: BL=(u0,v1), BR=(u1,v1), TR=(u1,v0), TL=(u0,v0)
        let [u0, v0, u1, v1] = uv;

        let has_texture = texture.is_valid() && self.textures.contains_key(&texture.0);

        let min_px = pbl[0].min(pbr[0]).min(ptr[0]).min(ptl[0]).max(0.0) as u32;
        let max_px = (pbl[0].max(pbr[0]).max(ptr[0]).max(ptl[0]).ceil() as u32).min(buf_w - 1);
        let min_py = pbl[1].min(pbr[1]).min(ptr[1]).min(ptl[1]).max(0.0) as u32;
        let max_py = (pbl[1].max(pbr[1]).max(ptr[1]).max(ptl[1]).ceil() as u32).min(buf_h - 1);

        // Precompute inverse for bilinear interpolation
        // Since the quad is axis-aligned, we can use simple lerp
        let x_range = pbr[0] - pbl[0];
        let y_range = ptl[1] - pbl[1];

        let blend = self.blend_fn();

        for py in min_py..=max_py {
            for px in min_px..=max_px {
                let fx = px as f32 + 0.5;
                let fy = py as f32 + 0.5;

                // Check if inside quad
                if fx < pbl[0] || fx > pbr[0] || fy < pbl[1] || fy > ptr[1] {
                    continue;
                }

                // Interpolate UV
                let t_x = if x_range > 0.0 { (fx - pbl[0]) / x_range } else { 0.5 };
                let t_y = if y_range > 0.0 { (fy - pbl[1]) / y_range } else { 0.5 };

                // UV at this pixel (match make_quad):
                // BL=(u0,v1), BR=(u1,v1), TL=(u0,v0), TR=(u1,v0)
                let u = u0 + (u1 - u0) * t_x;
                let v = v1 + (v0 - v1) * t_y; // v1 at bottom, v0 at top

                let tex_color = if has_texture {
                    self.textures[&texture.0].sample_uv(u.clamp(0.0, 1.0), v.clamp(0.0, 1.0))
                } else {
                    [1.0, 1.0, 1.0, 1.0]
                };

                let src = [
                    tex_color[0] * color.r,
                    tex_color[1] * color.g,
                    tex_color[2] * color.b,
                    tex_color[3] * color.a,
                ];

                let buf = self.active_buf_mut();
                let dst = buf.get(px, py);
                buf.set(px, py, blend(src, dst));
            }
        }
    }

    /// Draw a LitSprite: gradient × shadow mask × PCF.
    fn rasterize_lit_sprite(
        &mut self,
        position: [f32; 2],
        size: [f32; 2],
        uv: [f32; 4],
        color: Color,
        texture: TextureId,
        shadow_mask: TextureId,
        screen_size: (f32, f32),
        _shadow_filter: u32,
        shadow_strength: f32,
    ) {
        let hw = size[0] / 2.0;
        let hh = size[1] / 2.0;

        let cam = self.current_camera.as_ref().unwrap().clone();
        let buf_w = self.active_buf().width;
        let buf_h = self.active_buf().height;

        let world_bl = [position[0] - hw, position[1] - hh];
        let world_tr = [position[0] + hw, position[1] + hh];
        let pbl = world_to_pixel(&cam, buf_w, buf_h, world_bl[0], world_bl[1]);
        let ptr = world_to_pixel(&cam, buf_w, buf_h, world_tr[0], world_tr[1]);

        let [u0, v0, u1, v1] = uv;

        let has_gradient = texture.is_valid() && self.textures.contains_key(&texture.0);
        let has_shadow = self.textures.contains_key(&shadow_mask.0);

        let min_px = pbl[0].max(0.0) as u32;
        let max_px = (ptr[0].ceil() as u32).min(buf_w - 1);
        let min_py = pbl[1].max(0.0) as u32;
        let max_py = (ptr[1].ceil() as u32).min(buf_h - 1);

        let x_range = ptr[0] - pbl[0];
        let y_range = ptr[1] - pbl[1];

        let blend = self.blend_fn();

        for py in min_py..=max_py {
            for px in min_px..=max_px {
                let fx = px as f32 + 0.5;
                let fy = py as f32 + 0.5;

                if fx < pbl[0] || fx > ptr[0] || fy < pbl[1] || fy > ptr[1] {
                    continue;
                }

                let t_x = if x_range > 0.0 { (fx - pbl[0]) / x_range } else { 0.5 };
                let t_y = if y_range > 0.0 { (fy - pbl[1]) / y_range } else { 0.5 };

                let u = u0 + (u1 - u0) * t_x;
                let v = v1 + (v0 - v1) * t_y;

                // Light shape from gradient texture (or solid white for directional)
                let light = if has_gradient {
                    let tex = self.textures[&texture.0].sample_uv(
                        u.clamp(0.0, 1.0), v.clamp(0.0, 1.0),
                    );
                    [
                        tex[0] * color.r,
                        tex[1] * color.g,
                        tex[2] * color.b,
                        tex[3] * color.a,
                    ]
                } else {
                    [color.r, color.g, color.b, color.a]
                };

                // Shadow mask: gl_FragCoord.xy / u_screen_size
                // In the real shader, gl_FragCoord has origin at bottom-left of FBO.
                // Our pixel buffer also has Y=0 at bottom, so this maps directly.
                let shadow = if has_shadow {
                    let su = (px as f32 + 0.5) / screen_size.0;
                    let sv = (py as f32 + 0.5) / screen_size.1;
                    let raw_shadow = self.textures[&shadow_mask.0].sample_uv(su, sv)[0];
                    // Apply shadow strength: mix(1.0, shadow, shadow_strength)
                    1.0 - shadow_strength * (1.0 - raw_shadow)
                } else {
                    1.0
                };

                // Lit shader: frag_color = vec4(light.rgb * shadow, light.a)
                let frag = [
                    light[0] * shadow,
                    light[1] * shadow,
                    light[2] * shadow,
                    light[3], // alpha NOT multiplied by shadow
                ];

                let buf = self.active_buf_mut();
                let dst = buf.get(px, py);
                buf.set(px, py, blend(frag, dst));
            }
        }
    }

    /// Get a snapshot of a render target's texture as a PixelBuffer.
    fn get_target_texture(&self, target_id: u32) -> Option<&PixelBuffer> {
        let tex_id = self.target_textures.get(&target_id)?;
        self.textures.get(tex_id)
    }

    /// Get the screen buffer.
    fn screen_buf(&self) -> &PixelBuffer {
        &self.screen
    }
}

impl Renderer for PixelRenderer {
    type Error = String;

    fn init(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn begin_frame(&mut self, camera: &Camera) {
        self.current_camera = Some(camera.clone());
    }

    fn clear(&mut self, color: Color) {
        let c = [color.r, color.g, color.b, color.a];
        self.active_buf_mut().clear(c);
    }

    fn draw(&mut self, command: RenderCommand) {
        match command {
            RenderCommand::Mesh(mesh) => {
                let c = [mesh.color.r, mesh.color.g, mesh.color.b, mesh.color.a];
                self.rasterize_mesh(&mesh.positions, &mesh.indices, c, mesh.vertex_colors.as_deref());
            }
            RenderCommand::Sprite(s) => {
                self.rasterize_sprite(s.position, s.size, s.uv, s.color, s.texture);
            }
            RenderCommand::LitSprite(l) => {
                self.rasterize_lit_sprite(
                    l.position, l.size, l.uv, l.color,
                    l.texture, l.shadow_mask, l.screen_size, l.shadow_filter,
                    l.shadow_strength,
                );
            }
            _ => {} // Ignore lines, rects, terrain for now
        }
    }

    fn end_frame(&mut self) {
        // Sync FBO contents to its texture so subsequent reads see the rendered data
        if self.current_target != 0 {
            if let Some(&tex_id) = self.target_textures.get(&self.current_target) {
                if let Some(buf) = self.targets.get(&self.current_target) {
                    self.textures.insert(tex_id, buf.clone());
                }
            }
        }
    }

    fn create_texture(&mut self, desc: &TextureDescriptor) -> Result<TextureId, String> {
        let id = self.next_texture_id;
        self.next_texture_id += 1;
        // Store texture data as PixelBuffer
        let mut buf = PixelBuffer::new(desc.width, desc.height);
        let bpp = desc.format.bytes_per_pixel();
        for y in 0..desc.height {
            for x in 0..desc.width {
                let idx = ((y * desc.width + x) * bpp as u32) as usize;
                let pixel = match bpp {
                    4 => [
                        desc.data[idx] as f32 / 255.0,
                        desc.data[idx + 1] as f32 / 255.0,
                        desc.data[idx + 2] as f32 / 255.0,
                        desc.data[idx + 3] as f32 / 255.0,
                    ],
                    3 => [
                        desc.data[idx] as f32 / 255.0,
                        desc.data[idx + 1] as f32 / 255.0,
                        desc.data[idx + 2] as f32 / 255.0,
                        1.0,
                    ],
                    1 => {
                        let v = desc.data[idx] as f32 / 255.0;
                        [v, v, v, 1.0]
                    }
                    _ => [1.0, 1.0, 1.0, 1.0],
                };
                buf.set(x, y, pixel);
            }
        }
        self.textures.insert(id, buf);
        Ok(TextureId(id))
    }

    fn destroy_texture(&mut self, id: TextureId) {
        self.textures.remove(&id.0);
    }

    fn screen_size(&self) -> (f32, f32) {
        (self.screen_w, self.screen_h)
    }

    fn set_screen_size(&mut self, width: f32, height: f32) {
        self.screen_w = width;
        self.screen_h = height;
    }

    fn set_blend_mode(&mut self, mode: BlendMode) {
        self.current_blend = mode;
    }

    fn create_render_target(&mut self, w: u32, h: u32) -> Result<(RenderTargetId, TextureId), String> {
        let rt_id = self.next_target_id;
        self.next_target_id += 1;
        let tex_id = self.next_texture_id;
        self.next_texture_id += 1;

        self.targets.insert(rt_id, PixelBuffer::new(w, h));
        self.textures.insert(tex_id, PixelBuffer::new(w, h));
        self.target_textures.insert(rt_id, tex_id);

        Ok((RenderTargetId(rt_id), TextureId(tex_id)))
    }

    fn bind_render_target(&mut self, target: RenderTargetId) {
        self.current_target = target.0;
    }

    fn destroy_render_target(&mut self, target: RenderTargetId) {
        self.targets.remove(&target.0);
        self.target_textures.remove(&target.0);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// APPROACH B TESTS — exercise real LightingSystem through PixelRenderer
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn real_lightmap_point_light_no_shadow_is_radial() {
    // Exercise the real LightingSystem with a non-shadow point light.
    // The lightmap should show a radial gradient centered on the light.
    let mut sys = LightingSystem::new();
    sys.set_ambient(Color::BLACK);
    sys.set_enabled(true);
    sys.add_light(PointLight {
        position: Vec2::new(0.0, 0.0),
        color: Color::WHITE,
        intensity: 1.0,
        radius: 5.0,
        casts_shadows: false,
        shadow: ShadowSettings::default(),
    });

    let mut renderer = PixelRenderer::new(200.0, 150.0);
    let camera = Camera::new(20.0, 15.0);

    sys.ensure_resources(&mut renderer);
    sys.render_lightmap(&mut renderer, &camera);

    // Get the lightmap texture
    let lightmap_target_id = 1; // first render target created
    let lightmap_tex = renderer.get_target_texture(lightmap_target_id)
        .expect("lightmap should exist");

    // Center should be bright, edges dim
    let center = lightmap_tex.brightness(100, 75);
    let edge = lightmap_tex.brightness(0, 0);

    eprintln!("[REAL] Center: {}, Edge: {}", center, edge);
    assert!(center > 0.3, "lightmap center should be bright, got {}", center);
    assert!(edge < 0.05, "lightmap edge should be dim (ambient=black), got {}", edge);
}

#[test]
fn real_lightmap_shadow_mask_coverage() {
    // THE KEY TEST: Exercise the real LightingSystem with the game's occluders
    // and check what fraction of the shadow mask is black.
    // If the shadow mask is almost entirely black, the point light looks like
    // a tiny cone (only the unshadowed wedge is visible).

    let mut sys = LightingSystem::new();
    sys.set_ambient(Color::new(0.05, 0.05, 0.08, 1.0));
    sys.set_enabled(true);
    sys.set_ground_shadow(Some(-4.5));

    sys.add_light(PointLight {
        position: Vec2::new(0.0, 3.0),
        color: Color::new(1.0, 0.9, 0.7, 1.0),
        intensity: 1.0,
        radius: 6.0,
        casts_shadows: true,
        shadow: ShadowSettings::soft(),
    });

    // Game occluders: ground platform + trigger box
    sys.set_occluders(vec![
        Occluder::from_aabb(0.0, -5.5, 15.0, 1.0),  // 30×2 ground platform
        Occluder::from_aabb(6.0, -3.0, 1.0, 1.0),    // trigger box
    ]);

    let mut renderer = PixelRenderer::new(200.0, 150.0);
    let camera = Camera::new(20.0, 15.0);

    sys.ensure_resources(&mut renderer);
    sys.render_lightmap(&mut renderer, &camera);

    // The shadow mask is the second render target created (after lightmap)
    let shadow_mask_target_id = 2;
    let shadow_mask = renderer.get_target_texture(shadow_mask_target_id)
        .expect("shadow mask should exist");

    // Count black vs white pixels in the shadow mask
    let total = (shadow_mask.width * shadow_mask.height) as f32;
    let mut black_count = 0u32;
    let mut white_count = 0u32;
    for py in 0..shadow_mask.height {
        for px in 0..shadow_mask.width {
            let b = shadow_mask.brightness(px, py);
            if b < 0.1 {
                black_count += 1;
            } else if b > 0.9 {
                white_count += 1;
            }
        }
    }

    let black_pct = black_count as f32 / total * 100.0;
    let white_pct = white_count as f32 / total * 100.0;

    eprintln!("[REAL] Shadow mask: {:.1}% black, {:.1}% white", black_pct, white_pct);
    eprintln!("[REAL] Black pixels: {}, White pixels: {}, Total: {}", black_count, white_count, total as u32);

    // CRITICAL: If the shadow mask is too black, the point light will look
    // like a tiny cone because most of it is being shadow-masked away.
    // The ground shadow should only block the area BELOW the ground,
    // not the entire viewport. With correct projection, the game scene
    // produces ~22% black. With broken projection (shadow quads collapsing
    // inward), it jumps to ~47%.
    assert!(
        black_pct < 35.0,
        "shadow mask is {:.1}% black — too much! The point light will look \
         like a tiny cone because shadows are covering most of the viewport. \
         Ground/platform shadows should only affect the area below them, \
         not collapse inward toward the light.",
        black_pct
    );

    // There should be a substantial lit area (white) above the ground
    assert!(
        white_pct > 50.0,
        "shadow mask has only {:.1}% white — most of the light is being blocked",
        white_pct
    );

    // Now check the lightmap itself
    let lightmap_target_id = 1;
    let lightmap = renderer.get_target_texture(lightmap_target_id)
        .expect("lightmap should exist");

    // Sample the lightmap at key positions
    let near_light = brightness_at_world(lightmap, &camera, 0.0, 3.0);
    let below_ground = brightness_at_world(lightmap, &camera, 0.0, -6.0);
    let side = brightness_at_world(lightmap, &camera, 3.0, 1.0);

    eprintln!("[REAL] Lightmap near light: {}", near_light);
    eprintln!("[REAL] Lightmap below ground: {}", below_ground);
    eprintln!("[REAL] Lightmap to the side: {}", side);

    assert!(
        near_light > 0.1,
        "lightmap should be bright near the light, got {}",
        near_light
    );
    assert!(
        near_light > below_ground * 2.0,
        "near light should be brighter than below ground: near={}, below={}",
        near_light, below_ground
    );
}

#[test]
fn real_composite_game_scene() {
    // Full pipeline: scene → lightmap → composite, using real LightingSystem.
    let mut sys = LightingSystem::new();
    sys.set_ambient(Color::new(0.05, 0.05, 0.08, 1.0));
    sys.set_enabled(true);
    sys.set_ground_shadow(Some(-4.5));

    sys.add_light(PointLight {
        position: Vec2::new(0.0, 3.0),
        color: Color::new(1.0, 0.9, 0.7, 1.0),
        intensity: 1.0,
        radius: 6.0,
        casts_shadows: true,
        shadow: ShadowSettings::soft(),
    });

    sys.add_directional_light(DirectionalLight::new(
        Vec2::new(0.3, -1.0),
        Color::new(0.2, 0.2, 0.35, 1.0),
        1.0,
    ));

    sys.set_occluders(vec![
        Occluder::from_aabb(0.0, -5.5, 15.0, 1.0),
        Occluder::from_aabb(6.0, -3.0, 1.0, 1.0),
    ]);

    let mut renderer = PixelRenderer::new(200.0, 150.0);
    let camera = Camera::new(20.0, 15.0);

    // 1. Render scene
    renderer.bind_render_target(RenderTargetId::SCREEN);
    renderer.begin_frame(&camera);
    renderer.clear(Color::from_hex(0x1a1a2e));
    renderer.end_frame();

    // 2. Render lightmap (real code!)
    sys.ensure_resources(&mut renderer);
    sys.render_lightmap(&mut renderer, &camera);

    // 3. Composite (real code!)
    renderer.bind_render_target(RenderTargetId::SCREEN);
    sys.composite_lightmap(&mut renderer, &camera);

    // Inspect the final screen
    let screen = renderer.screen_buf();

    let near_light = brightness_at_world(screen, &camera, 0.0, 3.0);
    let mid = brightness_at_world(screen, &camera, 0.0, 0.0);
    let far = brightness_at_world(screen, &camera, 8.0, 3.0);
    let below = brightness_at_world(screen, &camera, 0.0, -6.0);

    eprintln!("[REAL COMPOSITE] Near light: {}", near_light);
    eprintln!("[REAL COMPOSITE] Mid: {}", mid);
    eprintln!("[REAL COMPOSITE] Far: {}", far);
    eprintln!("[REAL COMPOSITE] Below ground: {}", below);

    // Near the light should be the brightest
    assert!(
        near_light > far,
        "near light should be brighter than far: near={}, far={}",
        near_light, far
    );

    // Scene should not be uniformly dark (i.e., lighting works)
    assert!(
        near_light > 0.02,
        "scene near light should not be pitch black, got {}",
        near_light
    );
}

#[test]
fn real_shadow_mask_not_inverted_by_ground_platform() {
    // Specific regression: the 30×2 ground platform at (0,-5.5) has side edges
    // at x=±15. If shadow projection is broken, these edges project shadow quads
    // that cover the ENTIRE viewport, making the shadow mask almost all black.
    // The point light then looks like a "cone" — only a tiny unshadowed wedge.

    let mut sys = LightingSystem::new();
    sys.set_ambient(Color::BLACK);
    sys.set_enabled(true);

    sys.add_light(PointLight {
        position: Vec2::new(0.0, 3.0),
        color: Color::WHITE,
        intensity: 1.0,
        radius: 6.0,
        casts_shadows: true,
        shadow: ShadowSettings::default(),
    });

    // ONLY the ground platform — no trigger box, no ground shadow plane
    sys.set_occluders(vec![
        Occluder::from_aabb(0.0, -5.5, 15.0, 1.0),
    ]);

    let mut renderer = PixelRenderer::new(200.0, 150.0);
    let camera = Camera::new(20.0, 15.0);

    sys.ensure_resources(&mut renderer);
    sys.render_lightmap(&mut renderer, &camera);

    // Check shadow mask
    let shadow_mask = renderer.get_target_texture(2)
        .expect("shadow mask should exist");

    // The area ABOVE the ground platform (top half of viewport) should be
    // predominantly white (lit). Only the area below the platform should be black.
    let buf_w = shadow_mask.width;
    let buf_h = shadow_mask.height;
    let midpoint_y = buf_h / 2; // roughly y=0 in world space

    // Count white pixels in the top half (y > midpoint)
    let mut top_white = 0u32;
    let mut top_total = 0u32;
    for py in midpoint_y..buf_h {
        for px in 0..buf_w {
            top_total += 1;
            if shadow_mask.brightness(px, py) > 0.5 {
                top_white += 1;
            }
        }
    }

    let top_white_pct = top_white as f32 / top_total as f32 * 100.0;
    eprintln!(
        "[REAL] Top half of shadow mask: {:.1}% white ({}/{})",
        top_white_pct, top_white, top_total
    );

    // The top half should be mostly white — the ground platform is BELOW
    // the viewport center, so it shouldn't cast shadows into the top half.
    assert!(
        top_white_pct > 70.0,
        "top half of shadow mask should be mostly lit (white), got {:.1}% white. \
         Ground platform shadows are incorrectly covering the area above the platform.",
        top_white_pct
    );
}

#[test]
fn real_point_light_is_radial_not_cone() {
    // REGRESSION: The point light appeared as a directional cone instead of
    // a radial glow. Root cause: project_from_point used light_radius as a
    // fixed projection distance. For occluder edges farther from the light
    // than light_radius, the projected shadow quads COLLAPSED INWARD toward
    // the light instead of extending outward.
    //
    // Example: ground platform bottom edge at y=-6.5 is ~17.8 units from
    // the donut light at y=3. With radius=6, the projected endpoints land
    // at y≈-0.2 (ABOVE the edge, between the edge and light). The resulting
    // shadow quad is an inverted trapezoid that covers the center of the
    // viewport — right where the point light should be. The light only
    // escapes through narrow gaps between collapsed quads → cone shape.

    let mut sys = LightingSystem::new();
    sys.set_ambient(Color::BLACK);
    sys.set_enabled(true);
    sys.set_ground_shadow(Some(-4.5));

    sys.add_light(PointLight {
        position: Vec2::new(0.0, 3.0),
        color: Color::WHITE,
        intensity: 1.0,
        radius: 6.0,
        casts_shadows: true,
        shadow: ShadowSettings::default(),
    });

    sys.set_occluders(vec![
        Occluder::from_aabb(0.0, -5.5, 15.0, 1.0),  // 30×2 ground platform
        Occluder::from_aabb(6.0, -3.0, 1.0, 1.0),    // trigger box
    ]);

    let mut renderer = PixelRenderer::new(200.0, 150.0);
    let camera = Camera::new(20.0, 15.0);

    sys.ensure_resources(&mut renderer);
    sys.render_lightmap(&mut renderer, &camera);

    // Get the lightmap
    let lightmap = renderer.get_target_texture(1).expect("lightmap");

    // The point light is at (0, 3) with radius 6, so it covers world-space
    // region (-6, -3) to (6, 9). Sample brightness at 8 compass directions
    // from the light center, all at the same distance (3 units = half radius).
    let light_pos = (0.0f32, 3.0f32);
    let sample_dist = 3.0;
    let directions = [
        ("right",       ( 1.0,  0.0)),
        ("upper-right", ( 0.7,  0.7)),
        ("up",          ( 0.0,  1.0)),
        ("upper-left",  (-0.7,  0.7)),
        ("left",        (-1.0,  0.0)),
        ("lower-left",  (-0.7, -0.7)),
        ("down",        ( 0.0, -1.0)),
        ("lower-right", ( 0.7, -0.7)),
    ];

    let mut brightnesses = Vec::new();
    for (name, (dx, dy)) in &directions {
        let wx = light_pos.0 + dx * sample_dist;
        let wy = light_pos.1 + dy * sample_dist;
        let b = brightness_at_world(lightmap, &camera, wx, wy);
        eprintln!("[RADIAL] {} ({:.1}, {:.1}): brightness = {:.4}", name, wx, wy, b);
        brightnesses.push((*name, b));
    }

    // For a radial point light, all 8 samples at equal distance should have
    // similar brightness. The "cone" bug manifests as a few directions being
    // bright and the rest being near-zero.
    let min_b = brightnesses.iter().map(|(_, b)| *b).fold(f32::MAX, f32::min);
    let max_b = brightnesses.iter().map(|(_, b)| *b).fold(f32::MIN, f32::max);
    let avg_b = brightnesses.iter().map(|(_, b)| *b).sum::<f32>() / brightnesses.len() as f32;

    eprintln!("[RADIAL] min={:.4}, max={:.4}, avg={:.4}, ratio={:.2}",
        min_b, max_b, avg_b, if max_b > 0.0 { min_b / max_b } else { 0.0 });

    // The dimmest direction should be at least 30% as bright as the brightest.
    // With the cone bug, the dimmest direction is 0 (fully shadowed) while
    // the brightest is ~0.75 (the unshadowed wedge).
    assert!(
        max_b > 0.1,
        "light should be visible at half-radius distance, got max brightness {}",
        max_b
    );
    assert!(
        min_b / max_b > 0.3,
        "point light is not radial! Dimmest direction ({}) has brightness {:.4} but \
         brightest ({}) has {:.4} — ratio {:.2}. This looks like a cone, not a radial light. \
         Shadow quads are likely collapsing inward and masking most of the light.",
        brightnesses.iter().min_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).unwrap().0,
        min_b,
        brightnesses.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).unwrap().0,
        max_b,
        min_b / max_b,
    );

    // Additionally check that at least 6 of 8 directions are meaningfully lit
    let lit_count = brightnesses.iter().filter(|(_, b)| *b > 0.05).count();
    assert!(
        lit_count >= 6,
        "only {}/8 directions are lit — point light looks like a cone, not radial",
        lit_count
    );
}

#[test]
fn shadow_quads_do_not_collapse_inward() {
    // Direct test: verify that projected shadow endpoints are FARTHER from
    // the light than the original occluder endpoints. If they're closer,
    // the shadow quad has collapsed inward (the root cause of the cone bug).
    use unison_lighting::shadow::project_point_shadows;

    let light_pos = [0.0f32, 3.0];
    let light_radius = 6.0;

    // Ground platform: edges are 15+ units from the light (farther than radius 6)
    let occluders = vec![
        Occluder::from_aabb(0.0, -5.5, 15.0, 1.0),
    ];

    let quads = project_point_shadows(light_pos, light_radius, &occluders, 0.0, 1.0);

    for (i, quad) in quads.iter().enumerate() {
        // Original endpoints are positions[0..4], projected are positions[4..8]
        let orig_a = [quad.positions[0], quad.positions[1]];
        let orig_b = [quad.positions[2], quad.positions[3]];
        let proj_b = [quad.positions[4], quad.positions[5]];
        let proj_a = [quad.positions[6], quad.positions[7]];

        let dist_a = ((orig_a[0] - light_pos[0]).powi(2) + (orig_a[1] - light_pos[1]).powi(2)).sqrt();
        let dist_a_proj = ((proj_a[0] - light_pos[0]).powi(2) + (proj_a[1] - light_pos[1]).powi(2)).sqrt();
        let dist_b = ((orig_b[0] - light_pos[0]).powi(2) + (orig_b[1] - light_pos[1]).powi(2)).sqrt();
        let dist_b_proj = ((proj_b[0] - light_pos[0]).powi(2) + (proj_b[1] - light_pos[1]).powi(2)).sqrt();

        eprintln!(
            "[QUAD {}] A: dist {:.1} → proj dist {:.1} | B: dist {:.1} → proj dist {:.1}",
            i, dist_a, dist_a_proj, dist_b, dist_b_proj
        );

        // Projected endpoints must be farther from the light than originals.
        // If they're closer, the shadow quad has collapsed inward.
        assert!(
            dist_a_proj > dist_a,
            "quad {} endpoint A collapsed inward: original dist {:.1}, projected dist {:.1}. \
             Shadow will cover area BETWEEN light and occluder instead of BEYOND occluder.",
            i, dist_a, dist_a_proj
        );
        assert!(
            dist_b_proj > dist_b,
            "quad {} endpoint B collapsed inward: original dist {:.1}, projected dist {:.1}. \
             Shadow will cover area BETWEEN light and occluder instead of BEYOND occluder.",
            i, dist_b, dist_b_proj
        );
    }
}
