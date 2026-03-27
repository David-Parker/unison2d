//! World — a self-contained game world with physics, objects, cameras, and lighting.
//!
//! Each Level (or the Game itself) owns a `World`. Multiple worlds can coexist
//! independently. The `World` struct composes subsystems:
//!
//! - `objects` ([`ObjectSystem`]) — physics simulation + object registry
//! - `cameras` ([`CameraSystem`]) — named cameras with optional follow targets
//! - `lighting` ([`LightingSystem`]) — point lights, directional lights, and lightmap compositing
//!
//! ## Render layers
//!
//! Use [`World::create_render_layer`] to define named render layers with per-layer
//! lighting control. Layers render in creation order. Lit layers are affected by
//! the lighting/shadow system; unlit layers are not. Use [`World::draw_to`] to
//! queue commands to a specific layer, or [`World::draw`] to queue to the default
//! scene layer. [`World::draw_overlay`] queues screen-space commands drawn after
//! all layers. All commands are cleared automatically each frame.

use unison_math::{Color, Vec2};
use unison_render::{
    BlendMode, Camera, DrawSprite, Renderer, RenderCommand, RenderTargetId, TextureId,
};
use unison_lighting::{LightId, LightingSystem};
use unison_profiler::profile_scope;

use crate::object::ObjectId;
use crate::object_system::ObjectSystem;
use crate::camera_system::CameraSystem;
use crate::camera_system::DEFAULT_CAMERA;

/// Rendering environment configuration for a World.
pub struct Environment {
    /// Background clear color.
    pub background_color: Color,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            background_color: Color::BLACK,
        }
    }
}

/// Handle for a render layer, returned by [`World::create_render_layer`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RenderLayerId(usize);

/// Configuration for a render layer.
pub struct RenderLayerConfig {
    /// Whether this layer is affected by the lighting/shadow system.
    /// Lit layers are rendered to an offscreen FBO with lighting composited;
    /// unlit layers render directly to the output target.
    pub lit: bool,
    /// Clear color for this layer. The first layer typically uses an opaque
    /// color (e.g. sky blue). Subsequent layers typically use
    /// [`Color::TRANSPARENT`] so lower layers show through.
    pub clear_color: Color,
}

/// A point light that tracks an object's position each step.
struct LightFollowTarget {
    light: LightId,
    object: ObjectId,
    offset: Vec2,
}

/// Internal render layer state.
struct RenderLayer {
    #[allow(dead_code)]
    name: String,
    config: RenderLayerConfig,
    commands: Vec<(RenderCommand, i32)>,
}

/// A self-contained game world.
///
/// Composes subsystems for physics/objects and cameras.
/// Call `step(dt)` each tick to advance the simulation.
///
/// ```ignore
/// let mut world = World::new();
/// world.set_background(Color::from_hex(0x1a1a2e));
/// world.objects.set_gravity(-9.8);
/// let player = world.objects.spawn_soft_body(desc);
/// world.cameras.follow("main", player, 0.08);
///
/// // Each tick:
/// world.step(dt);
/// ```
pub struct World {
    /// Object and physics subsystem.
    pub objects: ObjectSystem,
    /// Camera subsystem with named cameras.
    pub cameras: CameraSystem,
    /// Rendering environment (background color, etc.).
    pub environment: Environment,
    /// Lighting subsystem (disabled by default).
    pub lighting: LightingSystem,

    /// Named render layers, rendered in creation order.
    render_layers: Vec<RenderLayer>,
    /// The default lit "scene" layer — `draw()` routes here.
    default_layer: RenderLayerId,

    /// World-space draw commands drawn after all layers (not affected by lightmap).
    unlit_commands: Vec<(RenderCommand, i32)>,
    /// Screen-space overlay commands queued this frame (drawn after lighting).
    overlay_commands: Vec<(RenderCommand, i32)>,

    /// Point lights that auto-track object positions in `step()`.
    light_follow_targets: Vec<LightFollowTarget>,

