//! WebGL2 implementation of the Renderer trait

use std::collections::HashMap;
use web_sys::{
    WebGl2RenderingContext as GL, WebGlBuffer, WebGlProgram, WebGlShader, WebGlTexture,
    WebGlUniformLocation, WebGlVertexArrayObject,
};
use unison_math::Color;
use unison_render::{
    Camera, RenderCommand, Renderer, TextureDescriptor, TextureFilter,
    TextureFormat, TextureId, TextureWrap,
};

use crate::shaders;

/// WebGL2 renderer implementing the Renderer trait.
pub struct WebGlRenderer {
    gl: GL,
    program: WebGlProgram,
    // Uniforms
    u_view_projection: WebGlUniformLocation,
    u_color: WebGlUniformLocation,
    u_use_texture: WebGlUniformLocation,
    u_texture: WebGlUniformLocation,
    // Buffers (reused each frame)
    vao: WebGlVertexArrayObject,
    position_buffer: WebGlBuffer,
    uv_buffer: WebGlBuffer,
    index_buffer: WebGlBuffer,
    // Texture storage
    textures: HashMap<u32, WebGlTexture>,
    next_texture_id: u32,
    // State
    canvas_width: f32,
    canvas_height: f32,
}

impl WebGlRenderer {
    /// Create a new WebGL2 renderer from a canvas element.
    pub fn new(gl: GL, width: f32, height: f32) -> Result<Self, String> {
        // Compile shaders and link program
        let vert = compile_shader(&gl, GL::VERTEX_SHADER, shaders::VERTEX_SHADER)?;
        let frag = compile_shader(&gl, GL::FRAGMENT_SHADER, shaders::FRAGMENT_SHADER)?;
        let program = link_program(&gl, &vert, &frag)?;

        // Get uniform locations
        let u_view_projection = gl
            .get_uniform_location(&program, "u_view_projection")
            .ok_or("Failed to get u_view_projection location")?;
        let u_color = gl
            .get_uniform_location(&program, "u_color")
            .ok_or("Failed to get u_color location")?;
        let u_use_texture = gl
            .get_uniform_location(&program, "u_use_texture")
            .ok_or("Failed to get u_use_texture location")?;
        let u_texture = gl
            .get_uniform_location(&program, "u_texture")
            .ok_or("Failed to get u_texture location")?;

        // Create VAO and buffers
        let vao = gl
            .create_vertex_array()
            .ok_or("Failed to create VAO")?;
        let position_buffer = gl
            .create_buffer()
            .ok_or("Failed to create position buffer")?;
        let uv_buffer = gl.create_buffer().ok_or("Failed to create UV buffer")?;
        let index_buffer = gl
            .create_buffer()
            .ok_or("Failed to create index buffer")?;

        // Set up VAO with attribute layout
        gl.bind_vertex_array(Some(&vao));

        // Attribute 0: position (vec2)
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&position_buffer));
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);

        // Attribute 1: UV (vec2)
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&uv_buffer));
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_with_i32(1, 2, GL::FLOAT, false, 0, 0);

        // Index buffer
        gl.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, Some(&index_buffer));

        gl.bind_vertex_array(None);

        // Enable blending
        gl.enable(GL::BLEND);
        gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);

        Ok(Self {
            gl,
            program,
            u_view_projection,
            u_color,
            u_use_texture,
            u_texture,
            vao,
            position_buffer,
            uv_buffer,
            index_buffer,
            textures: HashMap::new(),
            next_texture_id: 1,
            canvas_width: width,
            canvas_height: height,
        })
    }

    /// Update canvas size (call when canvas resizes)
    pub fn set_size(&mut self, width: f32, height: f32) {
        self.canvas_width = width;
        self.canvas_height = height;
        self.gl
            .viewport(0, 0, width as i32, height as i32);
    }

    /// Build a 3x3 view-projection matrix from Camera.
    /// Maps world coordinates to clip space [-1, 1].
    fn build_view_projection(camera: &Camera) -> [f32; 9] {
        let half_w = camera.width / (2.0 * camera.zoom);
        let half_h = camera.height / (2.0 * camera.zoom);

        // Orthographic: scale then translate
        let sx = 1.0 / half_w;
        let sy = 1.0 / half_h;
        let tx = -camera.x / half_w;
        let ty = -camera.y / half_h;

        if camera.rotation == 0.0 {
            // Column-major 3x3
            [sx, 0.0, 0.0, 0.0, sy, 0.0, tx, ty, 1.0]
        } else {
            let cos_r = camera.rotation.cos();
            let sin_r = camera.rotation.sin();
            [
                sx * cos_r,
                sy * sin_r,
                0.0,
                -sx * sin_r,
                sy * cos_r,
                0.0,
                tx * cos_r - ty * sin_r,
                tx * sin_r + ty * cos_r,
                1.0,
            ]
        }
    }

    /// Upload positions, UVs, and indices then draw triangles.
    fn draw_mesh_data(
        &self,
        positions: &[f32],
        uvs: &[f32],
        indices: &[u32],
        color: Color,
        texture: TextureId,
    ) {
        let gl = &self.gl;

        gl.bind_vertex_array(Some(&self.vao));

        // Upload positions
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.position_buffer));
        unsafe {
            let view = js_sys::Float32Array::view(positions);
            gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
        }

        // Upload UVs
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.uv_buffer));
        unsafe {
            let view = js_sys::Float32Array::view(uvs);
            gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
        }

        // Upload indices
        gl.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, Some(&self.index_buffer));
        unsafe {
            let view = js_sys::Uint32Array::view(indices);
            gl.buffer_data_with_array_buffer_view(
                GL::ELEMENT_ARRAY_BUFFER,
                &view,
                GL::DYNAMIC_DRAW,
            );
        }

        // Set color
        gl.uniform4f(
            Some(&self.u_color),
            color.r,
            color.g,
            color.b,
            color.a,
        );

        // Bind texture if valid
        if texture.is_valid() {
            if let Some(tex) = self.textures.get(&texture.0) {
                gl.uniform1i(Some(&self.u_use_texture), 1);
                gl.active_texture(GL::TEXTURE0);
                gl.bind_texture(GL::TEXTURE_2D, Some(tex));
                gl.uniform1i(Some(&self.u_texture), 0);
            } else {
                gl.uniform1i(Some(&self.u_use_texture), 0);
            }
        } else {
            gl.uniform1i(Some(&self.u_use_texture), 0);
        }

        // Draw
        gl.draw_elements_with_i32(GL::TRIANGLES, indices.len() as i32, GL::UNSIGNED_INT, 0);

        gl.bind_vertex_array(None);
    }

    /// Generate a quad (2 triangles) for sprite/rect rendering.
    fn make_quad(
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        rotation: f32,
        uv: [f32; 4],
    ) -> ([f32; 8], [f32; 8], [u32; 6]) {
        let hw = w / 2.0;
        let hh = h / 2.0;

        let corners = if rotation == 0.0 {
            [
                (x - hw, y - hh), // bottom-left
                (x + hw, y - hh), // bottom-right
                (x + hw, y + hh), // top-right
                (x - hw, y + hh), // top-left
            ]
        } else {
            let cos_r = rotation.cos();
            let sin_r = rotation.sin();
            let offsets = [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)];
            let mut result = [(0.0f32, 0.0f32); 4];
            for (i, (ox, oy)) in offsets.iter().enumerate() {
                result[i] = (
                    x + ox * cos_r - oy * sin_r,
                    y + ox * sin_r + oy * cos_r,
                );
            }
            result
        };

        let positions = [
            corners[0].0,
            corners[0].1,
            corners[1].0,
            corners[1].1,
            corners[2].0,
            corners[2].1,
            corners[3].0,
            corners[3].1,
        ];

        let [u0, v0, u1, v1] = uv;
        let uvs = [u0, v1, u1, v1, u1, v0, u0, v0];
        let indices = [0, 1, 2, 0, 2, 3];

        (positions, uvs, indices)
    }
}

