//! Metal renderer — implements the `Renderer` trait for iOS.
//!
//! Maps the engine's platform-agnostic render commands to Metal draw calls.
//! Created by the game's FFI layer and injected into the `Engine` at startup.

use std::collections::HashMap;
use std::ffi::c_void;
use std::mem;

use foreign_types::ForeignType;
use metal::*;
use objc::runtime::Object;
use objc::sel;
use objc::sel_impl;

use unison_core::Color;
use unison_render::{
    AntiAliasing, BlendMode, Camera, DrawLitSprite, DrawMesh, DrawSprite, RenderCommand,
    RenderTargetId, Renderer, TextureDescriptor, TextureFilter, TextureFormat, TextureId,
};

use crate::shaders;

// GCD semaphore FFI for triple-buffering synchronization
const DISPATCH_TIME_FOREVER: u64 = !0;

extern "C" {
    fn dispatch_semaphore_create(value: i64) -> *mut Object;
    fn dispatch_semaphore_wait(dsema: *mut Object, timeout: u64) -> i64;
    fn dispatch_semaphore_signal(dsema: *mut Object) -> i64;
}

/// Safety: the dispatch_semaphore is thread-safe (GCD guarantees this).
unsafe impl Send for MetalRenderer {}

/// Number of frames in flight for triple-buffered vertex data.
const MAX_FRAMES_IN_FLIGHT: usize = 3;

/// Initial vertex buffer size in bytes (1 MB).
const INITIAL_VERTEX_BUFFER_SIZE: usize = 1024 * 1024;

/// Initial index buffer size in bytes (256 KB).
const INITIAL_INDEX_BUFFER_SIZE: usize = 256 * 1024;

// ── Uniform structs (must match MSL layout) ──

#[repr(C)]
#[derive(Copy, Clone)]
struct Uniforms {
    /// Metal's float3x3 stores each column as float4 (padded to 16 bytes).
    /// 3 columns × 16 bytes = 48 bytes total.
    view_projection: [[f32; 4]; 3],
}

#[repr(C)]
#[derive(Copy, Clone)]
struct FragmentUniforms {
    color: [f32; 4],
    use_texture: i32,
    _pad: [i32; 3],
}

#[repr(C)]
#[derive(Copy, Clone)]
struct LitFragmentUniforms {
    color: [f32; 4],
    use_texture: i32,
    _pad0: i32,
    screen_size: [f32; 2],
    shadow_filter: i32,
    shadow_strength: f32,
    _pad1: [f32; 2],
}

// ── Render target ──

struct RenderTarget {
    texture: Texture,
    msaa_texture: Option<Texture>,
    _width: u32,
    _height: u32,
}

/// Metal-based renderer for iOS.
///
/// Implements the [`Renderer`] trait from `unison-render`. Created with raw
/// Metal device and CAMetalLayer pointers received from Swift via FFI.
/// A set of pipeline states for one sample count (1 for screen, N for MSAA).
struct PipelineSet {
    base_alpha: RenderPipelineState,
    base_additive: RenderPipelineState,
    base_multiply: RenderPipelineState,
    lit_alpha: RenderPipelineState,
    lit_additive: RenderPipelineState,
}

pub struct MetalRenderer {
    device: Device,
    command_queue: CommandQueue,
    layer: *mut Object,

    // Pipeline states: screen (sample_count=1) and MSAA (sample_count=N)
    screen_pipelines: PipelineSet,
    msaa_pipelines: Option<PipelineSet>,

    // Shader libraries (kept for pipeline recreation on AA change)
    base_library: Library,
    lit_library: Library,

    // Default sampler
    sampler_state: SamplerState,

    // Triple-buffered vertex data
    vertex_buffers: [Buffer; MAX_FRAMES_IN_FLIGHT],
    index_buffers: [Buffer; MAX_FRAMES_IN_FLIGHT],
    frame_index: usize,
    vertex_offset: usize,
    index_offset: usize,

    // Textures
    textures: HashMap<u32, Texture>,
    next_texture_id: u32,

    // Render targets
    render_targets: HashMap<u32, RenderTarget>,
    next_render_target_id: u32,
    current_render_target: RenderTargetId,

    // Frame state — stored as retained (owned) objects
    current_command_buffer: Option<CommandBuffer>,
    current_encoder: Option<RenderCommandEncoder>,
    current_drawable: Option<Drawable>,
    view_projection: [[f32; 4]; 3],

    // Screen (pixel dimensions for Metal rendering)
    screen_width: f32,
    screen_height: f32,
    // Screen (point dimensions for game logic / UI / touch coords)
    screen_width_points: f32,
    screen_height_points: f32,

    // State tracking
    current_blend_mode: BlendMode,
    current_aa: AntiAliasing,

    // Triple-buffering semaphore — limits in-flight frames to MAX_FRAMES_IN_FLIGHT.
    // Signaled by the GPU completion handler, waited on at frame start.
    frame_semaphore: *mut Object,
}