    /// Offscreen FBO for lit layer groups (created lazily, resized on viewport change).
    scene_target: Option<RenderTargetId>,
    scene_texture: Option<TextureId>,
    scene_fbo_size: (u32, u32),
}

impl World {
    /// Create a new World with default settings.
    ///
    /// Comes with a default "main" camera (20x15 world units),
    /// standard gravity (-9.8), and a default lit "scene" render layer.
    pub fn new() -> Self {
        let default_layer = RenderLayerId(0);
        let scene_layer = RenderLayer {
            name: "scene".to_string(),
            config: RenderLayerConfig {
                lit: true,
                clear_color: Color::BLACK,
            },
            commands: Vec::new(),
        };

        Self {
            objects: ObjectSystem::new(),
            cameras: CameraSystem::new(),
            environment: Environment::default(),
            lighting: LightingSystem::new(),
            render_layers: vec![scene_layer],
            default_layer,
            light_follow_targets: Vec::new(),
            unlit_commands: Vec::new(),
            overlay_commands: Vec::new(),
            scene_target: None,
            scene_texture: None,
            scene_fbo_size: (0, 0),
        }
    }

    /// Set the background clear color.
    ///
    /// Updates the default scene layer's clear color and the environment
    /// background color. For custom layers, use [`set_layer_clear_color`](Self::set_layer_clear_color).
    pub fn set_background(&mut self, color: Color) {
        self.environment.background_color = color;
        self.render_layers[self.default_layer.0].config.clear_color = color;
    }

    /// Get the background clear color.
    pub fn background_color(&self) -> Color {
        self.environment.background_color
    }

    // ── Render layers ──

    /// Create a named render layer.
    ///
    /// Layers render in the order they are created. Lit layers are affected
    /// by the lighting/shadow system; unlit layers render directly to the
    /// output. Returns a handle for use with [`draw_to`](Self::draw_to).
    ///
    /// **Important:** Layers created *before* the default scene layer render
    /// behind it. To insert a layer before the scene, create it before any
    /// game objects are spawned — the default scene layer is always at index 0
    /// unless you insert before it.
    ///
    /// To insert a layer before the default scene layer, use
    /// [`create_render_layer_before`](Self::create_render_layer_before).
    pub fn create_render_layer(&mut self, name: &str, config: RenderLayerConfig) -> RenderLayerId {
        let id = RenderLayerId(self.render_layers.len());
        self.render_layers.push(RenderLayer {
            name: name.to_string(),
            config,
            commands: Vec::new(),
        });
        id
    }

    /// Create a named render layer inserted before `before`.
    ///
    /// All existing layer IDs at or after the insertion point (including
    /// `before`) shift by one. The default layer ID is updated automatically.
    pub fn create_render_layer_before(
        &mut self,
        name: &str,
        config: RenderLayerConfig,
        before: RenderLayerId,
    ) -> RenderLayerId {
        let idx = before.0;
        self.render_layers.insert(idx, RenderLayer {
            name: name.to_string(),
            config,
            commands: Vec::new(),
        });
        // Shift the default layer if it was at or after the insertion point
        if self.default_layer.0 >= idx {
            self.default_layer = RenderLayerId(self.default_layer.0 + 1);
        }
        RenderLayerId(idx)
    }

    /// Get the default scene layer ID.
    pub fn default_layer(&self) -> RenderLayerId {
        self.default_layer
    }

    /// Update a layer's clear color.
    pub fn set_layer_clear_color(&mut self, layer: RenderLayerId, color: Color) {
        self.render_layers[layer.0].config.clear_color = color;
    }

    // ── Custom draw commands ──

    /// Queue a world-space render command to a specific layer.
    ///
    /// Commands within a layer are sorted by z-order. The layer determines
    /// whether the command is affected by lighting. Cleared automatically
    /// after rendering.
    pub fn draw_to(&mut self, layer: RenderLayerId, command: RenderCommand, z_order: i32) {
        self.render_layers[layer.0].commands.push((command, z_order));
    }