impl Renderer for WebGlRenderer {
    type Error = String;

    fn init(&mut self) -> Result<(), String> {
        self.gl.viewport(
            0,
            0,
            self.canvas_width as i32,
            self.canvas_height as i32,
        );
        Ok(())
    }

    fn begin_frame(&mut self, camera: &Camera) {
        let gl = &self.gl;
        gl.use_program(Some(&self.program));

        let vp = Self::build_view_projection(camera);
        gl.uniform_matrix3fv_with_f32_array(Some(&self.u_view_projection), false, &vp);
    }

    fn clear(&mut self, color: Color) {
        let gl = &self.gl;
        gl.clear_color(color.r, color.g, color.b, color.a);
        gl.clear(GL::COLOR_BUFFER_BIT);
    }

    fn draw(&mut self, command: RenderCommand) {
        match command {
            RenderCommand::Mesh(mesh) => {
                let uvs = if mesh.uvs.is_empty() {
                    vec![0.0; mesh.positions.len()]
                } else {
                    mesh.uvs
                };
                self.draw_mesh_data(
                    &mesh.positions,
                    &uvs,
                    &mesh.indices,
                    mesh.color,
                    mesh.texture,
                );
            }
            RenderCommand::Sprite(sprite) => {
                let (positions, uvs, indices) = Self::make_quad(
                    sprite.position[0],
                    sprite.position[1],
                    sprite.size[0],
                    sprite.size[1],
                    sprite.rotation,
                    sprite.uv,
                );
                self.draw_mesh_data(
                    &positions,
                    &uvs,
                    &indices,
                    sprite.color,
                    sprite.texture,
                );
            }
            RenderCommand::Rect {
                position,
                size,
                color,
            } => {
                let cx = position[0] + size[0] / 2.0;
                let cy = position[1] + size[1] / 2.0;
                let (positions, uvs, indices) =
                    Self::make_quad(cx, cy, size[0], size[1], 0.0, [0.0, 0.0, 1.0, 1.0]);
                self.draw_mesh_data(&positions, &uvs, &indices, color, TextureId::NONE);
            }
            RenderCommand::Line {
                start,
                end,
                color,
                width,
            } => {
                // Draw line as a thin rectangle
                let dx = end[0] - start[0];
                let dy = end[1] - start[1];
                let len = (dx * dx + dy * dy).sqrt();
                if len < 1e-6 {
                    return;
                }
                let nx = -dy / len * width / 2.0;
                let ny = dx / len * width / 2.0;

                let positions = [
                    start[0] + nx,
                    start[1] + ny,
                    start[0] - nx,
                    start[1] - ny,
                    end[0] - nx,
                    end[1] - ny,
                    end[0] + nx,
                    end[1] + ny,
                ];
                let uvs = [0.0; 8];
                let indices = [0u32, 1, 2, 0, 2, 3];
                self.draw_mesh_data(&positions, &uvs, &indices, color, TextureId::NONE);
            }
            RenderCommand::Terrain {
                points,
                fill_color,
                line_color: _,
            } => {
                // Simple fan triangulation from first point
                if points.len() < 3 {
                    return;
                }
                let mut positions = Vec::with_capacity(points.len() * 2);
                for &(x, y) in &points {
                    positions.push(x);
                    positions.push(y);
                }
                let mut indices = Vec::new();
                for i in 1..(points.len() as u32 - 1) {
                    indices.push(0);
                    indices.push(i);
                    indices.push(i + 1);
                }
                let uvs = vec![0.0; positions.len()];
                self.draw_mesh_data(&positions, &uvs, &indices, fill_color, TextureId::NONE);
            }
        }
    }

