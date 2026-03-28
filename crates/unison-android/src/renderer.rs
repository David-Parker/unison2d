//! OpenGL ES 3.0 renderer for Android.
//!
//! A direct port of `unison-web/src/renderer.rs` from `web_sys` to `glow`.
//! The rendering logic (shaders, draw commands, render targets) is identical;
//! only the GL API surface differs.

use std::collections::HashMap;

use glow::HasContext;
use unison_render::{
    AntiAliasing, BlendMode, Camera, Color, RenderCommand, RenderTargetId, Renderer,
    TextureDescriptor, TextureFilter, TextureFormat, TextureId, TextureWrap,
};

use crate::shaders;

/// Cached uniform locations for a shader program.
struct ProgramUniforms {
    program: glow::Program,
    u_view_projection: glow::UniformLocation,
    u_color: glow::UniformLocation,
}

/// Uniform locations specific to the base (non-lit) shader.
struct BaseUniforms {
    u_use_texture: glow::UniformLocation,
    u_texture: glow::UniformLocation,
}

/// Uniform locations specific to the lit sprite shader.
struct LitUniforms {
    u_use_texture: glow::UniformLocation,
    u_texture: glow::UniformLocation,
    u_shadow_mask: glow::UniformLocation,
    u_screen_size: glow::UniformLocation,
    u_shadow_filter: glow::UniformLocation,
    u_shadow_strength: glow::UniformLocation,
}

/// Which shader program is currently active.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ActiveProgram {
    Base,
    Lit,
}

/// OpenGL ES 3.0 renderer using `glow`.
pub struct GlesRenderer {
    gl: glow::Context,
    // Base shader (sprites, meshes, rects, lines, terrain)
    base: ProgramUniforms,
    base_uniforms: BaseUniforms,
    // Lit sprite shader (light gradient x shadow mask x PCF)
    lit: ProgramUniforms,
    lit_uniforms: LitUniforms,
    active_program: ActiveProgram,
    // Buffers (reused each frame)
    vao: glow::VertexArray,
    position_buffer: glow::Buffer,
    uv_buffer: glow::Buffer,
    vertex_color_buffer: glow::Buffer,
    index_buffer: glow::Buffer,
    // Texture storage
    textures: HashMap<u32, glow::Texture>,
    next_texture_id: u32,
    // Render targets (offscreen FBOs with MSAA)
    msaa_fbos: HashMap<u32, glow::Framebuffer>,
    msaa_renderbuffers: HashMap<u32, glow::Renderbuffer>,
    render_targets: HashMap<u32, glow::Framebuffer>,
    render_target_sizes: HashMap<u32, (u32, u32)>,
    next_render_target_id: u32,
    current_render_target: RenderTargetId,
    msaa_samples: i32,
    // State — physical pixel dimensions
    screen_width: f32,
    screen_height: f32,
    // Logical dimensions (dp, matches touch coordinate space)
    screen_width_points: f32,
    screen_height_points: f32,
    current_blend_mode: BlendMode,
}

/// Reinterpret a typed slice as raw bytes for GL buffer uploads.
unsafe fn as_u8_slice<T>(slice: &[T]) -> &[u8] {
    std::slice::from_raw_parts(slice.as_ptr() as *const u8, std::mem::size_of_val(slice))
}

