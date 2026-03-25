//! Metal Shading Language (MSL) shader sources for the iOS renderer.
//!
//! Ported from the GLSL shaders in `unison-web/src/shaders.rs`.
//! Each shader pair (vertex + fragment) handles one category of render commands.

/// Shared vertex types used by all shaders.
///
/// These must match the vertex buffer layout in `MetalRenderer::write_quad`
/// and `MetalRenderer::write_mesh`.
pub const SHADER_TYPES: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Vertex {
    float2 position [[attribute(0)]];
    float2 uv       [[attribute(1)]];
    float4 color    [[attribute(2)]];
};

struct Uniforms {
    float3x3 view_projection;
};

struct FragmentUniforms {
    float4 color;
    int use_texture;   // i32 in Rust — avoids bool alignment ambiguity
    int _pad0;
    int _pad1;
    int _pad2;
};

struct LitFragmentUniforms {
    float4 color;
    int use_texture;   // i32 in Rust
    int _pad0;
    float2 screen_size;
    int shadow_filter;
    float shadow_strength;
    float2 _pad1;
};

struct VertexOut {
    float4 position [[position]];
    float2 uv;
    float4 vertex_color;
};
"#;

/// Vertex shader — transforms 2D positions by a 3×3 view-projection matrix.
///
/// Used by all render command types (sprites, meshes, lines, rects, terrain).
pub const VERTEX_SHADER: &str = r#"
vertex VertexOut vertex_main(
    Vertex in [[stage_in]],
    constant Uniforms& uniforms [[buffer(1)]]
) {
    float3 pos = uniforms.view_projection * float3(in.position, 1.0);
    VertexOut out;
    out.position = float4(pos.xy, 0.0, 1.0);
    out.uv = in.uv;
    out.vertex_color = in.color;
    return out;
}
"#;

/// Fragment shader — solid color with optional texture sampling.
///
/// Handles sprites, meshes, rects, lines, and terrain. When `use_texture`
/// is true, samples the texture and multiplies by color and per-vertex color.
pub const FRAGMENT_SHADER: &str = r#"
fragment float4 fragment_main(
    VertexOut in [[stage_in]],
    constant FragmentUniforms& uniforms [[buffer(0)]],
    texture2d<float> tex [[texture(0)]],
    sampler tex_sampler [[sampler(0)]]
) {
    if (uniforms.use_texture != 0) {
        float4 tex_color = tex.sample(tex_sampler, in.uv);
        return tex_color * uniforms.color * in.vertex_color;
    } else {
        return uniforms.color * in.vertex_color;
    }
}
"#;

/// Lit sprite fragment shader — samples both a light gradient texture and
/// a shadow mask, with optional PCF filtering for soft shadow edges.
///
/// Used by the lighting system for point lights and directional lights.
pub const LIT_FRAGMENT_SHADER: &str = r#"
float sample_shadow(texture2d<float> shadow_mask, sampler s, float2 uv) {
    return shadow_mask.sample(s, uv).r;
}

float sample_pcf5(texture2d<float> shadow_mask, sampler s, float2 uv, float2 ts) {
    return (sample_shadow(shadow_mask, s, uv) +
            sample_shadow(shadow_mask, s, uv + float2(-ts.x, 0.0)) +
            sample_shadow(shadow_mask, s, uv + float2( ts.x, 0.0)) +
            sample_shadow(shadow_mask, s, uv + float2(0.0, -ts.y)) +
            sample_shadow(shadow_mask, s, uv + float2(0.0,  ts.y))) / 5.0;
}

float sample_pcf13(texture2d<float> shadow_mask, sampler s, float2 uv, float2 ts) {
    float sum = 0.0;
    for (float x = -1.0; x <= 1.0; x += 1.0) {
        for (float y = -1.0; y <= 1.0; y += 1.0) {
            sum += sample_shadow(shadow_mask, s, uv + float2(x, y) * ts);
        }
    }
    sum += sample_shadow(shadow_mask, s, uv + float2(-2.0, 0.0) * ts);
    sum += sample_shadow(shadow_mask, s, uv + float2( 2.0, 0.0) * ts);
    sum += sample_shadow(shadow_mask, s, uv + float2(0.0, -2.0) * ts);
    sum += sample_shadow(shadow_mask, s, uv + float2(0.0,  2.0) * ts);
    return sum / 13.0;
}

fragment float4 lit_fragment_main(
    VertexOut in [[stage_in]],
    constant LitFragmentUniforms& uniforms [[buffer(0)]],
    texture2d<float> tex [[texture(0)]],
    texture2d<float> shadow_mask [[texture(1)]],
    sampler tex_sampler [[sampler(0)]]
) {
    // Light shape
    float4 light;
    if (uniforms.use_texture != 0) {
        light = tex.sample(tex_sampler, in.uv) * uniforms.color;
    } else {
        light = uniforms.color;
    }

    // Map fragment position to shadow mask UV
    float2 shadow_uv = in.position.xy / uniforms.screen_size;

    float2 ts = 1.0 / uniforms.screen_size;
    float shadow;
    if (uniforms.shadow_filter == 13) {
        shadow = sample_pcf13(shadow_mask, tex_sampler, shadow_uv, ts);
    } else if (uniforms.shadow_filter == 5) {
        shadow = sample_pcf5(shadow_mask, tex_sampler, shadow_uv, ts);
    } else {
        shadow = sample_shadow(shadow_mask, tex_sampler, shadow_uv);
    }

    // Apply shadow strength
    shadow = mix(1.0, shadow, uniforms.shadow_strength);

    return float4(light.rgb * shadow, light.a);
}
"#;

/// Returns the complete MSL source by concatenating types + vertex + base fragment.
pub fn base_shader_source() -> String {
    format!("{}\n{}\n{}", SHADER_TYPES, VERTEX_SHADER, FRAGMENT_SHADER)
}

/// Returns the complete MSL source for the lit sprite shader.
pub fn lit_shader_source() -> String {
    format!("{}\n{}\n{}", SHADER_TYPES, VERTEX_SHADER, LIT_FRAGMENT_SHADER)
}