    fn end_frame(&mut self) {
        self.gl.flush();
    }

    fn create_texture(&mut self, desc: &TextureDescriptor) -> Result<TextureId, String> {
        let gl = &self.gl;

        let texture = gl.create_texture().ok_or("Failed to create texture")?;
        gl.bind_texture(GL::TEXTURE_2D, Some(&texture));

        let (internal_format, format, _bytes_per_pixel) = match desc.format {
            TextureFormat::R8 => (GL::R8 as i32, GL::RED, 1),
            TextureFormat::Rg8 => (GL::RG8 as i32, GL::RG, 2),
            TextureFormat::Rgb8 => (GL::RGB8 as i32, GL::RGB, 3),
            TextureFormat::Rgba8 => (GL::RGBA8 as i32, GL::RGBA, 4),
        };

        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            GL::TEXTURE_2D,
            0,
            internal_format,
            desc.width as i32,
            desc.height as i32,
            0,
            format,
            GL::UNSIGNED_BYTE,
            Some(&desc.data),
        )
        .map_err(|e| format!("tex_image_2d failed: {:?}", e))?;

        // Set filtering
        let min_filter = match desc.min_filter {
            TextureFilter::Nearest => GL::NEAREST,
            TextureFilter::Linear => GL::LINEAR,
            TextureFilter::LinearMipmap => GL::LINEAR_MIPMAP_LINEAR,
        };
        let mag_filter = match desc.mag_filter {
            TextureFilter::Nearest => GL::NEAREST,
            _ => GL::LINEAR,
        };
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, min_filter as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, mag_filter as i32);

        // Set wrapping
        let wrap_s = wrap_mode(desc.wrap_u);
        let wrap_t = wrap_mode(desc.wrap_v);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, wrap_s as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, wrap_t as i32);

        // Generate mipmaps if needed
        if desc.min_filter == TextureFilter::LinearMipmap {
            gl.generate_mipmap(GL::TEXTURE_2D);
        }

        gl.bind_texture(GL::TEXTURE_2D, None);

        let id = self.next_texture_id;
        self.next_texture_id += 1;
        self.textures.insert(id, texture);
        Ok(TextureId(id))
    }

    fn destroy_texture(&mut self, id: TextureId) {
        if let Some(texture) = self.textures.remove(&id.0) {
            self.gl.delete_texture(Some(&texture));
        }
    }

    fn screen_size(&self) -> (f32, f32) {
        (self.canvas_width, self.canvas_height)
    }
}