impl GlesRenderer {
    /// Create a new GLES renderer.
    ///
    /// Must be called on the GL thread (after EGL context is current).
    /// `width` and `height` are physical pixel dimensions of the drawable.
    pub fn new(width: f32, height: f32) -> Result<Self, String> {
        // On Android, GL function pointers come from libEGL/libGLESv3.
        // We dlopen libEGL to get eglGetProcAddress, then use it for GL symbols.
        let gl = unsafe {
            let egl_lib = libc::dlopen(
                b"libEGL.so\0".as_ptr() as *const _,
                libc::RTLD_LAZY,
            );
            let egl_get_proc: Option<
                unsafe extern "C" fn(*const std::ffi::c_char) -> *const std::ffi::c_void,
            > = if !egl_lib.is_null() {
                let sym = libc::dlsym(egl_lib, b"eglGetProcAddress\0".as_ptr() as *const _);
                if sym.is_null() {
                    None
                } else {
                    Some(std::mem::transmute(sym))
                }
            } else {
                None
            };

            glow::Context::from_loader_function(|s| {
                let c_str = std::ffi::CString::new(s).unwrap();
                // Try eglGetProcAddress first (finds extension + core GLES functions)
                if let Some(get_proc) = egl_get_proc {
                    let ptr = get_proc(c_str.as_ptr());
                    if !ptr.is_null() {
                        return ptr as *const _;
                    }
                }
                // Fallback to dlsym on libGLESv3 / libGLESv2
                let ptr = libc::dlsym(libc::RTLD_DEFAULT, c_str.as_ptr());
                ptr as *const _
            })
        };

        // ── Base shader program ──
        let base_program =
            compile_program(&gl, shaders::VERTEX_SHADER, shaders::FRAGMENT_SHADER)?;
        let base = unsafe {
            ProgramUniforms {
                u_view_projection: gl
                    .get_uniform_location(base_program, "u_view_projection")
                    .ok_or("base: Failed to get u_view_projection")?,
                u_color: gl
                    .get_uniform_location(base_program, "u_color")
                    .ok_or("base: Failed to get u_color")?,
                program: base_program,
            }
        };
        let base_uniforms = unsafe {
            BaseUniforms {
                u_use_texture: gl
                    .get_uniform_location(base.program, "u_use_texture")
                    .ok_or("base: Failed to get u_use_texture")?,
                u_texture: gl
                    .get_uniform_location(base.program, "u_texture")
                    .ok_or("base: Failed to get u_texture")?,
            }
        };

        // ── Lit sprite shader program ──
        let lit_program =
            compile_program(&gl, shaders::VERTEX_SHADER, shaders::LIT_FRAGMENT_SHADER)?;
        let lit = unsafe {
            ProgramUniforms {
                u_view_projection: gl
                    .get_uniform_location(lit_program, "u_view_projection")
                    .ok_or("lit: Failed to get u_view_projection")?,
                u_color: gl
                    .get_uniform_location(lit_program, "u_color")
                    .ok_or("lit: Failed to get u_color")?,
                program: lit_program,
            }
        };
        let lit_uniforms = unsafe {
            LitUniforms {
                u_use_texture: gl
                    .get_uniform_location(lit.program, "u_use_texture")
                    .ok_or("lit: Failed to get u_use_texture")?,
                u_texture: gl
                    .get_uniform_location(lit.program, "u_texture")
                    .ok_or("lit: Failed to get u_texture")?,
                u_shadow_mask: gl
                    .get_uniform_location(lit.program, "u_shadow_mask")
                    .ok_or("lit: Failed to get u_shadow_mask")?,
                u_screen_size: gl
                    .get_uniform_location(lit.program, "u_screen_size")
                    .ok_or("lit: Failed to get u_screen_size")?,
                u_shadow_filter: gl
                    .get_uniform_location(lit.program, "u_shadow_filter")
                    .ok_or("lit: Failed to get u_shadow_filter")?,
                u_shadow_strength: gl
                    .get_uniform_location(lit.program, "u_shadow_strength")
                    .ok_or("lit: Failed to get u_shadow_strength")?,
            }
        };

        // Create VAO and buffers
        let (vao, position_buffer, uv_buffer, vertex_color_buffer, index_buffer) = unsafe {
            let vao = gl.create_vertex_array().map_err(|e| e.to_string())?;
            gl.bind_vertex_array(Some(vao));

            let position_buffer = gl.create_buffer().map_err(|e| e.to_string())?;
            let uv_buffer = gl.create_buffer().map_err(|e| e.to_string())?;
            let vertex_color_buffer = gl.create_buffer().map_err(|e| e.to_string())?;
            let index_buffer = gl.create_buffer().map_err(|e| e.to_string())?;

            // Attribute 0: position (vec2)
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(position_buffer));
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);