impl MetalRenderer {
    /// Create a new Metal renderer from raw pointers.
    ///
    /// # Safety
    /// `raw_device` must be a valid `MTLDevice` pointer and `raw_layer` must be
    /// a valid `CAMetalLayer` pointer. These are obtained from Swift via FFI.
    pub unsafe fn new(
        raw_device: *mut Object,
        raw_layer: *mut Object,
        width: f32,
        height: f32,
    ) -> Result<Self, String> {
        if raw_device.is_null() || raw_layer.is_null() {
            return Err("Null device or layer pointer".into());
        }

        // Retain the device — Swift passes an unretained pointer via
        // Unmanaged.passUnretained, so we must retain before from_ptr
        // (which will release on drop).
        let _: *mut Object = objc::msg_send![raw_device, retain];
        let device = Device::from_ptr(raw_device as *mut MTLDevice);

        let command_queue = device.new_command_queue();

        // Compile shaders
        let base_source = shaders::base_shader_source();
        let lit_source = shaders::lit_shader_source();

        let base_library = device
            .new_library_with_source(&base_source, &CompileOptions::new())
            .map_err(|e| format!("Failed to compile base shaders: {}", e))?;
        let lit_library = device
            .new_library_with_source(&lit_source, &CompileOptions::new())
            .map_err(|e| format!("Failed to compile lit shaders: {}", e))?;

        // Create pipeline states (sample_count=1 for screen rendering)
        let screen_pipelines = Self::create_pipeline_set(
            &device, &base_library, &lit_library, 1,
        )?;

        // Default sampler (trilinear filtering, clamp-to-edge)
        let sampler_desc = SamplerDescriptor::new();
        sampler_desc.set_min_filter(MTLSamplerMinMagFilter::Linear);
        sampler_desc.set_mag_filter(MTLSamplerMinMagFilter::Linear);
        sampler_desc.set_mip_filter(MTLSamplerMipFilter::Linear);
        sampler_desc.set_address_mode_s(MTLSamplerAddressMode::ClampToEdge);
        sampler_desc.set_address_mode_t(MTLSamplerAddressMode::ClampToEdge);
        let sampler_state = device.new_sampler(&sampler_desc);

        // Triple-buffered vertex and index buffers
        let vertex_buffers = std::array::from_fn(|_| {
            device.new_buffer(
                INITIAL_VERTEX_BUFFER_SIZE as u64,
                MTLResourceOptions::CPUCacheModeDefaultCache
                    | MTLResourceOptions::StorageModeShared,
            )
        });
        let index_buffers = std::array::from_fn(|_| {
            device.new_buffer(
                INITIAL_INDEX_BUFFER_SIZE as u64,
                MTLResourceOptions::CPUCacheModeDefaultCache
                    | MTLResourceOptions::StorageModeShared,
            )
        });

        Ok(Self {
            device,
            command_queue,
            layer: raw_layer,
            screen_pipelines,
            msaa_pipelines: None,
            base_library,
            lit_library,
            sampler_state,
            vertex_buffers,
            index_buffers,
            frame_index: 0,
            vertex_offset: 0,
            index_offset: 0,
            textures: HashMap::new(),
            next_texture_id: 1,
            render_targets: HashMap::new(),
            next_render_target_id: 1,
            current_render_target: RenderTargetId::SCREEN,
            current_command_buffer: None,
            current_encoder: None,
            current_drawable: None,
            view_projection: [[0.0; 4]; 3],
            screen_width: width,
            screen_height: height,
            // Default points to pixels until game_resize is called with point-based size
            screen_width_points: width,
            screen_height_points: height,
            current_blend_mode: BlendMode::Alpha,
            current_aa: AntiAliasing::None,
            frame_semaphore: dispatch_semaphore_create(MAX_FRAMES_IN_FLIGHT as i64),
        })
    }

    fn create_pipeline(
        device: &DeviceRef,
        vertex_fn: &FunctionRef,
        fragment_fn: &FunctionRef,
        vertex_desc: &VertexDescriptorRef,
        blend_mode: BlendMode,
        sample_count: u64,
    ) -> Result<RenderPipelineState, String> {
        let desc = RenderPipelineDescriptor::new();
        desc.set_vertex_function(Some(vertex_fn));
        desc.set_fragment_function(Some(fragment_fn));
        desc.set_vertex_descriptor(Some(vertex_desc));
        if sample_count > 1 {
            desc.set_sample_count(sample_count);
        }

        let color_attachment = desc.color_attachments().object_at(0).unwrap();
        color_attachment.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
        color_attachment.set_blending_enabled(true);

        match blend_mode {
            BlendMode::Alpha => {
                color_attachment.set_source_rgb_blend_factor(MTLBlendFactor::SourceAlpha);
                color_attachment.set_destination_rgb_blend_factor(MTLBlendFactor::OneMinusSourceAlpha);
                color_attachment.set_source_alpha_blend_factor(MTLBlendFactor::SourceAlpha);
                color_attachment.set_destination_alpha_blend_factor(MTLBlendFactor::OneMinusSourceAlpha);
            }
            BlendMode::Additive => {
                color_attachment.set_source_rgb_blend_factor(MTLBlendFactor::SourceAlpha);
                color_attachment.set_destination_rgb_blend_factor(MTLBlendFactor::One);
                color_attachment.set_source_alpha_blend_factor(MTLBlendFactor::SourceAlpha);
                color_attachment.set_destination_alpha_blend_factor(MTLBlendFactor::One);
            }
            BlendMode::Multiply => {
                color_attachment.set_source_rgb_blend_factor(MTLBlendFactor::DestinationColor);
                color_attachment.set_destination_rgb_blend_factor(MTLBlendFactor::Zero);
                color_attachment.set_source_alpha_blend_factor(MTLBlendFactor::DestinationAlpha);
                color_attachment.set_destination_alpha_blend_factor(MTLBlendFactor::Zero);
            }
        }

        device
            .new_render_pipeline_state(&desc)
            .map_err(|e| format!("Failed to create pipeline: {}", e))
    }