fn wrap_mode(wrap: TextureWrap) -> u32 {
    match wrap {
        TextureWrap::Repeat => GL::REPEAT,
        TextureWrap::ClampToEdge => GL::CLAMP_TO_EDGE,
        TextureWrap::MirroredRepeat => GL::MIRRORED_REPEAT,
    }
}

fn compile_shader(gl: &GL, shader_type: u32, source: &str) -> Result<WebGlShader, String> {
    let shader = gl
        .create_shader(shader_type)
        .ok_or("Failed to create shader")?;
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);

    if gl
        .get_shader_parameter(&shader, GL::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        let log = gl.get_shader_info_log(&shader).unwrap_or_default();
        gl.delete_shader(Some(&shader));
        Err(format!("Shader compilation failed: {}", log))
    }
}

fn link_program(gl: &GL, vert: &WebGlShader, frag: &WebGlShader) -> Result<WebGlProgram, String> {
    let program = gl.create_program().ok_or("Failed to create program")?;
    gl.attach_shader(&program, vert);
    gl.attach_shader(&program, frag);
    gl.link_program(&program);

    if gl
        .get_program_parameter(&program, GL::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        let log = gl.get_program_info_log(&program).unwrap_or_default();
        gl.delete_program(Some(&program));
        Err(format!("Program linking failed: {}", log))
    }
}