    /// Queue a world-space render command to the default scene layer.
    ///
    /// Equivalent to `draw_to(world.default_layer(), command, z_order)`.
    /// Commands are sorted alongside scene objects by z-order and drawn
    /// in the scene layer's camera/lighting pass. Cleared automatically
    /// after rendering.
    pub fn draw(&mut self, command: RenderCommand, z_order: i32) {
        let layer = self.default_layer;
        self.draw_to(layer, command, z_order);
    }

    /// Queue a world-space render command drawn after all layers.
    ///
    /// Like [`draw`](Self::draw) but not affected by the lightmap multiply.
    /// Use for effects that should appear at full brightness in front of the
    /// lit scene. For sky/background elements, consider using an unlit render
    /// layer via [`create_render_layer`](Self::create_render_layer) instead.
    pub fn draw_unlit(&mut self, command: RenderCommand, z_order: i32) {
        self.unlit_commands.push((command, z_order));
    }

    /// Queue a screen-space overlay command at the given z-order.
    ///
    /// Overlays are drawn after all layers and the unlit pass, in screen-space
    /// coordinates (0,0 = bottom-left, 1,1 = top-right). Not affected by
    /// camera position or lighting. Cleared automatically after rendering.
    pub fn draw_overlay(&mut self, command: RenderCommand, z_order: i32) {
        self.overlay_commands.push((command, z_order));
    }

    // ── Spawning ──

    /// Spawn a soft body object.
    pub fn spawn_soft_body(&mut self, desc: crate::object::SoftBodyDesc) -> crate::object::ObjectId {
        self.objects.spawn_soft_body(desc)
    }

    /// Spawn a rigid body object.
    pub fn spawn_rigid_body(&mut self, desc: crate::object::RigidBodyDesc) -> crate::object::ObjectId {
        self.objects.spawn_rigid_body(desc)
    }

    /// Spawn a static rectangle (platform, wall, floor).
    pub fn spawn_static_rect(&mut self, position: Vec2, size: Vec2, color: Color) -> crate::object::ObjectId {
        self.objects.spawn_static_rect(position, size, color)
    }

    /// Spawn a sprite-only object (no physics).
    pub fn spawn_sprite(&mut self, desc: crate::object::SpriteDesc) -> crate::object::ObjectId {
        self.objects.spawn_sprite(desc)
    }

    /// Despawn any object.
    pub fn despawn(&mut self, id: crate::object::ObjectId) {
        self.objects.despawn(id);
    }

    // ── Light follow ──

    /// Make a point light follow an object's position each step.
    ///
    /// The light's position is set to the object's position after each
    /// physics step. This is an instant sync (no smoothing) to avoid
    /// shadow artifacts from position lag.
    ///
    /// Only point lights can follow objects (directional lights have no position).
    pub fn light_follow(&mut self, light: LightId, target: ObjectId) {
        self.light_follow_with_offset(light, target, Vec2::ZERO);
    }

    /// Like [`light_follow`](Self::light_follow), but with a fixed offset.
    pub fn light_follow_with_offset(&mut self, light: LightId, target: ObjectId, offset: Vec2) {
        if let Some(ft) = self.light_follow_targets.iter_mut().find(|ft| ft.light == light) {
            ft.object = target;
            ft.offset = offset;
        } else {
            self.light_follow_targets.push(LightFollowTarget { light, object: target, offset });
        }
    }

    /// Update the offset for an already-following light. No-op if not following.
    pub fn set_light_follow_offset(&mut self, light: LightId, offset: Vec2) {
        if let Some(ft) = self.light_follow_targets.iter_mut().find(|ft| ft.light == light) {
            ft.offset = offset;
        }
    }

    /// Stop a light from following any object.
    pub fn light_unfollow(&mut self, light: LightId) {
        self.light_follow_targets.retain(|ft| ft.light != light);
    }

    /// Update all light follow targets from current object positions.
    fn update_light_follows(&mut self) {
        for ft in &self.light_follow_targets {
            let pos = self.objects.get_position(ft.object);
            if let Some(light) = self.lighting.get_light_mut(ft.light) {
                light.position = pos + ft.offset;
            }
        }
    }