            // Attribute 1: UV (vec2)
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(uv_buffer));
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, 0, 0);

            // Attribute 2: vertex color (vec4) — disabled by default, uses constant (1,1,1,1)
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_color_buffer));
            gl.vertex_attrib_pointer_f32(2, 4, glow::FLOAT, false, 0, 0);
            gl.disable_vertex_attrib_array(2);
            gl.vertex_attrib_4_f32(2, 1.0, 1.0, 1.0, 1.0);

            // Index buffer
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer));

            gl.bind_vertex_array(None);

            (vao, position_buffer, uv_buffer, vertex_color_buffer, index_buffer)
        };

        // Enable blending
        unsafe {
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        }

        // Default to no MSAA — budget Adreno/Mali GPUs take a big hit from
        // multisample renderbuffer storage and blit resolve. Games can opt
        // in via set_anti_aliasing() for higher-end devices.
        let msaa_samples = 0;

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
            vertex_color_buffer,
            index_buffer,
            textures: HashMap::new(),
            next_texture_id: 1,
            msaa_fbos: HashMap::new(),
            msaa_renderbuffers: HashMap::new(),
            render_targets: HashMap::new(),
            render_target_sizes: HashMap::new(),
            next_render_target_id: 1,
            current_render_target: RenderTargetId::SCREEN,
            msaa_samples,
            screen_width: width,
            screen_height: height,
            screen_width_points: width,
            screen_height_points: height,
            current_blend_mode: BlendMode::Alpha,
        })
    }

    // ── Display frame methods (called by GameState, not on Renderer trait) ──

    /// Begin a display frame. Binds the default framebuffer and clears.
    pub fn begin_display_frame(&mut self) {
        unsafe {
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            self.gl.viewport(0, 0, self.screen_width as i32, self.screen_height as i32);
            self.gl.clear_color(0.0, 0.0, 0.0, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }
        self.current_render_target = RenderTargetId::SCREEN;
    }

    /// End a display frame. GLSurfaceView handles eglSwapBuffers automatically.
    pub fn end_display_frame(&mut self) {
        unsafe { self.gl.flush() };
    }

    // ── Internal helpers ──

    /// Build a 3x3 view-projection matrix from Camera.
    /// Maps world coordinates to clip space [-1, 1].
    fn build_view_projection(camera: &Camera) -> [f32; 9] {
        let half_w = camera.width / (2.0 * camera.zoom);
        let half_h = camera.height / (2.0 * camera.zoom);

        let sx = 1.0 / half_w;
        let sy = 1.0 / half_h;
        let tx = -camera.x / half_w;
        let ty = -camera.y / half_h;

        if camera.rotation == 0.0 {
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
            unsafe { self.gl.use_program(Some(self.base.program)) };
            self.active_program = ActiveProgram::Base;
        }
    }

    /// Ensure the lit shader program is active.
    fn use_lit_program(&mut self) {
        if self.active_program != ActiveProgram::Lit {
            unsafe { self.gl.use_program(Some(self.lit.program)) };
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
        vertex_colors: Option<&[f32]>,
    ) {
        self.use_base_program();

        unsafe {
            self.gl.bind_vertex_array(Some(self.vao));

            // Upload positions
            self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.position_buffer));
            self.gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, as_u8_slice(positions), glow::DYNAMIC_DRAW);

            // Upload UVs
            self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.uv_buffer));
            self.gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, as_u8_slice(uvs), glow::DYNAMIC_DRAW);

            // Upload per-vertex colors if provided, otherwise use constant white
            if let Some(colors) = vertex_colors {
                self.gl.enable_vertex_attrib_array(2);
                self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vertex_color_buffer));
                self.gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, as_u8_slice(colors), glow::DYNAMIC_DRAW);
            } else {
                self.gl.disable_vertex_attrib_array(2);
                self.gl.vertex_attrib_4_f32(2, 1.0, 1.0, 1.0, 1.0);
            }

            // Upload indices
            self.gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.index_buffer));
            self.gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, as_u8_slice(indices), glow::DYNAMIC_DRAW);

            // Set color
            self.gl.uniform_4_f32(Some(&self.base.u_color), color.r, color.g, color.b, color.a);

            // Bind texture if valid
            if texture.is_valid() {
                if let Some(&tex) = self.textures.get(&texture.0) {
                    self.gl.uniform_1_i32(Some(&self.base_uniforms.u_use_texture), 1);
                    self.gl.active_texture(glow::TEXTURE0);
                    self.gl.bind_texture(glow::TEXTURE_2D, Some(tex));
                    self.gl.uniform_1_i32(Some(&self.base_uniforms.u_texture), 0);
                } else {
                    self.gl.uniform_1_i32(Some(&self.base_uniforms.u_use_texture), 0);
                }
            } else {
                self.gl.uniform_1_i32(Some(&self.base_uniforms.u_use_texture), 0);
            }

            // Draw
            self.gl.draw_elements(glow::TRIANGLES, indices.len() as i32, glow::UNSIGNED_INT, 0);

            self.gl.bind_vertex_array(None);
        }
    }

    /// Draw a lit sprite: light gradient x shadow mask with PCF.
    fn draw_lit_sprite_data(
        &mut self,
        positions: &[f32],
        uvs: &[f32],
        indices: &[u32],
        color: Color,
        texture: TextureId,
        shadow_mask: TextureId,
        screen_size: (f32, f32),
        shadow_filter: u32,
        shadow_strength: f32,
    ) {
        self.use_lit_program();

        unsafe {
            self.gl.bind_vertex_array(Some(self.vao));

            // Ensure per-vertex color is at default for lit sprites
            self.gl.disable_vertex_attrib_array(2);
            self.gl.vertex_attrib_4_f32(2, 1.0, 1.0, 1.0, 1.0);

            // Upload geometry
            self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.position_buffer));
            self.gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, as_u8_slice(positions), glow::DYNAMIC_DRAW);

            self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.uv_buffer));
            self.gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, as_u8_slice(uvs), glow::DYNAMIC_DRAW);

            self.gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.index_buffer));
            self.gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, as_u8_slice(indices), glow::DYNAMIC_DRAW);

            // Set uniforms
            self.gl.uniform_4_f32(Some(&self.lit.u_color), color.r, color.g, color.b, color.a);
            self.gl.uniform_2_f32(Some(&self.lit_uniforms.u_screen_size), screen_size.0, screen_size.1);
            self.gl.uniform_1_i32(Some(&self.lit_uniforms.u_shadow_filter), shadow_filter as i32);
            self.gl.uniform_1_f32(Some(&self.lit_uniforms.u_shadow_strength), shadow_strength);

            // Bind light gradient texture to TEXTURE0
            if texture.is_valid() {
                if let Some(&tex) = self.textures.get(&texture.0) {
                    self.gl.uniform_1_i32(Some(&self.lit_uniforms.u_use_texture), 1);
                    self.gl.active_texture(glow::TEXTURE0);
                    self.gl.bind_texture(glow::TEXTURE_2D, Some(tex));
                    self.gl.uniform_1_i32(Some(&self.lit_uniforms.u_texture), 0);
                } else {
                    self.gl.uniform_1_i32(Some(&self.lit_uniforms.u_use_texture), 0);
                }
            } else {
                self.gl.uniform_1_i32(Some(&self.lit_uniforms.u_use_texture), 0);
            }

            // Bind shadow mask texture to TEXTURE1
            if let Some(&mask) = self.textures.get(&shadow_mask.0) {
                self.gl.active_texture(glow::TEXTURE1);
                self.gl.bind_texture(glow::TEXTURE_2D, Some(mask));
                self.gl.uniform_1_i32(Some(&self.lit_uniforms.u_shadow_mask), 1);
            }

            // Draw
            self.gl.draw_elements(glow::TRIANGLES, indices.len() as i32, glow::UNSIGNED_INT, 0);

            // Unbind TEXTURE1
            self.gl.active_texture(glow::TEXTURE1);
            self.gl.bind_texture(glow::TEXTURE_2D, None);
            self.gl.active_texture(glow::TEXTURE0);

            self.gl.bind_vertex_array(None);
        }
    }

    /// Generate a quad (2 triangles) for sprite/rect rendering.
    fn make_quad(
        x: f32, y: f32, w: f32, h: f32, rotation: f32, uv: [f32; 4],
    ) -> ([f32; 8], [f32; 8], [u32; 6]) {
        let hw = w / 2.0;
        let hh = h / 2.0;

        let corners = if rotation == 0.0 {
            [
                (x - hw, y - hh),
                (x + hw, y - hh),
                (x + hw, y + hh),
                (x - hw, y + hh),
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
            corners[0].0, corners[0].1,
            corners[1].0, corners[1].1,
            corners[2].0, corners[2].1,
            corners[3].0, corners[3].1,
        ];

        let [u0, v0, u1, v1] = uv;
        let uvs = [u0, v1, u1, v1, u1, v0, u0, v0];
        let indices = [0, 1, 2, 0, 2, 3];

        (positions, uvs, indices)
    }
}