    /// Create a full set of pipeline states for a given sample count.
    fn create_pipeline_set(
        device: &DeviceRef,
        base_library: &Library,
        lit_library: &Library,
        sample_count: u64,
    ) -> Result<PipelineSet, String> {
        let vertex_fn = base_library.get_function("vertex_main", None)
            .map_err(|e| format!("vertex_main: {}", e))?;
        let fragment_fn = base_library.get_function("fragment_main", None)
            .map_err(|e| format!("fragment_main: {}", e))?;
        let lit_vertex_fn = lit_library.get_function("vertex_main", None)
            .map_err(|e| format!("lit vertex_main: {}", e))?;
        let lit_fragment_fn = lit_library.get_function("lit_fragment_main", None)
            .map_err(|e| format!("lit_fragment_main: {}", e))?;

        // Vertex descriptor: position (float2), uv (float2), color (float4)
        let vd = VertexDescriptor::new();
        let attr0 = vd.attributes().object_at(0).unwrap();
        attr0.set_format(MTLVertexFormat::Float2);
        attr0.set_offset(0);
        attr0.set_buffer_index(0);
        let attr1 = vd.attributes().object_at(1).unwrap();
        attr1.set_format(MTLVertexFormat::Float2);
        attr1.set_offset(8);
        attr1.set_buffer_index(0);
        let attr2 = vd.attributes().object_at(2).unwrap();
        attr2.set_format(MTLVertexFormat::Float4);
        attr2.set_offset(16);
        attr2.set_buffer_index(0);
        let layout = vd.layouts().object_at(0).unwrap();
        layout.set_stride(32);
        layout.set_step_function(MTLVertexStepFunction::PerVertex);

        Ok(PipelineSet {
            base_alpha: Self::create_pipeline(device, &vertex_fn, &fragment_fn, &vd, BlendMode::Alpha, sample_count)?,
            base_additive: Self::create_pipeline(device, &vertex_fn, &fragment_fn, &vd, BlendMode::Additive, sample_count)?,
            base_multiply: Self::create_pipeline(device, &vertex_fn, &fragment_fn, &vd, BlendMode::Multiply, sample_count)?,
            lit_alpha: Self::create_pipeline(device, &lit_vertex_fn, &lit_fragment_fn, &vd, BlendMode::Alpha, sample_count)?,
            lit_additive: Self::create_pipeline(device, &lit_vertex_fn, &lit_fragment_fn, &vd, BlendMode::Additive, sample_count)?,
        })
    }

    /// Compute a 3×3 orthographic view-projection matrix from a Camera.
    /// Compute the 3×3 view-projection matrix, matching the WebGL renderer.
    /// Metal's float3x3 pads each column to float4 (16-byte aligned).
    fn compute_view_projection(camera: &Camera) -> [[f32; 4]; 3] {
        let half_w = camera.width / (2.0 * camera.zoom);
        let half_h = camera.height / (2.0 * camera.zoom);

        let sx = 1.0 / half_w;
        let sy = 1.0 / half_h;
        let tx = -camera.x / half_w;
        let ty = -camera.y / half_h;

        if camera.rotation == 0.0 {
            // Column-major 3×3, each column padded to float4
            [
                [sx, 0.0, 0.0, 0.0],
                [0.0, sy, 0.0, 0.0],
                [tx, ty, 1.0, 0.0],
            ]
        } else {
            let cos_r = camera.rotation.cos();
            let sin_r = camera.rotation.sin();
            [
                [sx * cos_r, sy * sin_r, 0.0, 0.0],
                [-sx * sin_r, sy * cos_r, 0.0, 0.0],
                [tx * cos_r - ty * sin_r, tx * sin_r + ty * cos_r, 1.0, 0.0],
            ]
        }
    }

    /// Write a textured quad into the vertex/index buffers.
    /// Returns (vertex_buffer_offset, index_buffer_offset, index_count).
    fn write_quad(
        &mut self,
        position: [f32; 2],
        size: [f32; 2],
        rotation: f32,
        uv: [f32; 4],
        vertex_color: [f32; 4],
    ) -> (usize, usize, u32) {
        let hw = size[0] * 0.5;
        let hh = size[1] * 0.5;
        let corners = [[-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh]];
        let (sin_r, cos_r) = rotation.sin_cos();

        // UV corners: [min_u, min_v, max_u, max_v]
        let uvs = [
            [uv[0], uv[3]], // bottom-left
            [uv[2], uv[3]], // bottom-right
            [uv[2], uv[1]], // top-right
            [uv[0], uv[1]], // top-left
        ];

        let v_off = self.vertex_offset;
        let i_off = self.index_offset;

        // Write 4 vertices: position(2f) + uv(2f) + color(4f) = 32 bytes each
        let vertex_data = unsafe {
            let ptr = (self.vertex_buffers[self.frame_index].contents() as *mut u8).add(v_off)
                as *mut f32;
            std::slice::from_raw_parts_mut(ptr, 4 * 8)
        };

        for (i, corner) in corners.iter().enumerate() {
            let rx = corner[0] * cos_r - corner[1] * sin_r + position[0];
            let ry = corner[0] * sin_r + corner[1] * cos_r + position[1];
            let base = i * 8;
            vertex_data[base] = rx;
            vertex_data[base + 1] = ry;
            vertex_data[base + 2] = uvs[i][0];
            vertex_data[base + 3] = uvs[i][1];
            vertex_data[base + 4] = vertex_color[0];
            vertex_data[base + 5] = vertex_color[1];
            vertex_data[base + 6] = vertex_color[2];
            vertex_data[base + 7] = vertex_color[3];
        }

        // Write 6 indices (two triangles: 0-1-2, 0-2-3)
        let index_data = unsafe {
            let ptr = (self.index_buffers[self.frame_index].contents() as *mut u8).add(i_off)
                as *mut u32;
            std::slice::from_raw_parts_mut(ptr, 6)
        };
        // Indices are relative to the vertex buffer offset (set_vertex_buffer
        // already positions Metal at v_off), so always use 0-3.
        index_data[0] = 0;
        index_data[1] = 1;
        index_data[2] = 2;
        index_data[3] = 0;
        index_data[4] = 2;
        index_data[5] = 3;

        self.vertex_offset += 4 * 32;
        self.index_offset += 6 * mem::size_of::<u32>();

        (v_off, i_off, 6)
    }

