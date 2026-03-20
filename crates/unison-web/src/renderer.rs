//! WebGL2 implementation of the Renderer trait

use std::collections::HashMap;
use web_sys::{
    WebGl2RenderingContext as GL, WebGlBuffer, WebGlFramebuffer, WebGlProgram, WebGlShader,
    WebGlTexture, WebGlUniformLocation, WebGlVertexArrayObject,
};
use unison_math::Color;
use unison_render::{
    BlendMode, Camera, RenderCommand, RenderTargetId, Renderer, TextureDescriptor, TextureFilter,
    TextureFormat, TextureId, TextureWrap,
};

use crate::shaders;

/// Cached uniform locations for a shader program.
struct ProgramUniforms {
    program: WebGlProgram,
    u_view_projection: WebGlUniformLocation,
    u_color: WebGlUniformLocation,
}

/// Uniform locations specific to the base (non-lit) shader.
struct BaseUniforms {
    u_use_texture: WebGlUniformLocation,
    u_texture: WebGlUniformLocation,
}

/// Uniform locations specific to the lit sprite shader.
struct LitUniforms {
    u_use_texture: WebGlUniformLocation,
    u_texture: WebGlUniformLocation,
    u_shadow_mask: WebGlUniformLocation,
    u_screen_size: WebGlUniformLocation,
    u_shadow_filter: WebGlUniformLocation,
}

/// Which shader program is currently active.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ActiveProgram {
    Base,
    Lit,
}

/// WebGL2 renderer implementing the Renderer trait.
pub struct WebGlRenderer {
    gl: GL,
    // Base shader (sprites, meshes, rects, lines, terrain)
    base: ProgramUniforms,
    base_uniforms: BaseUniforms,
    // Lit sprite shader (light gradient × shadow mask × PCF)
    lit: ProgramUniforms,
    lit_uniforms: LitUniforms,
    active_program: ActiveProgram,
    // Buffers (reused each frame)
    vao: WebGlVertexArrayObject,
    position_buffer: WebGlBuffer,
    uv_buffer: WebGlBuffer,
    index_buffer: WebGlBuffer,
    // Texture storage
    textures: HashMap<u32, WebGlTexture>,
    next_texture_id: u32,
    // Render targets (offscreen FBOs)
    render_targets: HashMap<u32, WebGlFramebuffer>,
    render_target_sizes: HashMap<u32, (u32, u32)>,
    next_render_target_id: u32,
    // State
    canvas_width: f32,
    canvas_height: f32,
    current_blend_mode: BlendMode,
}