    // ── Simulation ──

    /// Advance the world by one timestep.
    ///
    /// Steps physics, then updates camera follow targets and light follow targets.
    pub fn step(&mut self, dt: f32) {
        self.objects.step(dt);
        self.cameras.update_follows(&self.objects);
        self.update_light_follows();
    }

    /// Snapshot physics state for interpolated rendering.
    pub fn snapshot_for_render(&mut self) {
        self.objects.snapshot_for_render();
    }

    // ── Rendering (internal) ──

    /// Merge object render commands into the default layer's command list.
    fn merge_objects_into_default_layer(&mut self) {
        let object_cmds = self.objects.render_commands_with_z();
        let layer = &mut self.render_layers[self.default_layer.0];
        layer.commands.extend(object_cmds);
    }

    /// Draw a single layer's commands sorted by z-order to the currently-bound target.
    fn render_layer_commands(
        renderer: &mut dyn Renderer<Error = String>,
        camera: &Camera,
        commands: &[(RenderCommand, i32)],
        clear_color: Option<Color>,
    ) {
        renderer.begin_frame(camera);
        if let Some(color) = clear_color {
            renderer.clear(color);
        }
        let mut sorted: Vec<_> = commands.to_vec();
        sorted.sort_by_key(|(_, z)| *z);
        for (cmd, _) in sorted {
            renderer.draw(cmd);
        }
        renderer.end_frame();
    }

    /// Create or resize the scene FBO to match the current screen size.
    fn ensure_scene_fbo(&mut self, renderer: &mut dyn Renderer<Error = String>) {
        let (w, h) = renderer.drawable_size();
        let (w, h) = (w as u32, h as u32);

        if self.scene_target.is_some() && self.scene_fbo_size == (w, h) {
            return;
        }

        // Destroy old FBO if size changed
        if let Some(target) = self.scene_target.take() {
            renderer.destroy_render_target(target);
        }
        if let Some(tex) = self.scene_texture.take() {
            renderer.destroy_texture(tex);
        }

        let (target, texture) = renderer
            .create_render_target(w, h)
            .expect("Failed to create scene render target");
        self.scene_target = Some(target);
        self.scene_texture = Some(texture);
        self.scene_fbo_size = (w, h);
    }

    /// Composite the scene FBO texture onto the currently-bound target with alpha blending.
    ///
    /// If `clear_color` is `Some`, the target is cleared first (within the same
    /// begin/end frame) to avoid flicker from a separate clear pass.
    fn composite_scene_fbo(
        renderer: &mut dyn Renderer<Error = String>,
        camera: &Camera,
        scene_texture: TextureId,
        clear_color: Option<Color>,
    ) {
        let (min_x, min_y, max_x, max_y) = camera.bounds();
        let cx = (min_x + max_x) / 2.0;
        let cy = (min_y + max_y) / 2.0;

        renderer.begin_frame(camera);
        if let Some(color) = clear_color {
            renderer.clear(color);
        }
        renderer.set_blend_mode(BlendMode::Alpha);
        // OpenGL FBOs need a V-flip (Y=0 at bottom); Metal does not (Y=0 at top).
        let uv = if renderer.fbo_origin_top_left() {
            [0.0, 0.0, 1.0, 1.0]
        } else {
            [0.0, 1.0, 1.0, 0.0]
        };
        renderer.draw(RenderCommand::Sprite(DrawSprite {
            texture: scene_texture,
            position: [cx, cy],
            size: [max_x - min_x, max_y - min_y],
            rotation: 0.0,
            uv,
            color: Color::WHITE,
        }));
        renderer.end_frame();
    }

    /// Returns true if any lit layer has commands queued.
    fn has_lit_content(&self) -> bool {
        self.render_layers.iter().any(|l| l.config.lit && !l.commands.is_empty())
    }