    /// Write mesh vertices into the vertex/index buffers.
    /// Returns (vertex_buffer_offset, index_buffer_offset, index_count).
    fn write_mesh(&mut self, mesh: &DrawMesh) -> (usize, usize, u32) {
        let vertex_count = mesh.positions.len() / 2;
        let v_off = self.vertex_offset;
        let i_off = self.index_offset;

        let vertex_data = unsafe {
            let ptr = (self.vertex_buffers[self.frame_index].contents() as *mut u8).add(v_off)
                as *mut f32;
            std::slice::from_raw_parts_mut(ptr, vertex_count * 8)
        };

        for i in 0..vertex_count {
            let base = i * 8;
            vertex_data[base] = mesh.positions[i * 2];
            vertex_data[base + 1] = mesh.positions[i * 2 + 1];
            vertex_data[base + 2] = mesh.uvs[i * 2];
            vertex_data[base + 3] = mesh.uvs[i * 2 + 1];
            if let Some(ref vc) = mesh.vertex_colors {
                vertex_data[base + 4] = vc[i * 4];
                vertex_data[base + 5] = vc[i * 4 + 1];
                vertex_data[base + 6] = vc[i * 4 + 2];
                vertex_data[base + 7] = vc[i * 4 + 3];
            } else {
                vertex_data[base + 4] = 1.0;
                vertex_data[base + 5] = 1.0;
                vertex_data[base + 6] = 1.0;
                vertex_data[base + 7] = 1.0;
            }
        }

        let index_data = unsafe {
            let ptr = (self.index_buffers[self.frame_index].contents() as *mut u8).add(i_off)
                as *mut u32;
            std::slice::from_raw_parts_mut(ptr, mesh.indices.len())
        };
        // Indices are relative — vertex buffer offset handles positioning.
        for (i, &idx) in mesh.indices.iter().enumerate() {
            index_data[i] = idx;
        }

        self.vertex_offset += vertex_count * 32;
        self.index_offset += mesh.indices.len() * mem::size_of::<u32>();

        (v_off, i_off, mesh.indices.len() as u32)
    }


    /// Get the MTLTexture from the current drawable (via ObjC message).
    fn drawable_texture(&self) -> Option<&TextureRef> {
        self.current_drawable.as_ref().map(|drawable| unsafe {
            let tex_ptr: *mut Object = objc::msg_send![drawable.as_ptr(), texture];
            &*(tex_ptr as *const TextureRef)
        })
    }

    /// Ensure the render encoder exists (lazy creation for render target switches).
    fn ensure_encoder(&mut self) {
        if self.current_encoder.is_some() {
            return;
        }


        let cmd_buf = self.current_command_buffer.as_ref().expect("No command buffer");
        let desc = RenderPassDescriptor::new();
        let color_att = desc.color_attachments().object_at(0).unwrap();

        if self.current_render_target == RenderTargetId::SCREEN {
            if let Some(tex) = self.drawable_texture() {
                color_att.set_texture(Some(tex));
            }
            color_att.set_load_action(MTLLoadAction::Load);
            color_att.set_store_action(MTLStoreAction::Store);
        } else {
            let target = self
                .render_targets
                .get(&self.current_render_target.0)
                .expect("Invalid render target");
            if let Some(ref msaa_tex) = target.msaa_texture {
                color_att.set_texture(Some(msaa_tex));
                color_att.set_resolve_texture(Some(&target.texture));
                color_att.set_load_action(MTLLoadAction::Load);
                color_att.set_store_action(MTLStoreAction::StoreAndMultisampleResolve);
            } else {
                color_att.set_texture(Some(&target.texture));
                color_att.set_load_action(MTLLoadAction::Load);
                color_att.set_store_action(MTLStoreAction::Store);
            }
        }

        let encoder = cmd_buf.new_render_command_encoder(&desc);
        encoder.set_fragment_sampler_state(0, Some(&self.sampler_state));
        self.current_encoder = Some(encoder.to_owned());
    }

    /// Encode a draw call using the base pipeline with given texture and color.
    fn encode_base_draw(
        &self,
        texture: TextureId,
        color: Color,
        v_off: usize,
        i_off: usize,
        index_count: u32,
    ) {
        let encoder = self.current_encoder.as_ref().unwrap();

        let ps = self.active_pipelines();
        let pipeline = match self.current_blend_mode {
            BlendMode::Alpha => &ps.base_alpha,
            BlendMode::Additive => &ps.base_additive,
            BlendMode::Multiply => &ps.base_multiply,
        };
        encoder.set_render_pipeline_state(pipeline);

        // Vertex uniforms (buffer index 1)
        let uniforms = Uniforms {
            view_projection: self.view_projection,
        };
        encoder.set_vertex_bytes(
            1,
            mem::size_of::<Uniforms>() as u64,
            &uniforms as *const Uniforms as *const c_void,
        );

        // Fragment uniforms (buffer index 0)
        let frag_uniforms = FragmentUniforms {
            color: [color.r, color.g, color.b, color.a],
            use_texture: if texture != TextureId::NONE { 1 } else { 0 },
            _pad: [0; 3],
        };
        encoder.set_fragment_bytes(
            0,
            mem::size_of::<FragmentUniforms>() as u64,
            &frag_uniforms as *const FragmentUniforms as *const c_void,
        );

        // Bind texture
        if texture != TextureId::NONE {
            if let Some(tex) = self.textures.get(&texture.0) {
                encoder.set_fragment_texture(0, Some(tex));
            }
        }

        // Set vertex buffer and draw
        let vb = &self.vertex_buffers[self.frame_index];
        let ib = &self.index_buffers[self.frame_index];
        encoder.set_vertex_buffer(0, Some(vb), v_off as u64);
        encoder.draw_indexed_primitives(
            MTLPrimitiveType::Triangle,
            index_count as u64,
            MTLIndexType::UInt32,
            ib,
            i_off as u64,
        );
    }

