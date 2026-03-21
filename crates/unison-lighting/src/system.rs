//! LightingSystem — manages lights and renders the lightmap.

use std::collections::HashMap;

use unison_math::Color;
use unison_render::{
    BlendMode, Camera, DrawLitSprite, DrawMesh, DrawSprite, RenderCommand, RenderTargetId,
    Renderer, TextureId,
};

use crate::gradient::generate_radial_gradient;
use crate::light::{DirectionalLight, LightId, PointLight};
use crate::occluder::Occluder;
use crate::shadow::{project_directional_shadows, project_point_shadows};

/// Manages point lights, ambient color, and the lightmap FBO.
///
/// The lighting system renders all light contributions to an offscreen
/// framebuffer (the "lightmap"), then composites it over the scene with
/// multiply blending. This darkens unlit areas and tints lit areas.
///
/// # Rendering flow
///
/// 1. [`ensure_resources`](Self::ensure_resources) — lazily creates the lightmap FBO and gradient texture
/// 2. [`render_lightmap`](Self::render_lightmap) — clears to ambient, draws each light additively
/// 3. [`composite_lightmap`](Self::composite_lightmap) — multiply-blends the lightmap over the current target
pub struct LightingSystem {
    lights: HashMap<u32, PointLight>,
    directional_lights: HashMap<u32, DirectionalLight>,
    next_id: u32,
    ambient: Color,
    enabled: bool,
    // FBO resources (lazily created on first render)
    lightmap_target: Option<RenderTargetId>,
    lightmap_texture: Option<TextureId>,
    gradient_texture: Option<TextureId>,
    lightmap_size: (u32, u32),
    // Shadow resources
    shadow_mask_target: Option<RenderTargetId>,
    shadow_mask_texture: Option<TextureId>,
    shadow_mask_size: (u32, u32),
    occluders: Vec<Occluder>,
    ground_shadow_y: Option<f32>,
}

impl LightingSystem {
    /// Create a new lighting system (disabled, no lights, black ambient).
    pub fn new() -> Self {
        Self {
            lights: HashMap::new(),
            directional_lights: HashMap::new(),
            next_id: 0,
            ambient: Color::BLACK,
            enabled: false,
            lightmap_target: None,
            lightmap_texture: None,
            gradient_texture: None,
            lightmap_size: (0, 0),
            shadow_mask_target: None,
            shadow_mask_texture: None,
            shadow_mask_size: (0, 0),
            occluders: Vec::new(),
            ground_shadow_y: None,
        }
    }

    // ── Ambient ──

    /// Set the ambient light color.
    ///
    /// This is the base illumination for areas not touched by any light.
    /// Use a dim color (e.g., `Color::new(0.1, 0.1, 0.15, 1.0)`) for
    /// atmospheric darkness, or `Color::WHITE` to effectively disable darkening.
    pub fn set_ambient(&mut self, color: Color) {
        self.ambient = color;
    }

    /// Get the current ambient light color.
    pub fn ambient(&self) -> Color {
        self.ambient
    }

    // ── Enable/disable ──

    /// Enable or disable the lighting system.
    ///
    /// When disabled, `auto_render` and `render_to_targets` skip the
    /// lighting pass entirely — the scene renders without any darkening.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if the lighting system is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    // ── Light management ──

    /// Add a point light and return its handle.
    pub fn add_light(&mut self, light: PointLight) -> LightId {
        let id = self.next_id;
        self.next_id += 1;
        self.lights.insert(id, light);
        LightId(id)
    }

    /// Remove a light by handle.
    pub fn remove_light(&mut self, id: LightId) {
        self.lights.remove(&id.0);
    }

    /// Get a light by handle.
    pub fn get_light(&self, id: LightId) -> Option<&PointLight> {
        self.lights.get(&id.0)
    }

    /// Get a mutable reference to a light by handle.
    pub fn get_light_mut(&mut self, id: LightId) -> Option<&mut PointLight> {
        self.lights.get_mut(&id.0)
    }

    /// Number of lights currently in the system.
    pub fn light_count(&self) -> usize {
        self.lights.len()
    }

    /// Remove all lights.
    pub fn clear_lights(&mut self) {
        self.lights.clear();
    }

    // ── Directional light management ──

    /// Add a directional light and return its handle.
    pub fn add_directional_light(&mut self, light: DirectionalLight) -> LightId {
        let id = self.next_id;
        self.next_id += 1;
        self.directional_lights.insert(id, light);
        LightId(id)
    }

    /// Remove a directional light by handle.
    pub fn remove_directional_light(&mut self, id: LightId) {
        self.directional_lights.remove(&id.0);
    }