    /// Render all layers, grouping consecutive lit layers for shared FBO + lighting.
    ///
    /// - Unlit layers render directly to `output_target`.
    /// - Consecutive lit layers share the scene FBO; lighting is applied once
    ///   per group, then the result is composited onto `output_target`.
    fn render_layers(
        &mut self,
        renderer: &mut dyn Renderer<Error = String>,
        camera: &Camera,
        output_target: RenderTargetId,
    ) {
        let do_lighting = self.lighting.is_enabled() && self.lighting.has_lights();

        if do_lighting {
            let occluders = self.objects.collect_occluders();
            self.lighting.set_occluders(occluders);
            self.lighting.ensure_resources(renderer);
        }

        // Ensure scene FBO exists if any lit layers have content
        let need_fbo = self.has_lit_content();
        if need_fbo {
            self.ensure_scene_fbo(renderer);
        }

        let scene_target = self.scene_target;
        let scene_texture = self.scene_texture;

        // Track whether the output target has been cleared this pass.
        // The first layer to touch the output target must clear it with its
        // clear color so stale content from previous frames doesn't bleed through.
        let mut output_cleared = false;

        let layer_count = self.render_layers.len();
        let mut i = 0;
        while i < layer_count {
            let layer = &self.render_layers[i];

            if !layer.config.lit {
                // ── Unlit layer: render directly to output ──
                if !layer.commands.is_empty() {
                    renderer.bind_render_target(output_target);
                    Self::render_layer_commands(
                        renderer,
                        camera,
                        &layer.commands,
                        Some(layer.config.clear_color),
                    );
                } else {
                    // Empty unlit layer — still clear with its color
                    renderer.bind_render_target(output_target);
                    renderer.begin_frame(camera);
                    renderer.clear(layer.config.clear_color);
                    renderer.end_frame();
                }
                output_cleared = true;
                i += 1;
            } else {
                // ── Lit group: collect consecutive lit layers ──
                // If no scene FBO exists (no lit content this frame), skip the entire lit group
                let (fbo, tex) = match (scene_target, scene_texture) {
                    (Some(f), Some(t)) => (f, t),
                    _ => {
                        // Skip all consecutive lit layers
                        while i < layer_count && self.render_layers[i].config.lit {
                            i += 1;
                        }
                        continue;
                    }
                };
                let group_start = i;

                // First lit layer in group — clear the FBO
                renderer.bind_render_target(fbo);
                Self::render_layer_commands(
                    renderer,
                    camera,
                    &self.render_layers[i].commands,
                    Some(Color::TRANSPARENT),
                );
                i += 1;

                // Continue with consecutive lit layers (no clear)
                while i < layer_count && self.render_layers[i].config.lit {
                    if !self.render_layers[i].commands.is_empty() {
                        renderer.bind_render_target(fbo);
                        Self::render_layer_commands(
                            renderer,
                            camera,
                            &self.render_layers[i].commands,
                            None, // no clear — accumulate onto existing content
                        );
                    }
                    i += 1;
                }

                // Apply lighting to the scene FBO
                if do_lighting {
                    self.lighting.render_lightmap(renderer, camera);
                    renderer.bind_render_target(fbo);
                    self.lighting.composite_lightmap(renderer, camera);
                }

                // Composite lit scene FBO onto output.
                // If the output hasn't been cleared yet (no unlit layer preceded
                // this lit group), clear it with the first lit layer's clear color
                // to prevent stale content from previous frames bleeding through.
                renderer.bind_render_target(output_target);
                let clear = if !output_cleared {
                    output_cleared = true;
                    Some(self.render_layers[group_start].config.clear_color)
                } else {
                    None
                };
                // Only composite if group had content (avoid blank overlay)
                let group_has_content = (group_start..i)
                    .any(|j| !self.render_layers[j].commands.is_empty());
                if group_has_content {
                    Self::composite_scene_fbo(renderer, camera, tex, clear);
                } else if let Some(color) = clear {
                    // No content to composite, but still need to clear
                    renderer.begin_frame(camera);
                    renderer.clear(color);
                    renderer.end_frame();
                }
            }
        }
    }

