//! GLSL shader sources for WebGL2

/// Vertex shader — transforms positions by view-projection matrix
pub const VERTEX_SHADER: &str = r#"#version 300 es
precision mediump float;

layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_uv;

uniform mat3 u_view_projection;

out vec2 v_uv;

void main() {
    vec3 pos = u_view_projection * vec3(a_position, 1.0);
    gl_Position = vec4(pos.xy, 0.0, 1.0);
    v_uv = a_uv;
}
"#;

/// Fragment shader — solid color with optional texture
pub const FRAGMENT_SHADER: &str = r#"#version 300 es
precision mediump float;

in vec2 v_uv;

uniform vec4 u_color;
uniform bool u_use_texture;
uniform sampler2D u_texture;

out vec4 frag_color;

void main() {
    if (u_use_texture) {
        vec4 tex = texture(u_texture, v_uv);
        frag_color = tex * u_color;
    } else {
        frag_color = u_color;
    }
}
"#;