impl Renderer for GlesRenderer {
    type Error = String;

    fn init(&mut self) -> Result<(), String> {
        unsafe {
            self.gl.viewport(0, 0, self.screen_width as i32, self.screen_height as i32);
        }
        Ok(())
    }

    fn begin_frame(&mut self, camera: &Camera) {
        let vp = Self::build_view_projection(camera);

        // Set VP matrix on both programs so switches mid-frame work correctly.
        unsafe {
            self.gl.use_program(Some(self.base.program));
            self.gl.uniform_matrix_3_f32_slice(Some(&self.base.u_view_projection), false, &vp);

            self.gl.use_program(Some(self.lit.program));
            self.gl.uniform_matrix_3_f32_slice(Some(&self.lit.u_view_projection), false, &vp);

            // Restore the active program
            match self.active_program {
                ActiveProgram::Base => self.gl.use_program(Some(self.base.program)),
                ActiveProgram::Lit => self.gl.use_program(Some(self.lit.program)),
            }
        }
    }

    fn clear(&mut self, color: Color) {
        unsafe {
            self.gl.clear_color(color.r, color.g, color.b, color.a);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }
    }

    fn draw(&mut self, command: RenderCommand) {
        match command {
            RenderCommand::LitSprite(lit) => {
                let (positions, uvs, indices) = Self::make_quad(
                    lit.position[0], lit.position[1],
                    lit.size[0], lit.size[1],
                    lit.rotation, lit.uv,
                );
                self.draw_lit_sprite_data(
                    &positions, &uvs, &indices,
                    lit.color, lit.texture, lit.shadow_mask,
                    lit.screen_size, lit.shadow_filter, lit.shadow_strength,
                );
            }
            RenderCommand::Mesh(mesh) => {
                let uvs = if mesh.uvs.is_empty() {
                    vec![0.0; mesh.positions.len()]
                } else {
                    mesh.uvs
                };
                self.draw_mesh_data(
                    &mesh.positions, &uvs, &mesh.indices,
                    mesh.color, mesh.texture,
                    mesh.vertex_colors.as_deref(),
                );
            }
            RenderCommand::Sprite(sprite) => {
                let (positions, uvs, indices) = Self::make_quad(
                    sprite.position[0], sprite.position[1],
                    sprite.size[0], sprite.size[1],
                    sprite.rotation, sprite.uv,
                );
                self.draw_mesh_data(
                    &positions, &uvs, &indices,
                    sprite.color, sprite.texture, None,
                );
            }
            RenderCommand::Rect { position, size, color } => {
                let cx = position[0] + size[0] / 2.0;
                let cy = position[1] + size[1] / 2.0;
                let (positions, uvs, indices) =
                    Self::make_quad(cx, cy, size[0], size[1], 0.0, [0.0, 0.0, 1.0, 1.0]);
                self.draw_mesh_data(&positions, &uvs, &indices, color, TextureId::NONE, None);
            }
            RenderCommand::Line { start, end, color, width } => {
                let dx = end[0] - start[0];
                let dy = end[1] - start[1];
                let len = (dx * dx + dy * dy).sqrt();
                if len < 1e-6 {
                    return;
                }
                let nx = -dy / len * width / 2.0;
                let ny = dx / len * width / 2.0;

                let positions = [
                    start[0] + nx, start[1] + ny,
                    start[0] - nx, start[1] - ny,
                    end[0] - nx, end[1] - ny,
                    end[0] + nx, end[1] + ny,
                ];
                let uvs = [0.0; 8];
                let indices = [0u32, 1, 2, 0, 2, 3];
                self.draw_mesh_data(&positions, &uvs, &indices, color, TextureId::NONE, None);
            }
            RenderCommand::Terrain { points, fill_color, line_color: _ } => {
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
                self.draw_mesh_data(&positions, &uvs, &indices, fill_color, TextureId::NONE, None);
            }
        }
    }