impl WebGlRenderer {
    /// Create a new WebGL2 renderer from a canvas element.
    pub fn new(gl: GL, width: f32, height: f32) -> Result<Self, String> {
        // Compile shared vertex shader
        let vert = compile_shader(&gl, GL::VERTEX_SHADER, shaders::VERTEX_SHADER)?;

        // ── Base shader program ──
        let base_frag = compile_shader(&gl, GL::FRAGMENT_SHADER, shaders::FRAGMENT_SHADER)?;
        let base_program = link_program(&gl, &vert, &base_frag)?;

        let base = ProgramUniforms {
            u_view_projection: gl
                .get_uniform_location(&base_program, "u_view_projection")
                .ok_or("base: Failed to get u_view_projection")?,
            u_color: gl
                .get_uniform_location(&base_program, "u_color")
                .ok_or("base: Failed to get u_color")?,
            program: base_program,
        };
        let base_uniforms = BaseUniforms {
            u_use_texture: gl
                .get_uniform_location(&base.program, "u_use_texture")
                .ok_or("base: Failed to get u_use_texture")?,
            u_texture: gl
                .get_uniform_location(&base.program, "u_texture")
                .ok_or("base: Failed to get u_texture")?,
        };

        // ── Lit sprite shader program ──
        let lit_frag = compile_shader(&gl, GL::FRAGMENT_SHADER, shaders::LIT_FRAGMENT_SHADER)?;
        let lit_program = link_program(&gl, &vert, &lit_frag)?;

        let lit = ProgramUniforms {
            u_view_projection: gl
                .get_uniform_location(&lit_program, "u_view_projection")
                .ok_or("lit: Failed to get u_view_projection")?,
            u_color: gl
                .get_uniform_location(&lit_program, "u_color")
                .ok_or("lit: Failed to get u_color")?,
            program: lit_program,
        };
        let lit_uniforms = LitUniforms {
            u_use_texture: gl
                .get_uniform_location(&lit.program, "u_use_texture")
                .ok_or("lit: Failed to get u_use_texture")?,
            u_texture: gl
                .get_uniform_location(&lit.program, "u_texture")
                .ok_or("lit: Failed to get u_texture")?,
            u_shadow_mask: gl
                .get_uniform_location(&lit.program, "u_shadow_mask")
                .ok_or("lit: Failed to get u_shadow_mask")?,
            u_screen_size: gl
                .get_uniform_location(&lit.program, "u_screen_size")
                .ok_or("lit: Failed to get u_screen_size")?,
            u_shadow_filter: gl
                .get_uniform_location(&lit.program, "u_shadow_filter")
                .ok_or("lit: Failed to get u_shadow_filter")?,
        };

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
            base,
            base_uniforms,
            lit,
            lit_uniforms,
            active_program: ActiveProgram::Base,
            vao,
            position_buffer,
            uv_buffer,
            index_buffer,
            textures: HashMap::new(),
            next_texture_id: 1,
            render_targets: HashMap::new(),
            render_target_sizes: HashMap::new(),
            next_render_target_id: 1, // 0 = SCREEN
            canvas_width: width,
            canvas_height: height,
            current_blend_mode: BlendMode::Alpha,
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

    /// Ensure the base shader program is active.
    fn use_base_program(&mut self) {
        if self.active_program != ActiveProgram::Base {
            self.gl.use_program(Some(&self.base.program));
            self.active_program = ActiveProgram::Base;
        }
    }

    /// Ensure the lit shader program is active.
    fn use_lit_program(&mut self) {
        if self.active_program != ActiveProgram::Lit {
            self.gl.use_program(Some(&self.lit.program));
            self.active_program = ActiveProgram::Lit;
        }
    }

    /// Upload positions, UVs, and indices then draw triangles using the base shader.
    fn draw_mesh_data(
        &mut self,
        positions: &[f32],
        uvs: &[f32],
        indices: &[u32],
        color: Color,
        texture: TextureId,
    ) {
        self.use_base_program();
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
            Some(&self.base.u_color),
            color.r,
            color.g,
            color.b,
            color.a,
        );

        // Bind texture if valid
        if texture.is_valid() {
            if let Some(tex) = self.textures.get(&texture.0) {
                gl.uniform1i(Some(&self.base_uniforms.u_use_texture), 1);
                gl.active_texture(GL::TEXTURE0);
                gl.bind_texture(GL::TEXTURE_2D, Some(tex));
                gl.uniform1i(Some(&self.base_uniforms.u_texture), 0);
            } else {
                gl.uniform1i(Some(&self.base_uniforms.u_use_texture), 0);
            }
        } else {
            gl.uniform1i(Some(&self.base_uniforms.u_use_texture), 0);
        }

        // Draw
        gl.draw_elements_with_i32(GL::TRIANGLES, indices.len() as i32, GL::UNSIGNED_INT, 0);

        gl.bind_vertex_array(None);
    }