    /// Clear all layer command lists.
    fn clear_layer_commands(&mut self) {
        for layer in &mut self.render_layers {
            layer.commands.clear();
        }
    }

    /// Draw unlit commands in world space (if any). Called after the lighting pass.
    fn render_unlit(&self, renderer: &mut dyn Renderer<Error = String>, camera: &Camera) {
        if self.unlit_commands.is_empty() {
            return;
        }

        renderer.begin_frame(camera);

        let mut sorted = self.unlit_commands.clone();
        sorted.sort_by_key(|(_, z)| *z);
        for (cmd, _) in sorted {
            renderer.draw(cmd);
        }

        renderer.end_frame();
    }

    /// Draw overlay commands in screen space (if any).
    fn render_overlays(&self, renderer: &mut dyn Renderer<Error = String>) {
        if self.overlay_commands.is_empty() {
            return;
        }

        let screen_camera = Camera {
            x: 0.5,
            y: 0.5,
            width: 1.0,
            height: 1.0,
            zoom: 1.0,
            rotation: 0.0,
        };

        renderer.begin_frame(&screen_camera);

        let mut sorted = self.overlay_commands.clone();
        sorted.sort_by_key(|(_, z)| *z);
        for (cmd, _) in sorted {
            renderer.draw(cmd);
        }

        renderer.end_frame();
    }

    // ── Rendering (public) ──

    /// Render all layers through the "main" camera to the currently-bound target.
    ///
    /// This is a convenience method for simple single-camera setups.
    /// For multi-camera rendering, use `render_to_targets()` instead.
    ///
    /// Layers are rendered in creation order. Consecutive lit layers share a
    /// single offscreen FBO with lighting composited; unlit layers render
    /// directly. After all layers: unlit commands, then overlay commands.
    pub fn auto_render(&mut self, renderer: &mut dyn Renderer<Error = String>) {
        profile_scope!("world.auto_render");
        // Fit camera viewport to screen aspect ratio (keeps height, adjusts width)
        let (sw, sh) = renderer.screen_size();
        if let Some(c) = self.cameras.get_mut(DEFAULT_CAMERA) {
            c.fit_to_screen(sw, sh);
        }
        let camera = match self.cameras.get(DEFAULT_CAMERA) {
            Some(c) => c.clone(),
            None => return,
        };

        // Merge object commands into the default layer
        self.merge_objects_into_default_layer();

        // Render all layers (handles lit/unlit grouping, FBO, lighting)
        self.render_layers(renderer, &camera, RenderTargetId::SCREEN);

        // Unlit pass (world-space, after all layers — not darkened by lightmap)
        self.render_unlit(renderer, &camera);

        // Overlay pass (screen-space, after all layers)
        self.render_overlays(renderer);

        // Clear all queued commands for next frame
        self.clear_layer_commands();
        self.unlit_commands.clear();
        self.overlay_commands.clear();
    }

