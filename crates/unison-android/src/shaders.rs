//! GLSL ES 3.0 shader sources for Android (OpenGL ES 3.0).
//!
//! These are identical to the WebGL2 shaders — both use GLSL ES 3.0.

/// Vertex shader — transforms positions by view-projection matrix
pub const VERTEX_SHADER: &str = r#"#version 300 es
precision mediump float;

layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_uv;
layout(location = 2) in vec4 a_vertex_color;

uniform mat3 u_view_projection;

out vec2 v_uv;
out vec4 v_vertex_color;

void main() {
    vec3 pos = u_view_projection * vec3(a_position, 1.0);
    gl_Position = vec4(pos.xy, 0.0, 1.0);
    v_uv = a_uv;
    v_vertex_color = a_vertex_color;
}
"#;

/// Fragment shader — solid color with optional texture
pub const FRAGMENT_SHADER: &str = r#"#version 300 es
precision mediump float;

in vec2 v_uv;
in vec4 v_vertex_color;

uniform vec4 u_color;
uniform bool u_use_texture;
uniform sampler2D u_texture;

out vec4 frag_color;

void main() {
    if (u_use_texture) {
        vec4 tex = texture(u_texture, v_uv);
        frag_color = tex * u_color * v_vertex_color;
    } else {
        frag_color = u_color * v_vertex_color;
    }
}
"#;

/// Fragment shader for lit sprites — samples both a light gradient texture
/// and a shadow mask, with optional PCF filtering for soft shadow edges.
///
/// Uniforms:
/// - `u_color`: light color * intensity
/// - `u_texture`: light shape texture (radial gradient for point lights)
/// - `u_shadow_mask`: shadow mask FBO texture (white=lit, black=shadow)
/// - `u_screen_size`: viewport dimensions for gl_FragCoord -> shadow UV
/// - `u_shadow_filter`: PCF mode (0=none, 5=PCF5, 13=PCF13)
pub const LIT_FRAGMENT_SHADER: &str = r#"#version 300 es
precision mediump float;

in vec2 v_uv;
in vec4 v_vertex_color;

uniform vec4 u_color;
uniform bool u_use_texture;
uniform sampler2D u_texture;
uniform sampler2D u_shadow_mask;
uniform vec2 u_screen_size;
uniform int u_shadow_filter;
uniform float u_shadow_strength;

out vec4 frag_color;

float sample_shadow(vec2 uv) {
    return texture(u_shadow_mask, uv).r;
}

float sample_pcf5(vec2 uv, vec2 ts) {
    return (sample_shadow(uv) +
            sample_shadow(uv + vec2(-ts.x, 0.0)) +
            sample_shadow(uv + vec2( ts.x, 0.0)) +
            sample_shadow(uv + vec2(0.0, -ts.y)) +
            sample_shadow(uv + vec2(0.0,  ts.y))) / 5.0;
}

float sample_pcf13(vec2 uv, vec2 ts) {
    float sum = 0.0;
    for (float x = -1.0; x <= 1.0; x += 1.0) {
        for (float y = -1.0; y <= 1.0; y += 1.0) {
            sum += sample_shadow(uv + vec2(x, y) * ts);
        }
    }
    sum += sample_shadow(uv + vec2(-2.0, 0.0) * ts);
    sum += sample_shadow(uv + vec2( 2.0, 0.0) * ts);
    sum += sample_shadow(uv + vec2(0.0, -2.0) * ts);
    sum += sample_shadow(uv + vec2(0.0,  2.0) * ts);
    return sum / 13.0;
}

void main() {
    // Light shape: use gradient texture for point lights, solid color for directional
    vec4 light;
    if (u_use_texture) {
        light = texture(u_texture, v_uv) * u_color;
    } else {
        light = u_color;
    }

    // Map fragment position to shadow mask UV.
    // Both the lightmap (where this shader runs) and the shadow mask are FBOs,
    // so gl_FragCoord and texture coordinates share the same orientation — no flip needed.
    vec2 shadow_uv = gl_FragCoord.xy / u_screen_size;

    vec2 ts = 1.0 / u_screen_size;
    float shadow;
    if (u_shadow_filter == 13) {
        shadow = sample_pcf13(shadow_uv, ts);
    } else if (u_shadow_filter == 5) {
        shadow = sample_pcf5(shadow_uv, ts);
    } else {
        shadow = sample_shadow(shadow_uv);
    }

    // Apply shadow strength: clamp how dark shadows can be.
    // At shadow_strength=1.0, shadow passes through unchanged.
    // At shadow_strength=0.0, shadow is always 1.0 (no darkening).
    shadow = mix(1.0, shadow, u_shadow_strength);

    // Apply shadow to RGB only — not alpha. With additive blending (SRC_ALPHA, ONE),
    // alpha scales the contribution. If we multiplied shadow into alpha too,
    // the effective attenuation would be shadow^2 instead of shadow.
    frag_color = vec4(light.rgb * shadow, light.a);
}
"#;