    fn draw_sprite(&mut self, sprite: DrawSprite) {
        self.ensure_encoder();

        let texture = sprite.texture;
        let color = sprite.color;

        let (v_off, i_off, count) = self.write_quad(
            sprite.position,
            sprite.size,
            sprite.rotation,
            sprite.uv,
            [1.0, 1.0, 1.0, 1.0],
        );

        self.encode_base_draw(texture, color, v_off, i_off, count);
    }

    fn draw_mesh(&mut self, mesh: DrawMesh) {
        let vertex_count = mesh.positions.len() / 2;
        if vertex_count == 0 || mesh.indices.is_empty() {
            return;
        }

        self.ensure_encoder();
        let texture = mesh.texture;
        let color = mesh.color;
        let (v_off, i_off, count) = self.write_mesh(&mesh);
        self.encode_base_draw(texture, color, v_off, i_off, count);
    }

    fn draw_lit_sprite(&mut self, lit: DrawLitSprite) {
        self.ensure_encoder();

        // Write quad geometry first
        let (v_off, i_off, count) = self.write_quad(
            lit.position,
            lit.size,
            lit.rotation,
            lit.uv,
            [1.0, 1.0, 1.0, 1.0],
        );

        let encoder = self.current_encoder.as_ref().unwrap();

        let ps = self.active_pipelines();
        let pipeline = match self.current_blend_mode {
            BlendMode::Alpha => &ps.lit_alpha,
            BlendMode::Additive => &ps.lit_additive,
            BlendMode::Multiply => &ps.lit_alpha,
        };
        encoder.set_render_pipeline_state(pipeline);

        let uniforms = Uniforms {
            view_projection: self.view_projection,
        };
        encoder.set_vertex_bytes(
            1,
            mem::size_of::<Uniforms>() as u64,
            &uniforms as *const Uniforms as *const c_void,
        );

        let frag_uniforms = LitFragmentUniforms {
            color: [lit.color.r, lit.color.g, lit.color.b, lit.color.a],
            use_texture: if lit.texture != TextureId::NONE { 1 } else { 0 },
            _pad0: 0,
            screen_size: [lit.screen_size.0, lit.screen_size.1],
            shadow_filter: lit.shadow_filter as i32,
            shadow_strength: lit.shadow_strength,
            _pad1: [0.0; 2],
        };
        encoder.set_fragment_bytes(
            0,
            mem::size_of::<LitFragmentUniforms>() as u64,
            &frag_uniforms as *const LitFragmentUniforms as *const c_void,
        );

        if lit.texture != TextureId::NONE {
            if let Some(tex) = self.textures.get(&lit.texture.0) {
                encoder.set_fragment_texture(0, Some(tex));
            }
        }
        if lit.shadow_mask != TextureId::NONE {
            if let Some(tex) = self.textures.get(&lit.shadow_mask.0) {
                encoder.set_fragment_texture(1, Some(tex));
            }
        }

        let vb = &self.vertex_buffers[self.frame_index];
        let ib = &self.index_buffers[self.frame_index];
        encoder.set_vertex_buffer(0, Some(vb), v_off as u64);
        encoder.draw_indexed_primitives(
            MTLPrimitiveType::Triangle,
            count as u64,
            MTLIndexType::UInt32,
            ib,
            i_off as u64,
        );
    }