    /// Get a directional light by handle.
    pub fn get_directional_light(&self, id: LightId) -> Option<&DirectionalLight> {
        self.directional_lights.get(&id.0)
    }

    /// Get a mutable reference to a directional light by handle.
    pub fn get_directional_light_mut(&mut self, id: LightId) -> Option<&mut DirectionalLight> {
        self.directional_lights.get_mut(&id.0)
    }

    /// Number of directional lights currently in the system.
    pub fn directional_light_count(&self) -> usize {
        self.directional_lights.len()
    }

    /// Remove all directional lights.
    pub fn clear_directional_lights(&mut self) {
        self.directional_lights.clear();
    }

    // ── Combined queries ──

    /// Check if there are any lights (point or directional) in the system.
    pub fn has_lights(&self) -> bool {
        !self.lights.is_empty() || !self.directional_lights.is_empty()
    }

    /// Get the lightmap texture ID (if resources have been created).
    pub fn lightmap_texture(&self) -> Option<TextureId> {
        self.lightmap_texture
    }

    // ── Shadows ──

    /// Provide occluder geometry for shadow casting this frame.
    ///
    /// Call this each frame before `render_lightmap()` with fresh occluder
    /// data from `ObjectSystem::collect_occluders()`.
    pub fn set_occluders(&mut self, occluders: Vec<Occluder>) {
        self.occluders = occluders;
    }

    /// Set the ground plane Y for ground shadow casting.
    ///
    /// Pass `None` to disable ground shadows. The ground occluder is a
    /// horizontal edge spanning the camera bounds at this Y position.
    pub fn set_ground_shadow(&mut self, y: Option<f32>) {
        self.ground_shadow_y = y;
    }

    /// Check if any light in the system casts shadows.
    fn has_shadow_casters(&self) -> bool {
        self.lights.values().any(|l| l.casts_shadows)
            || self.directional_lights.values().any(|l| l.casts_shadows)
    }

    // ── Rendering ──

    /// Create or resize the lightmap FBO, shadow mask FBO, and gradient texture.
    ///
    /// Called automatically before rendering. Uses `renderer.screen_size()`
    /// to match FBO sizes to the current viewport.
    pub fn ensure_resources(&mut self, renderer: &mut dyn Renderer<Error = String>) {
        let (w, h) = renderer.screen_size();
        let (w, h) = (w as u32, h as u32);

        // ── Lightmap FBO ──
        if !(self.lightmap_target.is_some() && self.lightmap_size == (w, h)) {
            // Destroy old lightmap if size changed
            if let Some(target) = self.lightmap_target.take() {
                renderer.destroy_render_target(target);
            }
            if let Some(tex) = self.lightmap_texture.take() {
                renderer.destroy_texture(tex);
            }

            let (target, texture) = renderer
                .create_render_target(w, h)
                .expect("Failed to create lightmap render target");
            self.lightmap_target = Some(target);
            self.lightmap_texture = Some(texture);
            self.lightmap_size = (w, h);
        }

        // ── Shadow mask FBO ──
        if self.has_shadow_casters() {
            if !(self.shadow_mask_target.is_some() && self.shadow_mask_size == (w, h)) {
                if let Some(target) = self.shadow_mask_target.take() {
                    renderer.destroy_render_target(target);
                }
                if let Some(tex) = self.shadow_mask_texture.take() {
                    renderer.destroy_texture(tex);
                }

                let (target, texture) = renderer
                    .create_render_target(w, h)
                    .expect("Failed to create shadow mask render target");
                self.shadow_mask_target = Some(target);
                self.shadow_mask_texture = Some(texture);
                self.shadow_mask_size = (w, h);
            }
        }

        // ── Gradient texture (once) ──
        if self.gradient_texture.is_none() {
            let desc = generate_radial_gradient(64);
            let tex = renderer
                .create_texture(&desc)
                .expect("Failed to create gradient texture");
            self.gradient_texture = Some(tex);
        }
    }