    fn end_frame(&mut self) {
        unsafe { self.gl.flush() };
    }

    fn create_texture(&mut self, desc: &TextureDescriptor) -> Result<TextureId, String> {
        unsafe {
            let texture = self.gl.create_texture().map_err(|e| e.to_string())?;
            self.gl.bind_texture(glow::TEXTURE_2D, Some(texture));

            let (internal_format, format) = match desc.format {
                TextureFormat::R8 => (glow::R8 as i32, glow::RED),
                TextureFormat::Rg8 => (glow::RG8 as i32, glow::RG),
                TextureFormat::Rgb8 => (glow::RGB8 as i32, glow::RGB),
                TextureFormat::Rgba8 => (glow::RGBA8 as i32, glow::RGBA),
            };

            self.gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                internal_format,
                desc.width as i32,
                desc.height as i32,
                0,
                format,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&desc.data)),
            );

            // Set filtering
            let min_filter = match desc.min_filter {
                TextureFilter::Nearest => glow::NEAREST,
                TextureFilter::Linear => glow::LINEAR,
                TextureFilter::LinearMipmap => glow::LINEAR_MIPMAP_LINEAR,
            };
            let mag_filter = match desc.mag_filter {
                TextureFilter::Nearest => glow::NEAREST,
                _ => glow::LINEAR,
            };
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, min_filter as i32);
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, mag_filter as i32);

            // Set wrapping
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, wrap_mode(desc.wrap_u) as i32);
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, wrap_mode(desc.wrap_v) as i32);

            // Generate mipmaps if needed
            if desc.min_filter == TextureFilter::LinearMipmap {
                self.gl.generate_mipmap(glow::TEXTURE_2D);
            }

            self.gl.bind_texture(glow::TEXTURE_2D, None);

            let id = self.next_texture_id;
            self.next_texture_id += 1;
            self.textures.insert(id, texture);
            Ok(TextureId(id))
        }
    }

    fn destroy_texture(&mut self, id: TextureId) {
        if let Some(tex) = self.textures.remove(&id.0) {
            unsafe { self.gl.delete_texture(tex) };
        }
    }

    fn screen_size(&self) -> (f32, f32) {
        (self.screen_width_points, self.screen_height_points)
    }

    fn drawable_size(&self) -> (f32, f32) {
        (self.screen_width, self.screen_height)
    }

    fn set_screen_size(&mut self, width: f32, height: f32) {
        self.screen_width_points = width;
        self.screen_height_points = height;
        // Physical pixel dimensions (screen_width/screen_height) are set in gameInit
        // from the actual GL surface size — don't overwrite them here.
    }

    fn set_blend_mode(&mut self, mode: BlendMode) {
        if mode == self.current_blend_mode {
            return;
        }
        unsafe {
            match mode {
                BlendMode::Alpha => {
                    self.gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
                }
                BlendMode::Additive => {
                    self.gl.blend_func(glow::SRC_ALPHA, glow::ONE);
                }
                BlendMode::Multiply => {
                    self.gl.blend_func(glow::DST_COLOR, glow::ZERO);
                }
            }
        }
        self.current_blend_mode = mode;
    }

    fn create_render_target(&mut self, width: u32, height: u32) -> Result<(RenderTargetId, TextureId), String> {
        unsafe {
            // ── Texture FBO (always created — used for sampling the result) ──
            let texture_fbo = self.gl.create_framebuffer().map_err(|e| e.to_string())?;

            let texture = self.gl.create_texture().map_err(|e| e.to_string())?;
            self.gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            self.gl.tex_image_2d(
                glow::TEXTURE_2D, 0, glow::RGBA8 as i32,
                width as i32, height as i32, 0,
                glow::RGBA, glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(None),
            );
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
            self.gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);

            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(texture_fbo));
            self.gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D, Some(texture), 0,
            );

            let status = self.gl.check_framebuffer_status(glow::FRAMEBUFFER);
            if status != glow::FRAMEBUFFER_COMPLETE {
                self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
                self.gl.delete_framebuffer(texture_fbo);
                self.gl.delete_texture(texture);
                return Err(format!("Framebuffer not complete: status {}", status));
            }

            // Register texture
            let tex_id = self.next_texture_id;
            self.next_texture_id += 1;
            self.textures.insert(tex_id, texture);

            let rt_id = self.next_render_target_id;
            self.next_render_target_id += 1;

            if self.msaa_samples > 1 {
                // ── MSAA path: separate draw FBO with renderbuffer, blit to texture FBO ──
                let msaa_fbo = self.gl.create_framebuffer().map_err(|e| e.to_string())?;
                let rbo = self.gl.create_renderbuffer().map_err(|e| e.to_string())?;

                self.gl.bind_renderbuffer(glow::RENDERBUFFER, Some(rbo));
                self.gl.renderbuffer_storage_multisample(
                    glow::RENDERBUFFER, self.msaa_samples,
                    glow::RGBA8, width as i32, height as i32,
                );

                self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(msaa_fbo));
                self.gl.framebuffer_renderbuffer(
                    glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0,
                    glow::RENDERBUFFER, Some(rbo),
                );

                let status = self.gl.check_framebuffer_status(glow::FRAMEBUFFER);
                if status != glow::FRAMEBUFFER_COMPLETE {
                    self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
                    self.gl.delete_framebuffer(msaa_fbo);
                    self.gl.delete_framebuffer(texture_fbo);
                    self.gl.delete_renderbuffer(rbo);
                    self.gl.delete_texture(texture);
                    return Err(format!("MSAA framebuffer not complete: status {}", status));
                }

                self.msaa_fbos.insert(rt_id, msaa_fbo);
                self.msaa_renderbuffers.insert(rt_id, rbo);
            } else {
                // ── No MSAA: draw directly to the texture FBO ──
                // Store the texture FBO as both the "msaa" (draw) and resolve target
                // so bind_render_target works without special-casing.
                self.msaa_fbos.insert(rt_id, texture_fbo);
            }

            // Unbind
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            self.gl.bind_renderbuffer(glow::RENDERBUFFER, None);
            self.gl.bind_texture(glow::TEXTURE_2D, None);

            self.render_targets.insert(rt_id, texture_fbo);
            self.render_target_sizes.insert(rt_id, (width, height));

            Ok((RenderTargetId(rt_id), TextureId(tex_id)))
        }
    }

    fn bind_render_target(&mut self, target: RenderTargetId) {
        unsafe {
            // Unbind any texture from TEXTURE0 to prevent feedback loops
            self.gl.active_texture(glow::TEXTURE0);
            self.gl.bind_texture(glow::TEXTURE_2D, None);

            // Resolve the current MSAA FBO -> resolve FBO before switching away.
            // Skip if no MSAA (msaa_fbo == resolve_fbo, i.e. same FBO).
            let prev = self.current_render_target;
            if prev != target && prev != RenderTargetId::SCREEN {
                if let (Some(&msaa_fbo), Some(&resolve_fbo)) =
                    (self.msaa_fbos.get(&prev.0), self.render_targets.get(&prev.0))
                {
                    if msaa_fbo != resolve_fbo {
                        if let Some(&(w, h)) = self.render_target_sizes.get(&prev.0) {
                            self.gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(msaa_fbo));
                            self.gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(resolve_fbo));
                            self.gl.blit_framebuffer(
                                0, 0, w as i32, h as i32,
                                0, 0, w as i32, h as i32,
                                glow::COLOR_BUFFER_BIT, glow::NEAREST,
                            );
                        }
                    }
                }
            }

            // Bind the new target
            if target == RenderTargetId::SCREEN {
                self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
                self.gl.viewport(0, 0, self.screen_width as i32, self.screen_height as i32);
            } else if let Some(&msaa_fbo) = self.msaa_fbos.get(&target.0) {
                self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(msaa_fbo));
                if let Some(&(w, h)) = self.render_target_sizes.get(&target.0) {
                    self.gl.viewport(0, 0, w as i32, h as i32);
                }
            }

            self.current_render_target = target;
        }
    }

    fn destroy_render_target(&mut self, target: RenderTargetId) {
        let msaa_fbo = self.msaa_fbos.remove(&target.0);
        let resolve_fbo = self.render_targets.remove(&target.0);
        unsafe {
            if let Some(fbo) = msaa_fbo {
                self.gl.delete_framebuffer(fbo);
            }
            if let Some(rbo) = self.msaa_renderbuffers.remove(&target.0) {
                self.gl.delete_renderbuffer(rbo);
            }
            // Only delete the resolve FBO if it's different from the MSAA FBO
            // (when MSAA is off, they're the same FBO — already deleted above)
            if let Some(fbo) = resolve_fbo {
                if msaa_fbo != Some(fbo) {
                    self.gl.delete_framebuffer(fbo);
                }
            }
        }
        self.render_target_sizes.remove(&target.0);
        // Note: the associated texture is NOT destroyed — the caller may still use it
    }

    fn fbo_origin_top_left(&self) -> bool {
        false // OpenGL has origin at bottom-left
    }

    fn set_anti_aliasing(&mut self, mode: AntiAliasing) {
        let max_samples = unsafe { self.gl.get_parameter_i32(glow::MAX_SAMPLES) };
        self.msaa_samples = (mode.samples() as i32).min(max_samples);
    }

    fn anti_aliasing(&self) -> AntiAliasing {
        match self.msaa_samples {
            s if s >= 8 => AntiAliasing::MSAAx8,
            s if s >= 4 => AntiAliasing::MSAAx4,
            s if s >= 2 => AntiAliasing::MSAAx2,
            _ => AntiAliasing::None,
        }
    }
}