    fn draw_line(&mut self, start: [f32; 2], end: [f32; 2], color: Color, width: f32) {
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-6 {
            return;
        }

        self.ensure_encoder();

        let cx = (start[0] + end[0]) * 0.5;
        let cy = (start[1] + end[1]) * 0.5;
        let angle = dy.atan2(dx);

        let (v_off, i_off, count) = self.write_quad(
            [cx, cy],
            [len, width],
            angle,
            [0.0, 0.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
        );

        self.encode_base_draw(TextureId::NONE, color, v_off, i_off, count);
    }

    fn draw_rect(&mut self, position: [f32; 2], size: [f32; 2], color: Color) {
        self.ensure_encoder();

        let center = [position[0] + size[0] * 0.5, position[1] + size[1] * 0.5];

        let (v_off, i_off, count) = self.write_quad(
            center,
            size,
            0.0,
            [0.0, 0.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
        );

        self.encode_base_draw(TextureId::NONE, color, v_off, i_off, count);
    }

    fn draw_terrain(&mut self, points: Vec<(f32, f32)>, fill_color: Color, line_color: Color) {
        if points.len() < 2 {
            return;
        }

        self.ensure_encoder();

        // Extrude terrain downward to fill
        let min_y = points
            .iter()
            .map(|p| p.1)
            .fold(f32::INFINITY, f32::min)
            - 10.0;

        let vertex_count = points.len() * 2;
        let tri_count = (points.len() - 1) * 2;
        let v_off = self.vertex_offset;
        let i_off = self.index_offset;

        // Write vertices
        let vertex_data = unsafe {
            let ptr = (self.vertex_buffers[self.frame_index].contents() as *mut u8).add(v_off)
                as *mut f32;
            std::slice::from_raw_parts_mut(ptr, vertex_count * 8)
        };

        for (i, &(x, y)) in points.iter().enumerate() {
            // Top vertex
            let b = i * 2 * 8;
            vertex_data[b] = x;
            vertex_data[b + 1] = y;
            vertex_data[b + 2] = 0.0;
            vertex_data[b + 3] = 0.0;
            vertex_data[b + 4] = 1.0;
            vertex_data[b + 5] = 1.0;
            vertex_data[b + 6] = 1.0;
            vertex_data[b + 7] = 1.0;
            // Bottom vertex
            let b = (i * 2 + 1) * 8;
            vertex_data[b] = x;
            vertex_data[b + 1] = min_y;
            vertex_data[b + 2] = 0.0;
            vertex_data[b + 3] = 1.0;
            vertex_data[b + 4] = 1.0;
            vertex_data[b + 5] = 1.0;
            vertex_data[b + 6] = 1.0;
            vertex_data[b + 7] = 1.0;
        }

        // Write indices
        let index_count = tri_count * 3;
        let index_data = unsafe {
            let ptr = (self.index_buffers[self.frame_index].contents() as *mut u8).add(i_off)
                as *mut u32;
            std::slice::from_raw_parts_mut(ptr, index_count)
        };

        // Indices are relative — vertex buffer offset handles positioning.
        for i in 0..(points.len() - 1) {
            let tl = (i * 2) as u32;
            let bl = tl + 1;
            let tr = ((i + 1) * 2) as u32;
            let br = tr + 1;
            let idx = i * 6;
            index_data[idx] = tl;
            index_data[idx + 1] = bl;
            index_data[idx + 2] = tr;
            index_data[idx + 3] = tr;
            index_data[idx + 4] = bl;
            index_data[idx + 5] = br;
        }

        self.vertex_offset += vertex_count * 32;
        self.index_offset += index_count * mem::size_of::<u32>();

        self.encode_base_draw(TextureId::NONE, fill_color, v_off, i_off, index_count as u32);

        // Draw surface line
        if line_color.a > 0.0 {
            for i in 0..(points.len() - 1) {
                self.draw_line(
                    [points[i].0, points[i].1],
                    [points[i + 1].0, points[i + 1].1],
                    line_color,
                    0.02,
                );
            }
        }
    }


    /// Get the pipeline set matching the current render target's sample count.
    /// Screen targets use sample_count=1, offscreen MSAA targets use the MSAA set.
    fn active_pipelines(&self) -> &PipelineSet {
        if self.current_render_target != RenderTargetId::SCREEN {
            if let Some(ref msaa) = self.msaa_pipelines {
                return msaa;
            }
        }
        &self.screen_pipelines
    }

    /// Find the highest MSAA mode the device supports at or below `requested`.
    fn best_supported_aa(&self, requested: AntiAliasing) -> AntiAliasing {
        let candidates = [
            AntiAliasing::MSAAx8,
            AntiAliasing::MSAAx4,
            AntiAliasing::MSAAx2,
            AntiAliasing::None,
        ];
        for &mode in &candidates {
            if mode.samples() > requested.samples() {
                continue;
            }
            let count = mode.samples() as u64;
            let supported: bool = unsafe {
                objc::msg_send![self.device.as_ref(), supportsTextureSampleCount: count]
            };
            if supported {
                return mode;
            }
        }
        AntiAliasing::None
    }

    // ── Display frame lifecycle (called by GameState, not by the engine) ──

    /// Begin a display frame. Called once per CADisplayLink tick, before the
    /// engine's render cycle (which may call begin_frame/end_frame multiple times).
    ///
    /// Creates the command buffer, acquires the drawable, and waits on the
    /// triple-buffering semaphore.
    /// # Safety
    /// `raw_drawable` must be a valid CAMetalDrawable pointer from MTKView.currentDrawable.
    pub unsafe fn begin_display_frame(&mut self, raw_drawable: *mut Object) {
        unison_profiler::profile_scope!("metal.cmd_buffer");
        // Wait for a buffer slot — only blocks if all MAX_FRAMES_IN_FLIGHT are in-flight
        dispatch_semaphore_wait(self.frame_semaphore, DISPATCH_TIME_FOREVER);

        // Create command buffer for this display frame
        let cb = self.command_queue.new_command_buffer();
        self.current_command_buffer = Some(cb.to_owned());

        // Use the drawable provided by MTKView (not nextDrawable — that conflicts
        // with MTKView's own drawable management and causes flickering).
        // Retain it since Swift passed an unretained pointer.
        if !raw_drawable.is_null() {
            let _: *mut Object = objc::msg_send![raw_drawable, retain];
            self.current_drawable =
                Some(Drawable::from_ptr(raw_drawable as *mut MTLDrawable));
        }

        // Sync physical pixel dimensions from the drawable texture.
        // This keeps drawable_size() correct after device rotation, since
        // game_resize only updates logical point dimensions.
        // (Inlined instead of using drawable_texture() to avoid borrowing all of self.)
        if let Some(ref drawable) = self.current_drawable {
            let tex_ptr: *mut Object = objc::msg_send![drawable.as_ptr(), texture];
            if !tex_ptr.is_null() {
                let tex = &*(tex_ptr as *const TextureRef);
                self.screen_width = tex.width() as f32;
                self.screen_height = tex.height() as f32;
            }
        }

        // Clear the drawable to black immediately. The engine's render cycle
        // may skip clear() (e.g., overlay-only frames), and Metal's Load
        // action would read garbage from uninitialized memory.
        if let (Some(cmd_buf), Some(tex)) = (
            self.current_command_buffer.as_ref(),
            self.drawable_texture(),
        ) {
            let desc = RenderPassDescriptor::new();
            let att = desc.color_attachments().object_at(0).unwrap();
            att.set_texture(Some(tex));
            att.set_load_action(MTLLoadAction::Clear);
            att.set_clear_color(MTLClearColor::new(0.0, 0.0, 0.0, 1.0));
            att.set_store_action(MTLStoreAction::Store);
            let enc = cmd_buf.new_render_command_encoder(&desc);
            enc.end_encoding();
        }

        // Reset vertex/index offsets for this frame
        self.vertex_offset = 0;
        self.index_offset = 0;
    }

    /// End the display frame. Called once per CADisplayLink tick, after the
    /// engine's render cycle is complete.
    ///
    /// Ends any active encoder, presents the drawable, and commits the command
    /// buffer. GPU completion signals the frame semaphore asynchronously.
    pub fn end_display_frame(&mut self) {
        // End any active render encoder
        if let Some(encoder) = self.current_encoder.take() {
            encoder.end_encoding();
        }

        if let Some(cmd_buf) = self.current_command_buffer.take() {
            // Present the drawable
            if let Some(drawable) = self.current_drawable.take() {
                cmd_buf.present_drawable(&drawable);
            }

            // Signal the semaphore when the GPU finishes this frame,
            // freeing a buffer slot for begin_display_frame.
            let semaphore = self.frame_semaphore;
            let handler = block::ConcreteBlock::new(move |_: &metal::CommandBufferRef| {
                unsafe { dispatch_semaphore_signal(semaphore); }
            });
            cmd_buf.add_completed_handler(&handler.copy());

            {
                unison_profiler::profile_scope!("metal.commit");
                cmd_buf.commit();
            }
        }

        self.current_drawable = None;
        self.frame_index = (self.frame_index + 1) % MAX_FRAMES_IN_FLIGHT;
    }
}

impl Renderer for MetalRenderer {
    type Error = String;

    fn init(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn begin_frame(&mut self, camera: &Camera) {
        self.view_projection = Self::compute_view_projection(camera);
    }

    fn clear(&mut self, color: Color) {
        unison_profiler::profile_scope!("metal.clear");
        // End existing encoder to start fresh with a clear
        if let Some(encoder) = self.current_encoder.take() {
            encoder.end_encoding();
        }

        // Acquire drawable before borrowing command buffer
        let cmd_buf = self
            .current_command_buffer
            .as_ref()
            .expect("No command buffer");
        let desc = RenderPassDescriptor::new();
        let color_att = desc.color_attachments().object_at(0).unwrap();

        if self.current_render_target == RenderTargetId::SCREEN {
            if let Some(tex) = self.drawable_texture() {
                color_att.set_texture(Some(tex));
                color_att.set_store_action(MTLStoreAction::Store);
            }
        } else {
            let target = self
                .render_targets
                .get(&self.current_render_target.0)
                .expect("Invalid render target");
            if let Some(ref msaa_tex) = target.msaa_texture {
                color_att.set_texture(Some(msaa_tex));
                color_att.set_resolve_texture(Some(&target.texture));
                // StoreAndMultisampleResolve preserves the MSAA texture data
                // so subsequent passes can re-bind this target with LoadAction::Load.
                // Plain MultisampleResolve discards the MSAA data after resolving,
                // which corrupts the scene when the lightmap composite re-binds.
                color_att.set_store_action(MTLStoreAction::StoreAndMultisampleResolve);
            } else {
                color_att.set_texture(Some(&target.texture));
                color_att.set_store_action(MTLStoreAction::Store);
            }
        }

        color_att.set_load_action(MTLLoadAction::Clear);
        color_att.set_clear_color(MTLClearColor::new(
            color.r as f64,
            color.g as f64,
            color.b as f64,
            color.a as f64,
        ));

        let encoder = cmd_buf.new_render_command_encoder(&desc);
        encoder.set_fragment_sampler_state(0, Some(&self.sampler_state));
        self.current_encoder = Some(encoder.to_owned());
    }

    fn draw(&mut self, command: RenderCommand) {
        unison_profiler::profile_scope!("metal.draw");
        match command {
            RenderCommand::Sprite(sprite) => self.draw_sprite(sprite),
            RenderCommand::Mesh(mesh) => self.draw_mesh(mesh),
            RenderCommand::LitSprite(lit) => self.draw_lit_sprite(lit),
            RenderCommand::Line {
                start,
                end,
                color,
                width,
            } => self.draw_line(start, end, color, width),
            RenderCommand::Rect {
                position,
                size,
                color,
            } => self.draw_rect(position, size, color),
            RenderCommand::Terrain {
                points,
                fill_color,
                line_color,
            } => self.draw_terrain(points, fill_color, line_color),
        }
    }

    fn end_frame(&mut self) {
        if let Some(encoder) = self.current_encoder.take() {
            encoder.end_encoding();
        }
    }

    fn create_texture(&mut self, desc: &TextureDescriptor) -> Result<TextureId, String> {
        let wants_mipmaps = desc.min_filter == TextureFilter::LinearMipmap;
        let mip_levels = if wants_mipmaps {
            let max_dim = desc.width.max(desc.height) as f32;
            (max_dim.log2().floor() as u64) + 1
        } else {
            1
        };

        let tex_desc = metal::TextureDescriptor::new();
        tex_desc.set_width(desc.width as u64);
        tex_desc.set_height(desc.height as u64);
        tex_desc.set_pixel_format(match desc.format {
            TextureFormat::R8 => MTLPixelFormat::R8Unorm,
            TextureFormat::Rg8 => MTLPixelFormat::RG8Unorm,
            TextureFormat::Rgb8 => MTLPixelFormat::RGBA8Unorm, // expand below
            TextureFormat::Rgba8 => MTLPixelFormat::RGBA8Unorm,
        });
        tex_desc.set_mipmap_level_count(mip_levels);
        tex_desc.set_usage(MTLTextureUsage::ShaderRead);
        tex_desc.set_storage_mode(MTLStorageMode::Shared);

        let texture = self.device.new_texture(&tex_desc);

        // Upload pixel data (expand RGB8 → RGBA8 since Metal has no RGB format)
        let (upload_data, bytes_per_row) = match desc.format {
            TextureFormat::Rgb8 => {
                let pixel_count = desc.width as usize * desc.height as usize;
                let mut rgba = Vec::with_capacity(pixel_count * 4);
                for i in 0..pixel_count {
                    rgba.push(desc.data[i * 3]);
                    rgba.push(desc.data[i * 3 + 1]);
                    rgba.push(desc.data[i * 3 + 2]);
                    rgba.push(255);
                }
                (rgba, desc.width as usize * 4)
            }
            TextureFormat::R8 => (desc.data.clone(), desc.width as usize),
            TextureFormat::Rg8 => (desc.data.clone(), desc.width as usize * 2),
            TextureFormat::Rgba8 => (desc.data.clone(), desc.width as usize * 4),
        };

        let region = MTLRegion::new_2d(0, 0, desc.width as u64, desc.height as u64);
        texture.replace_region(
            region,
            0,
            upload_data.as_ptr() as *const c_void,
            bytes_per_row as u64,
        );

        // Generate mipmaps on the GPU
        if wants_mipmaps && mip_levels > 1 {
            let cmd_buf = self.command_queue.new_command_buffer();
            let blit = cmd_buf.new_blit_command_encoder();
            blit.generate_mipmaps(&texture);
            blit.end_encoding();
            cmd_buf.commit();
            cmd_buf.wait_until_completed();
        }

        let id = self.next_texture_id;
        self.next_texture_id += 1;
        self.textures.insert(id, texture);
        Ok(TextureId(id))
    }

    fn destroy_texture(&mut self, id: TextureId) {
        self.textures.remove(&id.0);
    }

    fn screen_size(&self) -> (f32, f32) {
        (self.screen_width_points, self.screen_height_points)
    }

    fn drawable_size(&self) -> (f32, f32) {
        (self.screen_width, self.screen_height)
    }

    fn fbo_origin_top_left(&self) -> bool { true }

    fn set_screen_size(&mut self, width: f32, height: f32) {
        self.screen_width_points = width;
        self.screen_height_points = height;
    }

    fn set_blend_mode(&mut self, mode: BlendMode) {
        self.current_blend_mode = mode;
    }

    fn create_render_target(
        &mut self,
        width: u32,
        height: u32,
    ) -> Result<(RenderTargetId, TextureId), String> {
        let tex_desc = metal::TextureDescriptor::new();
        tex_desc.set_width(width as u64);
        tex_desc.set_height(height as u64);
        tex_desc.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
        tex_desc.set_usage(MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead);
        tex_desc.set_storage_mode(MTLStorageMode::Private);
        let texture = self.device.new_texture(&tex_desc);

        // Re-validate AA in case it was set before device capability was checked
        let aa = self.best_supported_aa(self.current_aa);
        self.current_aa = aa;

        let msaa_texture = if aa != AntiAliasing::None {
            let msaa_desc = metal::TextureDescriptor::new();
            msaa_desc.set_width(width as u64);
            msaa_desc.set_height(height as u64);
            msaa_desc.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
            msaa_desc.set_texture_type(MTLTextureType::D2Multisample);
            msaa_desc.set_sample_count(aa.samples() as u64);
            msaa_desc.set_usage(MTLTextureUsage::RenderTarget);
            msaa_desc.set_storage_mode(MTLStorageMode::Private);
            Some(self.device.new_texture(&msaa_desc))
        } else {
            None
        };

        let target_id = self.next_render_target_id;
        self.next_render_target_id += 1;
        let tex_id = self.next_texture_id;
        self.next_texture_id += 1;

        // Store in both maps so the texture can be used in draw commands
        self.textures.insert(tex_id, texture.clone());
        self.render_targets.insert(
            target_id,
            RenderTarget {
                texture,
                msaa_texture,
                _width: width,
                _height: height,
            },
        );

        Ok((RenderTargetId(target_id), TextureId(tex_id)))
    }

    fn bind_render_target(&mut self, target: RenderTargetId) {
        unison_profiler::profile_scope!("metal.bind_target");
        if let Some(encoder) = self.current_encoder.take() {
            encoder.end_encoding();
        }
        self.current_render_target = target;
    }

    fn destroy_render_target(&mut self, target: RenderTargetId) {
        self.render_targets.remove(&target.0);
    }

    fn set_anti_aliasing(&mut self, mode: AntiAliasing) {
        // Downgrade to the highest sample count the device supports.
        let aa = self.best_supported_aa(mode);
        self.current_aa = aa;

        // Create MSAA pipeline set if needed (must match MSAA texture sample count).
        if aa != AntiAliasing::None {
            self.msaa_pipelines = Self::create_pipeline_set(
                &self.device,
                &self.base_library,
                &self.lit_library,
                aa.samples() as u64,
            )
            .ok();
        } else {
            self.msaa_pipelines = None;
        }
    }

    fn anti_aliasing(&self) -> AntiAliasing {
        self.current_aa
    }
}