    /// Render all lights into the internal lightmap FBO.
    ///
    /// For each light:
    /// - If the light casts shadows and there are occluders, renders shadow
    ///   geometry to the shadow mask FBO, then draws the light as a
    ///   `LitSprite` that samples both the gradient and shadow mask.
    /// - Otherwise, draws the light as a plain additive sprite.
    ///
    /// The caller must call [`ensure_resources`](Self::ensure_resources) first.
    pub fn render_lightmap(&self, renderer: &mut dyn Renderer<Error = String>, camera: &Camera) {
        let lightmap_target = match self.lightmap_target {
            Some(t) => t,
            None => return,
        };
        let gradient = match self.gradient_texture {
            Some(t) => t,
            None => return,
        };

        let screen_size = renderer.screen_size();

        // Build combined occluder list (object occluders + optional ground)
        let occluders = self.build_occluders();
        let has_occluders = !occluders.is_empty();

        renderer.bind_render_target(lightmap_target);
        renderer.begin_frame(camera);
        renderer.clear(self.ambient);
        renderer.set_blend_mode(BlendMode::Additive);

        // ── Point lights ──
        for light in self.lights.values() {
            let size = light.radius * 2.0;
            let color = Color::new(
                light.color.r * light.intensity,
                light.color.g * light.intensity,
                light.color.b * light.intensity,
                1.0,
            );

            if light.casts_shadows && has_occluders {
                if let Some(shadow_mask_tex) = self.shadow_mask_texture {
                    // Render shadow mask for this light
                    self.render_shadow_mask_point(renderer, camera, light, &occluders);

                    // Switch back to lightmap and draw lit sprite
                    renderer.bind_render_target(lightmap_target);
                    renderer.begin_frame(camera);
                    renderer.set_blend_mode(BlendMode::Additive);

                    renderer.draw(RenderCommand::LitSprite(DrawLitSprite {
                        texture: gradient,
                        shadow_mask: shadow_mask_tex,
                        position: [light.position.x, light.position.y],
                        size: [size, size],
                        rotation: 0.0,
                        uv: [0.0, 0.0, 1.0, 1.0],
                        color,
                        screen_size,
                        shadow_filter: light.shadow.filter.as_uniform_value(),
                        shadow_strength: light.shadow.strength,
                    }));
                    continue;
                }
            }

            // Non-shadow path: plain additive sprite
            renderer.draw(RenderCommand::Sprite(DrawSprite {
                texture: gradient,
                position: [light.position.x, light.position.y],
                size: [size, size],
                rotation: 0.0,
                uv: [0.0, 0.0, 1.0, 1.0],
                color,
            }));
        }

        // ── Directional lights ──
        if !self.directional_lights.is_empty() {
            let (min_x, min_y, max_x, max_y) = camera.bounds();
            let cx = (min_x + max_x) / 2.0;
            let cy = (min_y + max_y) / 2.0;
            let w = max_x - min_x;
            let h = max_y - min_y;

            for light in self.directional_lights.values() {
                let color = Color::new(
                    light.color.r * light.intensity,
                    light.color.g * light.intensity,
                    light.color.b * light.intensity,
                    1.0,
                );

                if light.casts_shadows && has_occluders {
                    if let Some(shadow_mask_tex) = self.shadow_mask_texture {
                        self.render_shadow_mask_directional(
                            renderer, camera, light, &occluders, w, h,
                        );

                        renderer.bind_render_target(lightmap_target);
                        renderer.begin_frame(camera);
                        renderer.set_blend_mode(BlendMode::Additive);

                        renderer.draw(RenderCommand::LitSprite(DrawLitSprite {
                            texture: TextureId::NONE,
                            shadow_mask: shadow_mask_tex,
                            position: [cx, cy],
                            size: [w, h],
                            rotation: 0.0,
                            uv: [0.0, 0.0, 1.0, 1.0],
                            color,
                            screen_size,
                            shadow_filter: light.shadow.filter.as_uniform_value(),
                            shadow_strength: light.shadow.strength,
                        }));
                        continue;
                    }
                }

                renderer.draw(RenderCommand::Sprite(DrawSprite {
                    texture: TextureId::NONE,
                    position: [cx, cy],
                    size: [w, h],
                    rotation: 0.0,
                    uv: [0.0, 0.0, 1.0, 1.0],
                    color,
                }));
            }
        }

        renderer.set_blend_mode(BlendMode::Alpha);
        renderer.end_frame();
    }

    /// Build the combined occluder list for this frame.
    fn build_occluders(&self) -> Vec<Occluder> {
        self.occluders.clone()
    }