fn wrap_mode(wrap: TextureWrap) -> u32 {
    match wrap {
        TextureWrap::Repeat => glow::REPEAT,
        TextureWrap::ClampToEdge => glow::CLAMP_TO_EDGE,
        TextureWrap::MirroredRepeat => glow::MIRRORED_REPEAT,
    }
}

// ── Shader compilation helpers ──

fn compile_shader(gl: &glow::Context, shader_type: u32, source: &str) -> Result<glow::Shader, String> {
    unsafe {
        let shader = gl.create_shader(shader_type).map_err(|e| e.to_string())?;
        gl.shader_source(shader, source);
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            let log = gl.get_shader_info_log(shader);
            gl.delete_shader(shader);
            return Err(format!("Shader compile error: {}", log));
        }
        Ok(shader)
    }
}

fn compile_program(
    gl: &glow::Context,
    vert_source: &str,
    frag_source: &str,
) -> Result<glow::Program, String> {
    let vert = compile_shader(gl, glow::VERTEX_SHADER, vert_source)?;
    let frag = compile_shader(gl, glow::FRAGMENT_SHADER, frag_source)?;
    unsafe {
        let program = gl.create_program().map_err(|e| e.to_string())?;
        gl.attach_shader(program, vert);
        gl.attach_shader(program, frag);
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            let log = gl.get_program_info_log(program);
            gl.delete_program(program);
            gl.delete_shader(vert);
            gl.delete_shader(frag);
            return Err(format!("Program link error: {}", log));
        }
        gl.detach_shader(program, vert);
        gl.detach_shader(program, frag);
        gl.delete_shader(vert);
        gl.delete_shader(frag);
        Ok(program)
    }
}