    /// Draw a lit sprite: light gradient × shadow mask with PCF.
    fn draw_lit_sprite(
        &mut self,
        positions: &[f32],
        uvs: &[f32],
        indices: &[u32],
        color: Color,
        texture: TextureId,
        shadow_mask: TextureId,
        screen_size: (f32, f32),
        shadow_filter: u32,
    ) {
        self.use_lit_program();
        let gl = &self.gl;

        gl.bind_vertex_array(Some(&self.vao));

        // Upload geometry
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.position_buffer));
        unsafe {
            let view = js_sys::Float32Array::view(positions);
            gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
        }
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.uv_buffer));
        unsafe {
            let view = js_sys::Float32Array::view(uvs);
            gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
        }
        gl.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, Some(&self.index_buffer));
        unsafe {
            let view = js_sys::Uint32Array::view(indices);
            gl.buffer_data_with_array_buffer_view(GL::ELEMENT_ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
        }

        // Set uniforms
        gl.uniform4f(Some(&self.lit.u_color), color.r, color.g, color.b, color.a);
        gl.uniform2f(Some(&self.lit_uniforms.u_screen_size), screen_size.0, screen_size.1);
        gl.uniform1i(Some(&self.lit_uniforms.u_shadow_filter), shadow_filter as i32);

        // Bind light gradient texture to TEXTURE0 (point lights have a gradient;
        // directional lights use TextureId::NONE for solid color)
        if texture.is_valid() {
            if let Some(tex) = self.textures.get(&texture.0) {
                gl.uniform1i(Some(&self.lit_uniforms.u_use_texture), 1);
                gl.active_texture(GL::TEXTURE0);
                gl.bind_texture(GL::TEXTURE_2D, Some(tex));
                gl.uniform1i(Some(&self.lit_uniforms.u_texture), 0);
            } else {
                gl.uniform1i(Some(&self.lit_uniforms.u_use_texture), 0);
            }
        } else {
            gl.uniform1i(Some(&self.lit_uniforms.u_use_texture), 0);
        }

        // Bind shadow mask texture to TEXTURE1
        if let Some(mask) = self.textures.get(&shadow_mask.0) {
            gl.active_texture(GL::TEXTURE1);
            gl.bind_texture(GL::TEXTURE_2D, Some(mask));
            gl.uniform1i(Some(&self.lit_uniforms.u_shadow_mask), 1);
        }

        // Draw
        gl.draw_elements_with_i32(GL::TRIANGLES, indices.len() as i32, GL::UNSIGNED_INT, 0);

        // Unbind TEXTURE1
        gl.active_texture(GL::TEXTURE1);
        gl.bind_texture(GL::TEXTURE_2D, None);
        gl.active_texture(GL::TEXTURE0);

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
        let vp = Self::build_view_projection(camera);

        // Set VP matrix on whichever program is currently active,
        // and also on the other one so switches mid-frame work correctly.
        let gl = &self.gl;

        gl.use_program(Some(&self.base.program));
        gl.uniform_matrix3fv_with_f32_array(Some(&self.base.u_view_projection), false, &vp);

        gl.use_program(Some(&self.lit.program));
        gl.uniform_matrix3fv_with_f32_array(Some(&self.lit.u_view_projection), false, &vp);

        // Restore the active program
        match self.active_program {
            ActiveProgram::Base => gl.use_program(Some(&self.base.program)),
            ActiveProgram::Lit => gl.use_program(Some(&self.lit.program)),
        }
    }

    fn clear(&mut self, color: Color) {
        let gl = &self.gl;
        gl.clear_color(color.r, color.g, color.b, color.a);
        gl.clear(GL::COLOR_BUFFER_BIT);
    }

    fn draw(&mut self, command: RenderCommand) {
        match command {
            RenderCommand::LitSprite(lit) => {
                let (positions, uvs, indices) = Self::make_quad(
                    lit.position[0],
                    lit.position[1],
                    lit.size[0],
                    lit.size[1],
                    lit.rotation,
                    lit.uv,
                );
                self.draw_lit_sprite(
                    &positions,
                    &uvs,
                    &indices,
                    lit.color,
                    lit.texture,
                    lit.shadow_mask,
                    lit.screen_size,
                    lit.shadow_filter,
                );
            }
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

    fn set_blend_mode(&mut self, mode: BlendMode) {
        if mode == self.current_blend_mode {
            return;
        }
        let gl = &self.gl;
        match mode {
            BlendMode::Alpha => {
                gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);
            }
            BlendMode::Additive => {
                gl.blend_func(GL::SRC_ALPHA, GL::ONE);
            }
            BlendMode::Multiply => {
                gl.blend_func(GL::DST_COLOR, GL::ZERO);
            }
        }
        self.current_blend_mode = mode;
    }

    fn create_render_target(&mut self, width: u32, height: u32) -> Result<(RenderTargetId, TextureId), String> {
        let gl = &self.gl;

        // Create framebuffer
        let fbo = gl.create_framebuffer()
            .ok_or("Failed to create framebuffer")?;

        // Create color texture
        let texture = gl.create_texture()
            .ok_or("Failed to create render target texture")?;
        gl.bind_texture(GL::TEXTURE_2D, Some(&texture));
        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            GL::TEXTURE_2D,
            0,
            GL::RGBA8 as i32,
            width as i32,
            height as i32,
            0,
            GL::RGBA,
            GL::UNSIGNED_BYTE,
            None,
        ).map_err(|e| format!("Failed to allocate render target texture: {:?}", e))?;

        // Linear filtering, clamp to edge
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, GL::LINEAR as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, GL::LINEAR as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::CLAMP_TO_EDGE as i32);

        // Attach texture to framebuffer
        gl.bind_framebuffer(GL::FRAMEBUFFER, Some(&fbo));
        gl.framebuffer_texture_2d(
            GL::FRAMEBUFFER,
            GL::COLOR_ATTACHMENT0,
            GL::TEXTURE_2D,
            Some(&texture),
            0,
        );

        // Check completeness
        let status = gl.check_framebuffer_status(GL::FRAMEBUFFER);
        if status != GL::FRAMEBUFFER_COMPLETE {
            gl.bind_framebuffer(GL::FRAMEBUFFER, None);
            gl.delete_framebuffer(Some(&fbo));
            gl.delete_texture(Some(&texture));
            return Err(format!("Framebuffer not complete: status {}", status));
        }

        // Unbind
        gl.bind_framebuffer(GL::FRAMEBUFFER, None);
        gl.bind_texture(GL::TEXTURE_2D, None);

        // Register texture so it can be used in draw commands
        let tex_id = self.next_texture_id;
        self.next_texture_id += 1;
        self.textures.insert(tex_id, texture);

        // Register render target
        let rt_id = self.next_render_target_id;
        self.next_render_target_id += 1;
        self.render_targets.insert(rt_id, fbo);
        self.render_target_sizes.insert(rt_id, (width, height));

        Ok((RenderTargetId(rt_id), TextureId(tex_id)))
    }

    fn bind_render_target(&mut self, target: RenderTargetId) {
        let gl = &self.gl;

        // Unbind any texture from TEXTURE0 to prevent feedback loops.
        // A stale texture binding from a prior draw call can conflict with
        // the FBO's color attachment if they reference the same texture.
        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, None);

        if target == RenderTargetId::SCREEN {
            gl.bind_framebuffer(GL::FRAMEBUFFER, None);
            gl.viewport(0, 0, self.canvas_width as i32, self.canvas_height as i32);
        } else if let Some(fbo) = self.render_targets.get(&target.0) {
            gl.bind_framebuffer(GL::FRAMEBUFFER, Some(fbo));
            if let Some(&(w, h)) = self.render_target_sizes.get(&target.0) {
                gl.viewport(0, 0, w as i32, h as i32);
            }
        }
    }

    fn destroy_render_target(&mut self, target: RenderTargetId) {
        if let Some(fbo) = self.render_targets.remove(&target.0) {
            self.gl.delete_framebuffer(Some(&fbo));
        }
        self.render_target_sizes.remove(&target.0);
        // Note: the associated texture is NOT destroyed — the caller may still use it
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