    /// Render shadow geometry for a point light to the shadow mask FBO.
    fn render_shadow_mask_point(
        &self,
        renderer: &mut dyn Renderer<Error = String>,
        camera: &Camera,
        light: &PointLight,
        occluders: &[Occluder],
    ) {
        let shadow_target = match self.shadow_mask_target {
            Some(t) => t,
            None => return,
        };

        let quads = project_point_shadows(
            [light.position.x, light.position.y],
            light.radius,
            occluders,
            light.shadow.distance,
            light.shadow.attenuation,
        );

        renderer.bind_render_target(shadow_target);
        renderer.begin_frame(camera);
        renderer.clear(Color::WHITE);
        renderer.set_blend_mode(BlendMode::Alpha);

        // Draw shadow quads with per-vertex colors (gradient for distance fade)
        let use_vertex_colors = light.shadow.distance > 0.0;
        for quad in &quads {
            renderer.draw(RenderCommand::Mesh(DrawMesh {
                positions: quad.positions.to_vec(),
                uvs: vec![0.0; quad.positions.len()],
                indices: quad.indices.to_vec(),
                texture: TextureId::NONE,
                color: Color::BLACK,
                vertex_colors: if use_vertex_colors {
                    Some(quad.vertex_colors.to_vec())
                } else {
                    None
                },
            }));
        }

        // Erase shadows below ground plane
        self.clear_below_ground(renderer, camera);

        renderer.end_frame();
    }

    /// Render shadow geometry for a directional light to the shadow mask FBO.
    fn render_shadow_mask_directional(
        &self,
        renderer: &mut dyn Renderer<Error = String>,
        camera: &Camera,
        light: &DirectionalLight,
        occluders: &[Occluder],
        cam_width: f32,
        cam_height: f32,
    ) {
        let shadow_target = match self.shadow_mask_target {
            Some(t) => t,
            None => return,
        };

        // Cast distance = camera diagonal so shadows span the full viewport
        let cast_distance = (cam_width * cam_width + cam_height * cam_height).sqrt();

        let quads = project_directional_shadows(
            [light.direction.x, light.direction.y],
            cast_distance,
            occluders,
            light.shadow.distance,
            light.shadow.attenuation,
        );

        renderer.bind_render_target(shadow_target);
        renderer.begin_frame(camera);
        renderer.clear(Color::WHITE);
        renderer.set_blend_mode(BlendMode::Alpha);

        let use_vertex_colors = light.shadow.distance > 0.0;
        for quad in &quads {
            renderer.draw(RenderCommand::Mesh(DrawMesh {
                positions: quad.positions.to_vec(),
                uvs: vec![0.0; quad.positions.len()],
                indices: quad.indices.to_vec(),
                texture: TextureId::NONE,
                color: Color::BLACK,
                vertex_colors: if use_vertex_colors {
                    Some(quad.vertex_colors.to_vec())
                } else {
                    None
                },
            }));
        }

        // Erase shadows below ground plane
        self.clear_below_ground(renderer, camera);

        renderer.end_frame();
    }

    /// Draw a white rect over everything below the ground shadow Y.
    ///
    /// This erases any shadow geometry that bled below the ground surface,
    /// effectively clipping shadows at the ground plane.
    fn clear_below_ground(
        &self,
        renderer: &mut dyn Renderer<Error = String>,
        camera: &Camera,
    ) {
        let ground_y = match self.ground_shadow_y {
            Some(y) => y,
            None => return,
        };

        let (min_x, min_y, max_x, _) = camera.bounds();
        let margin = (max_x - min_x) * 0.5;
        let x = min_x - margin;
        let w = (max_x - min_x) + margin * 2.0;
        let h = ground_y - min_y + margin;

        renderer.draw(RenderCommand::Rect {
            position: [x, min_y - margin],
            size: [w, h],
            color: Color::WHITE,
        });
    }

    /// Composite the lightmap over the currently-bound render target.
    ///
    /// Draws a full-viewport quad with the lightmap texture using multiply
    /// blending. This darkens unlit areas and tints lit areas. The caller
    /// should have already rendered the scene to the current target.
    pub fn composite_lightmap(
        &self,
        renderer: &mut dyn Renderer<Error = String>,
        camera: &Camera,
    ) {
        let lightmap_tex = match self.lightmap_texture {
            Some(t) => t,
            None => return,
        };

        let (min_x, min_y, max_x, max_y) = camera.bounds();
        let cx = (min_x + max_x) / 2.0;
        let cy = (min_y + max_y) / 2.0;

        renderer.begin_frame(camera);
        renderer.set_blend_mode(BlendMode::Multiply);
        renderer.draw(RenderCommand::Sprite(DrawSprite {
            texture: lightmap_tex,
            position: [cx, cy],
            size: [max_x - min_x, max_y - min_y],
            rotation: 0.0,
            // V-flip UVs for FBO texture orientation
            uv: [0.0, 1.0, 1.0, 0.0],
            color: Color::WHITE,
        }));
        renderer.set_blend_mode(BlendMode::Alpha);
        renderer.end_frame();
    }
}

impl Default for LightingSystem {
    fn default() -> Self {
        Self::new()
    }
}