    /// Render all layers through each named camera into its assigned render target.
    ///
    /// For each `(camera_name, target_id)` pair: renders all layers through
    /// that camera into the target. Use with `Engine::composite_layer()`
    /// to arrange outputs on screen.
    ///
    /// Layers, lighting, unlit commands, and overlays are included in each
    /// target and cleared after this call.
    pub fn render_to_targets(
        &mut self,
        renderer: &mut dyn Renderer<Error = String>,
        camera_targets: &[(&str, RenderTargetId)],
    ) {
        profile_scope!("world.render_to_targets");

        // Merge object commands into the default layer
        self.merge_objects_into_default_layer();

        // Fit all cameras to screen aspect ratio
        let (sw, sh) = renderer.screen_size();
        for &(cam_name, _) in camera_targets {
            if let Some(c) = self.cameras.get_mut(cam_name) {
                c.fit_to_screen(sw, sh);
            }
        }

        for &(cam_name, target_id) in camera_targets {
            let camera = match self.cameras.get(cam_name) {
                Some(c) => c.clone(),
                None => continue,
            };

            // Render all layers to this target
            self.render_layers(renderer, &camera, target_id);

            // Unlit pass for this target
            renderer.bind_render_target(target_id);
            self.render_unlit(renderer, &camera);

            // Overlay pass for this target
            renderer.bind_render_target(target_id);
            self.render_overlays(renderer);
        }

        // Clear all queued commands for next frame
        self.clear_layer_commands();
        self.unlit_commands.clear();
        self.overlay_commands.clear();
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl unison_ui::facade::OverlayTarget for World {
    fn draw_overlay(&mut self, command: RenderCommand, z_order: i32) {
        self.overlay_commands.push((command, z_order));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unison_math::Vec2;
    use unison_physics::mesh::create_ring_mesh;
    use crate::object::SoftBodyDesc;
    use unison_render::TextureId;

    #[test]
    fn world_new_has_defaults() {
        let world = World::new();
        assert!(world.cameras.get(DEFAULT_CAMERA).is_some());
        assert_eq!(world.background_color(), Color::BLACK);
        assert_eq!(world.objects.object_count(), 0);
    }

    // world_step_advances_physics moved to unison-tests crate.

    #[test]
    fn world_camera_follow_updates() {
        let mut world = World::new();
        let mesh = create_ring_mesh(1.0, 0.5, 8, 2);

        let id = world.objects.spawn_soft_body(SoftBodyDesc {
            mesh,
            material: unison_physics::Material::RUBBER,
            position: Vec2::new(10.0, 5.0),
            color: Color::WHITE,
            texture: TextureId::NONE,
        });

        world.cameras.follow(DEFAULT_CAMERA, id, 1.0); // instant snap
        world.step(1.0 / 60.0);

        let cam = world.cameras.get(DEFAULT_CAMERA).unwrap();
        // Camera should have moved toward the object
        assert!(cam.x.abs() > 0.0 || cam.y.abs() > 0.0);
    }

    #[test]
    fn set_background() {
        let mut world = World::new();
        let color = Color::from_hex(0x1a1a2e);
        world.set_background(color);
        assert_eq!(world.background_color().r, color.r);
    }

    #[test]
    fn draw_queues_to_default_layer() {
        let mut world = World::new();
        world.draw(RenderCommand::Rect {
            position: [0.0, 0.0],
            size: [1.0, 1.0],
            color: Color::RED,
        }, -100);
        world.draw(RenderCommand::Rect {
            position: [0.0, 0.0],
            size: [1.0, 1.0],
            color: Color::BLUE,
        }, 100);

        let layer = &world.render_layers[world.default_layer.0];
        assert_eq!(layer.commands.len(), 2);
    }

    #[test]
    fn draw_to_queues_to_specific_layer() {
        let mut world = World::new();
        let sky = world.create_render_layer("sky", RenderLayerConfig {
            lit: false,
            clear_color: Color::BLUE,
        });

        world.draw_to(sky, RenderCommand::Rect {
            position: [0.0, 0.0],
            size: [1.0, 1.0],
            color: Color::WHITE,
        }, 0);

        assert_eq!(world.render_layers[sky.0].commands.len(), 1);
        assert_eq!(world.render_layers[world.default_layer.0].commands.len(), 0);
    }

    #[test]
    fn create_render_layer_before() {
        let mut world = World::new();
        // Default layer is at index 0
        assert_eq!(world.default_layer.0, 0);
        assert_eq!(world.render_layers.len(), 1);

        // Insert sky layer before the default scene layer
        let sky = world.create_render_layer_before(
            "sky",
            RenderLayerConfig { lit: false, clear_color: Color::BLUE },
            world.default_layer(),
        );

        assert_eq!(sky.0, 0); // sky is now at index 0
        assert_eq!(world.default_layer.0, 1); // scene shifted to index 1
        assert_eq!(world.render_layers.len(), 2);
        assert!(!world.render_layers[0].config.lit); // sky = unlit
        assert!(world.render_layers[1].config.lit); // scene = lit
    }

    #[test]
    fn set_layer_clear_color() {
        let mut world = World::new();
        let sky = world.create_render_layer("sky", RenderLayerConfig {
            lit: false,
            clear_color: Color::BLACK,
        });

        world.set_layer_clear_color(sky, Color::BLUE);
        assert_eq!(world.render_layers[sky.0].config.clear_color.b, Color::BLUE.b);
    }

    #[test]
    fn draw_overlay_queues_commands() {
        let mut world = World::new();
        world.draw_overlay(RenderCommand::Rect {
            position: [0.1, 0.1],
            size: [0.2, 0.05],
            color: Color::WHITE,
        }, 0);

        assert_eq!(world.overlay_commands.len(), 1);
    }

    #[test]
    fn light_follow_updates_position() {
        let mut world = World::new();
        world.lighting.set_enabled(true);

        let obj = world.objects.spawn_soft_body(SoftBodyDesc {
            mesh: create_ring_mesh(1.0, 0.5, 8, 2),
            material: unison_physics::Material::RUBBER,
            position: Vec2::new(5.0, 3.0),
            color: Color::WHITE,
            texture: TextureId::NONE,
        });

        let light = world.lighting.add_light(unison_lighting::PointLight {
            position: Vec2::ZERO,
            color: Color::WHITE,
            intensity: 1.0,
            radius: 5.0,
            casts_shadows: false,
            shadow: Default::default(),
        });

        world.light_follow(light, obj);
        world.step(1.0 / 60.0);

        let light_pos = world.lighting.get_light(light).unwrap().position;
        let obj_pos = world.objects.get_position(obj);
        assert!((light_pos.x - obj_pos.x).abs() < 0.01);
        assert!((light_pos.y - obj_pos.y).abs() < 0.01);
    }

    #[test]
    fn light_follow_with_offset() {
        let mut world = World::new();
        world.lighting.set_enabled(true);

        let obj = world.objects.spawn_soft_body(SoftBodyDesc {
            mesh: create_ring_mesh(1.0, 0.5, 8, 2),
            material: unison_physics::Material::RUBBER,
            position: Vec2::new(5.0, 3.0),
            color: Color::WHITE,
            texture: TextureId::NONE,
        });

        let light = world.lighting.add_light(unison_lighting::PointLight {
            position: Vec2::ZERO,
            color: Color::WHITE,
            intensity: 1.0,
            radius: 5.0,
            casts_shadows: false,
            shadow: Default::default(),
        });

        world.light_follow_with_offset(light, obj, Vec2::new(0.0, 2.0));
        world.step(1.0 / 60.0);

        let light_pos = world.lighting.get_light(light).unwrap().position;
        let obj_pos = world.objects.get_position(obj);
        assert!((light_pos.x - obj_pos.x).abs() < 0.01);
        assert!((light_pos.y - (obj_pos.y + 2.0)).abs() < 0.01);
    }

    #[test]
    fn light_unfollow_stops_tracking() {
        let mut world = World::new();
        world.lighting.set_enabled(true);

        let light = world.lighting.add_light(unison_lighting::PointLight {
            position: Vec2::new(99.0, 99.0),
            color: Color::WHITE,
            intensity: 1.0,
            radius: 5.0,
            casts_shadows: false,
            shadow: Default::default(),
        });

        let obj = world.objects.spawn_soft_body(SoftBodyDesc {
            mesh: create_ring_mesh(1.0, 0.5, 8, 2),
            material: unison_physics::Material::RUBBER,
            position: Vec2::new(5.0, 3.0),
            color: Color::WHITE,
            texture: TextureId::NONE,
        });

        world.light_follow(light, obj);
        world.light_unfollow(light);
        world.step(1.0 / 60.0);

        // Light should still be at its original position
        let light_pos = world.lighting.get_light(light).unwrap().position;
        assert!((light_pos.x - 99.0).abs() < 0.01);
        assert!((light_pos.y - 99.0).abs() < 0.01);
    }
}
