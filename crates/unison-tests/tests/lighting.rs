//! Integration tests for the lighting system.

use std::cell::RefCell;
use std::rc::Rc;
use unison_lighting::{DirectionalLight, LightingSystem, PointLight, ShadowFilter};
use unison_lighting::gradient::generate_radial_gradient;
use unison_math::{Color, Vec2};
use unison_render::{
    BlendMode, Camera, DrawLitSprite, RenderCommand, RenderTargetId, Renderer,
    TextureDescriptor, TextureFormat, TextureId,
};

// ── Mock renderer for e2e tests ──

/// Record of a render operation for inspection.
#[derive(Debug, Clone)]
enum RenderOp {
    BindTarget(RenderTargetId),
    BeginFrame { camera_x: f32, camera_y: f32 },
    Clear(Color),
    SetBlend(BlendMode),
    DrawSprite { position: [f32; 2], size: [f32; 2], texture: TextureId },
    DrawLitSprite {
        position: [f32; 2],
        size: [f32; 2],
        texture: TextureId,
        shadow_mask: TextureId,
        screen_size: (f32, f32),
        shadow_filter: u32,
    },
    DrawMesh { vertex_count: usize, color: Color },
    EndFrame,
}

/// A mock renderer that records all operations for test inspection.
struct MockRenderer {
    ops: Rc<RefCell<Vec<RenderOp>>>,
    next_texture_id: u32,
    next_rt_id: u32,
    screen_w: f32,
    screen_h: f32,
}

impl MockRenderer {
    fn new(w: f32, h: f32) -> Self {
        Self {
            ops: Rc::new(RefCell::new(Vec::new())),
            next_texture_id: 1,
            next_rt_id: 1,
            screen_w: w,
            screen_h: h,
        }
    }

    fn ops(&self) -> Vec<RenderOp> {
        self.ops.borrow().clone()
    }

    fn clear_ops(&self) {
        self.ops.borrow_mut().clear();
    }
}

impl Renderer for MockRenderer {
    type Error = String;

    fn init(&mut self) -> Result<(), String> { Ok(()) }

    fn begin_frame(&mut self, camera: &Camera) {
        self.ops.borrow_mut().push(RenderOp::BeginFrame {
            camera_x: camera.x,
            camera_y: camera.y,
        });
    }

    fn clear(&mut self, color: Color) {
        self.ops.borrow_mut().push(RenderOp::Clear(color));
    }

    fn draw(&mut self, command: RenderCommand) {
        let op = match command {
            RenderCommand::Sprite(s) => RenderOp::DrawSprite {
                position: s.position,
                size: s.size,
                texture: s.texture,
            },
            RenderCommand::LitSprite(l) => RenderOp::DrawLitSprite {
                position: l.position,
                size: l.size,
                texture: l.texture,
                shadow_mask: l.shadow_mask,
                screen_size: l.screen_size,
                shadow_filter: l.shadow_filter,
            },
            RenderCommand::Mesh(m) => RenderOp::DrawMesh {
                vertex_count: m.positions.len() / 2,
                color: m.color,
            },
            _ => return,
        };
        self.ops.borrow_mut().push(op);
    }

    fn end_frame(&mut self) {
        self.ops.borrow_mut().push(RenderOp::EndFrame);
    }

    fn create_texture(&mut self, _desc: &TextureDescriptor) -> Result<TextureId, String> {
        let id = self.next_texture_id;
        self.next_texture_id += 1;
        Ok(TextureId(id))
    }

    fn destroy_texture(&mut self, _id: TextureId) {}

    fn screen_size(&self) -> (f32, f32) {
        (self.screen_w, self.screen_h)
    }

    fn set_blend_mode(&mut self, mode: BlendMode) {
        self.ops.borrow_mut().push(RenderOp::SetBlend(mode));
    }

    fn create_render_target(&mut self, _w: u32, _h: u32) -> Result<(RenderTargetId, TextureId), String> {
        let rt = self.next_rt_id;
        self.next_rt_id += 1;
        let tex = self.next_texture_id;
        self.next_texture_id += 1;
        Ok((RenderTargetId(rt), TextureId(tex)))
    }

    fn bind_render_target(&mut self, target: RenderTargetId) {
        self.ops.borrow_mut().push(RenderOp::BindTarget(target));
    }

    fn destroy_render_target(&mut self, _target: RenderTargetId) {}
}

// ── LightingSystem defaults ──

#[test]
fn lighting_system_defaults() {
    let sys = LightingSystem::new();
    assert_eq!(sys.light_count(), 0);
    assert!(!sys.is_enabled());
    assert_eq!(sys.ambient().r, Color::BLACK.r);
    assert_eq!(sys.ambient().g, Color::BLACK.g);
    assert_eq!(sys.ambient().b, Color::BLACK.b);
    assert!(sys.lightmap_texture().is_none());
}

// ── Add / remove lights ──

#[test]
fn lighting_system_add_remove() {
    let mut sys = LightingSystem::new();

    let id1 = sys.add_light(PointLight::new(
        Vec2::new(1.0, 2.0),
        Color::WHITE,
        1.0,
        5.0,
    ));
    assert_eq!(sys.light_count(), 1);

    let id2 = sys.add_light(PointLight::new(
        Vec2::new(3.0, 4.0),
        Color::RED,
        0.5,
        3.0,
    ));
    assert_eq!(sys.light_count(), 2);

    // Verify we can get lights back
    let light1 = sys.get_light(id1).expect("light1 should exist");
    assert_eq!(light1.position.x, 1.0);
    assert_eq!(light1.position.y, 2.0);
    assert_eq!(light1.radius, 5.0);

    let light2 = sys.get_light(id2).expect("light2 should exist");
    assert_eq!(light2.position.x, 3.0);

    // Remove first light
    sys.remove_light(id1);
    assert_eq!(sys.light_count(), 1);
    assert!(sys.get_light(id1).is_none());
    assert!(sys.get_light(id2).is_some());

    // Remove second light
    sys.remove_light(id2);
    assert_eq!(sys.light_count(), 0);
}

#[test]
fn lighting_system_remove_after_clear_is_noop() {
    let mut sys = LightingSystem::new();
    let id = sys.add_light(PointLight::new(Vec2::ZERO, Color::WHITE, 1.0, 5.0));
    sys.clear_lights();
    sys.remove_light(id); // should not panic even though already cleared
    assert_eq!(sys.light_count(), 0);
}

// ── Mutate lights ──

#[test]
fn lighting_system_get_mut() {
    let mut sys = LightingSystem::new();
    let id = sys.add_light(PointLight::new(
        Vec2::new(0.0, 0.0),
        Color::WHITE,
        1.0,
        5.0,
    ));

    // Mutate position
    let light = sys.get_light_mut(id).expect("should exist");
    light.position = Vec2::new(10.0, 20.0);
    light.intensity = 2.0;

    // Verify mutation persisted
    let light = sys.get_light(id).expect("should still exist");
    assert_eq!(light.position.x, 10.0);
    assert_eq!(light.position.y, 20.0);
    assert_eq!(light.intensity, 2.0);
}

// ── Clear lights ──

#[test]
fn lighting_system_clear() {
    let mut sys = LightingSystem::new();
    for i in 0..5 {
        sys.add_light(PointLight::new(
            Vec2::new(i as f32, 0.0),
            Color::WHITE,
            1.0,
            3.0,
        ));
    }
    assert_eq!(sys.light_count(), 5);

    sys.clear_lights();
    assert_eq!(sys.light_count(), 0);
}

// ── Ambient ──

#[test]
fn lighting_system_ambient() {
    let mut sys = LightingSystem::new();
    let ambient = Color::new(0.1, 0.2, 0.3, 1.0);
    sys.set_ambient(ambient);

    let got = sys.ambient();
    assert_eq!(got.r, ambient.r);
    assert_eq!(got.g, ambient.g);
    assert_eq!(got.b, ambient.b);
}

// ── Enabled flag ──

#[test]
fn lighting_system_enabled() {
    let mut sys = LightingSystem::new();
    assert!(!sys.is_enabled());

    sys.set_enabled(true);
    assert!(sys.is_enabled());

    sys.set_enabled(false);
    assert!(!sys.is_enabled());
}

// ── Gradient texture ──

#[test]
fn gradient_texture_valid() {
    let desc = generate_radial_gradient(64);
    assert_eq!(desc.width, 64);
    assert_eq!(desc.height, 64);
    assert_eq!(desc.format, TextureFormat::Rgba8);
    assert_eq!(desc.data.len(), 64 * 64 * 4);
}

#[test]
fn gradient_center_bright_edge_dark() {
    let size = 64u32;
    let desc = generate_radial_gradient(size);

    // Center pixel should be fully bright (alpha = 255)
    let center = size / 2;
    let center_idx = ((center * size + center) * 4) as usize;
    assert_eq!(desc.data[center_idx], 255, "center R should be 255");
    assert_eq!(desc.data[center_idx + 1], 255, "center G should be 255");
    assert_eq!(desc.data[center_idx + 2], 255, "center B should be 255");
    // Alpha at center should be near max (quadratic falloff at dist=0 is 1.0)
    assert!(desc.data[center_idx + 3] > 250, "center alpha should be near 255");

    // Corner pixel (0,0) should be dark (far from center)
    let corner_idx = 0usize;
    assert_eq!(desc.data[corner_idx + 3], 0, "corner alpha should be 0");
}

// ── Directional lights ──

#[test]
fn directional_light_add_get() {
    let mut sys = LightingSystem::new();

    let id = sys.add_directional_light(DirectionalLight::new(
        Vec2::new(0.0, -1.0),
        Color::WHITE,
        1.0,
    ));
    assert_eq!(sys.directional_light_count(), 1);

    let light = sys.get_directional_light(id).expect("should exist");
    assert_eq!(light.direction.y, -1.0);
    assert_eq!(light.intensity, 1.0);
}

#[test]
fn directional_light_add_remove() {
    let mut sys = LightingSystem::new();

    let id1 = sys.add_directional_light(DirectionalLight::new(
        Vec2::new(0.0, -1.0),
        Color::WHITE,
        1.0,
    ));
    let id2 = sys.add_directional_light(DirectionalLight::new(
        Vec2::new(1.0, 0.0),
        Color::RED,
        0.5,
    ));
    assert_eq!(sys.directional_light_count(), 2);

    sys.remove_directional_light(id1);
    assert_eq!(sys.directional_light_count(), 1);
    assert!(sys.get_directional_light(id1).is_none());
    assert!(sys.get_directional_light(id2).is_some());

    sys.remove_directional_light(id2);
    assert_eq!(sys.directional_light_count(), 0);
}

#[test]
fn directional_light_mutate() {
    let mut sys = LightingSystem::new();
    let id = sys.add_directional_light(DirectionalLight::new(
        Vec2::new(0.0, -1.0),
        Color::WHITE,
        1.0,
    ));

    let light = sys.get_directional_light_mut(id).expect("should exist");
    light.intensity = 2.0;
    light.color = Color::new(0.5, 0.5, 1.0, 1.0);

    let light = sys.get_directional_light(id).expect("should still exist");
    assert_eq!(light.intensity, 2.0);
    assert_eq!(light.color.b, 1.0);
}

#[test]
fn directional_light_clear() {
    let mut sys = LightingSystem::new();
    for _ in 0..3 {
        sys.add_directional_light(DirectionalLight::new(
            Vec2::new(0.0, -1.0),
            Color::WHITE,
            1.0,
        ));
    }
    assert_eq!(sys.directional_light_count(), 3);

    sys.clear_directional_lights();
    assert_eq!(sys.directional_light_count(), 0);
}

#[test]
fn has_lights_mixed() {
    let mut sys = LightingSystem::new();
    assert!(!sys.has_lights());

    // Add only a point light
    let p = sys.add_light(PointLight::new(Vec2::ZERO, Color::WHITE, 1.0, 5.0));
    assert!(sys.has_lights());

    // Add a directional light
    let d = sys.add_directional_light(DirectionalLight::new(
        Vec2::new(0.0, -1.0),
        Color::WHITE,
        1.0,
    ));
    assert!(sys.has_lights());

    // Remove point light — still has directional
    sys.remove_light(p);
    assert!(sys.has_lights());

    // Remove directional — now empty
    sys.remove_directional_light(d);
    assert!(!sys.has_lights());
}

// ── Shadow types ──

#[test]
fn shadow_filter_uniform_values() {
    use unison_lighting::ShadowFilter;
    assert_eq!(ShadowFilter::None.as_uniform_value(), 0);
    assert_eq!(ShadowFilter::Pcf5.as_uniform_value(), 5);
    assert_eq!(ShadowFilter::Pcf13.as_uniform_value(), 13);
}

#[test]
fn shadow_filter_default_is_none() {
    use unison_lighting::ShadowFilter;
    let filter: ShadowFilter = Default::default();
    assert_eq!(filter, ShadowFilter::None);
}

#[test]
fn point_light_shadow_fields_default() {
    let light = PointLight::new(Vec2::ZERO, Color::WHITE, 1.0, 5.0);
    assert!(!light.casts_shadows);
    assert_eq!(light.shadow_filter, unison_lighting::ShadowFilter::None);
}

#[test]
fn directional_light_shadow_fields_default() {
    let light = DirectionalLight::new(Vec2::new(0.0, -1.0), Color::WHITE, 1.0);
    assert!(!light.casts_shadows);
    assert_eq!(light.shadow_filter, unison_lighting::ShadowFilter::None);
}

// ── Occluder construction ──

#[test]
fn occluder_from_aabb() {
    use unison_lighting::Occluder;
    let occ = Occluder::from_aabb(0.0, 0.0, 1.0, 1.0);
    assert_eq!(occ.edges.len(), 4);

    // Check normals point outward
    let normals: Vec<[f32; 2]> = occ.edges.iter().map(|e| e.normal).collect();
    assert!(normals.contains(&[0.0, -1.0])); // bottom
    assert!(normals.contains(&[1.0, 0.0]));  // right
    assert!(normals.contains(&[0.0, 1.0]));  // top
    assert!(normals.contains(&[-1.0, 0.0])); // left
}

#[test]
fn occluder_from_ground() {
    use unison_lighting::Occluder;
    let occ = Occluder::from_ground(-5.0, -10.0, 10.0);
    assert_eq!(occ.edges.len(), 1);
    let edge = &occ.edges[0];
    assert_eq!(edge.a[1], -5.0);
    assert_eq!(edge.b[1], -5.0);
    assert_eq!(edge.normal, [0.0, -1.0]); // points down (casts shadow below)
}

#[test]
fn occluder_from_boundary_edges() {
    use unison_lighting::Occluder;
    // Simple square: 4 vertices, 2 triangles
    let positions = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
    let boundary = vec![(0, 1), (1, 2), (2, 3), (3, 0)];
    let occ = Occluder::from_boundary_edges(&positions, &boundary);
    assert_eq!(occ.edges.len(), 4);
}

// ── Shadow projection ──

#[test]
fn back_facing_point_test() {
    use unison_lighting::shadow::is_back_facing_point;
    use unison_lighting::OccluderEdge;

    // Edge with normal pointing right, light to the left
    let edge = OccluderEdge {
        a: [0.0, -1.0],
        b: [0.0, 1.0],
        normal: [1.0, 0.0], // points right
    };

    // Light is to the left → normal points away from light → back-facing
    assert!(is_back_facing_point(&edge, [-5.0, 0.0]));
    // Light is to the right → normal points toward light → front-facing
    assert!(!is_back_facing_point(&edge, [5.0, 0.0]));
}

#[test]
fn back_facing_directional_test() {
    use unison_lighting::shadow::is_back_facing_directional;
    use unison_lighting::OccluderEdge;

    let edge = OccluderEdge {
        a: [0.0, -1.0],
        b: [0.0, 1.0],
        normal: [1.0, 0.0], // points right
    };

    // Light shining to the right → normal aligns → back-facing
    assert!(is_back_facing_directional(&edge, [1.0, 0.0]));
    // Light shining to the left → normal opposes → front-facing
    assert!(!is_back_facing_directional(&edge, [-1.0, 0.0]));
}

#[test]
fn project_point_shadows_basic() {
    use unison_lighting::shadow::project_point_shadows;
    use unison_lighting::Occluder;

    // AABB at origin, light above → bottom edge is back-facing
    let occ = Occluder::from_aabb(0.0, 0.0, 1.0, 1.0);
    let quads = project_point_shadows([0.0, 5.0], 10.0, &[occ]);

    // Light above should shadow from bottom edge (and side edges partially)
    assert!(!quads.is_empty(), "should have shadow quads");

    // All shadow quads should have valid positions (no NaN)
    for quad in &quads {
        for &v in &quad.positions {
            assert!(v.is_finite(), "shadow vertex should be finite");
        }
    }
}

#[test]
fn project_directional_shadows_basic() {
    use unison_lighting::shadow::project_directional_shadows;
    use unison_lighting::Occluder;

    let occ = Occluder::from_aabb(0.0, 0.0, 1.0, 1.0);
    // Light shining downward → top edge normal (up) aligns with direction → back-facing
    let quads = project_directional_shadows([0.0, -1.0], 20.0, &[occ]);

    assert!(!quads.is_empty(), "should have shadow quads from directional light");
    for quad in &quads {
        for &v in &quad.positions {
            assert!(v.is_finite(), "shadow vertex should be finite");
        }
    }
}

#[test]
fn project_point_shadows_no_occluders() {
    use unison_lighting::shadow::project_point_shadows;
    let quads = project_point_shadows([0.0, 0.0], 10.0, &[]);
    assert!(quads.is_empty());
}

// ── Boundary edge computation ──

#[test]
fn compute_boundary_edges_square() {
    use unison_lighting::compute_boundary_edges;
    // Two triangles forming a square: shared diagonal is NOT a boundary edge
    let triangles = [0, 1, 2, 0, 2, 3];
    let boundary = compute_boundary_edges(&triangles);
    // 4 boundary edges (the outer edges of the square)
    assert_eq!(boundary.len(), 4);
    // The shared edge (0,2) should NOT be in the boundary
    assert!(!boundary.contains(&(0, 2)));
}

#[test]
fn compute_boundary_edges_ring_mesh() {
    use unison_lighting::compute_boundary_edges;
    use unison_physics::mesh::create_ring_mesh;

    let mesh = create_ring_mesh(1.0, 0.25, 16, 6);
    let boundary = compute_boundary_edges(&mesh.triangles);

    // A ring has two boundary loops: inner and outer
    // Inner: 16 edges, outer: 16 edges = 32 total
    assert_eq!(boundary.len(), 32, "ring should have 32 boundary edges (inner + outer)");
}

// ── LightingSystem shadow methods ──

#[test]
fn lighting_system_set_occluders() {
    use unison_lighting::Occluder;

    let mut sys = LightingSystem::new();
    let occluders = vec![
        Occluder::from_aabb(0.0, 0.0, 1.0, 1.0),
        Occluder::from_aabb(5.0, 0.0, 2.0, 1.0),
    ];
    sys.set_occluders(occluders);
    // No panic; occluders stored internally
}

#[test]
fn lighting_system_ground_shadow() {
    let mut sys = LightingSystem::new();
    sys.set_ground_shadow(Some(-5.0));
    sys.set_ground_shadow(None);
    // No panic
}

// ── World integration ──

#[test]
fn world_has_lighting() {
    let world = unison2d::World::new();
    assert_eq!(world.lighting.light_count(), 0);
    assert!(!world.lighting.is_enabled());
}

#[test]
fn world_collect_occluders() {
    let mut world = unison2d::World::new();

    // Spawn a static rect — should produce an occluder
    world.spawn_static_rect(
        Vec2::new(0.0, -5.0),
        Vec2::new(10.0, 2.0),
        Color::WHITE,
    );

    let occluders = world.objects.collect_occluders();
    assert_eq!(occluders.len(), 1, "should have 1 occluder from the static rect");
    assert_eq!(occluders[0].edges.len(), 4, "AABB should have 4 edges");
}

#[test]
fn world_set_casts_shadow() {
    let mut world = unison2d::World::new();

    let id = world.spawn_static_rect(
        Vec2::new(0.0, -5.0),
        Vec2::new(10.0, 2.0),
        Color::WHITE,
    );

    assert!(world.objects.casts_shadow(id));
    assert_eq!(world.objects.collect_occluders().len(), 1);

    world.objects.set_casts_shadow(id, false);
    assert!(!world.objects.casts_shadow(id));
    assert_eq!(world.objects.collect_occluders().len(), 0);
}

// ── Shadow geometry correctness (diagnostic) ──

#[test]
fn point_shadow_projects_away_from_light() {
    use unison_lighting::shadow::project_point_shadows;
    use unison_lighting::Occluder;

    // Light above an AABB at origin. The bottom edge (normal=[0,-1]) is back-facing
    // because its normal points away from the light (which is above).
    let occ = Occluder::from_aabb(0.0, 0.0, 1.0, 1.0);
    let light_pos = [0.0, 5.0];
    let quads = project_point_shadows(light_pos, 10.0, &[occ]);

    // Find the shadow quad from the bottom edge (both original vertices have y = -1)
    let bottom_quads: Vec<_> = quads
        .iter()
        .filter(|q| {
            // Original vertices (first two) should both be at y = -1
            let ay = q.positions[1];
            let by = q.positions[3];
            (ay - (-1.0)).abs() < 0.01 && (by - (-1.0)).abs() < 0.01
        })
        .collect();
    assert!(
        !bottom_quads.is_empty(),
        "bottom edge should produce a shadow quad"
    );

    // Projected vertices (last two) should be BELOW the original vertices
    // since the light is above — shadow extends downward
    for quad in &bottom_quads {
        let a_proj_y = quad.positions[7]; // a' y
        let b_proj_y = quad.positions[5]; // b' y
        assert!(
            a_proj_y < -1.0,
            "projected a' should be below original edge, got y={}",
            a_proj_y
        );
        assert!(
            b_proj_y < -1.0,
            "projected b' should be below original edge, got y={}",
            b_proj_y
        );
    }
}

#[test]
fn point_shadow_correct_back_face_selection() {
    use unison_lighting::shadow::{is_back_facing_point, project_point_shadows};
    use unison_lighting::Occluder;

    // AABB at (0,0) with half-extents (1,1)
    let occ = Occluder::from_aabb(0.0, 0.0, 1.0, 1.0);

    // Light directly above at (0, 5):
    // - Bottom normal [0,-1] → to_light = [0, 6] → dot = -6 → back-facing ✓
    // - Top normal [0,1] → to_light = [0, 4] → dot = 4 → front-facing ✓
    // - Left normal [-1,0] → to_light depends on midpoint
    // - Right normal [1,0] → to_light depends on midpoint
    let light_above = [0.0, 5.0];

    let mut back_normals: Vec<[f32; 2]> = Vec::new();
    for edge in &occ.edges {
        if is_back_facing_point(edge, light_above) {
            back_normals.push(edge.normal);
        }
    }

    // Bottom edge should definitely be back-facing
    assert!(
        back_normals.contains(&[0.0, -1.0]),
        "bottom edge should be back-facing for light above"
    );
    // Top edge should NOT be back-facing
    assert!(
        !back_normals.contains(&[0.0, 1.0]),
        "top edge should be front-facing for light above"
    );

    // Verify shadow count matches back-facing count
    let quads = project_point_shadows(light_above, 10.0, &[occ]);
    assert_eq!(
        quads.len(),
        back_normals.len(),
        "one shadow quad per back-facing edge"
    );
}

#[test]
fn directional_shadow_projects_along_direction() {
    use unison_lighting::shadow::project_directional_shadows;
    use unison_lighting::Occluder;

    // Light shining downward [0, -1]. Top edge normal is [0, 1], which
    // aligns with direction (dot = -1 > 0 is false... let me check).
    // is_back_facing_directional: dot(normal, direction) > 0
    // Top normal [0,1] · direction [0,-1] = -1 → NOT back-facing
    // Bottom normal [0,-1] · direction [0,-1] = 1 → back-facing ✓
    //
    // This means for downward light, the BOTTOM edge casts shadow — shadow
    // extends downward below the object. This is correct: downward-traveling
    // light hits the top surface; shadow appears below.
    let occ = Occluder::from_aabb(0.0, 0.0, 1.0, 1.0);
    let quads = project_directional_shadows([0.0, -1.0], 20.0, &[occ]);

    for quad in &quads {
        // Projected vertices should be below the original vertices (direction is downward)
        let a_proj_y = quad.positions[7];
        let b_proj_y = quad.positions[5];
        let a_orig_y = quad.positions[1];
        let b_orig_y = quad.positions[3];
        assert!(
            a_proj_y < a_orig_y,
            "projected a' should be below original for downward light"
        );
        assert!(
            b_proj_y < b_orig_y,
            "projected b' should be below original for downward light"
        );
    }
}

#[test]
fn ground_occluder_blocks_light_below() {
    use unison_lighting::shadow::{is_back_facing_point, project_point_shadows};
    use unison_lighting::Occluder;

    // Ground at y=-5, light above at y=3
    let ground = Occluder::from_ground(-5.0, -20.0, 20.0);
    let light_pos = [0.0, 3.0];

    // Ground normal is [0, -1] (down). For a light above:
    // to_light from ground midpoint = [0, 8], dot(normal, to_light) = -8 < 0 → back-facing
    // This means ground DOES cast shadow for light above it — prevents light bleed below ground.
    for edge in &ground.edges {
        assert!(
            is_back_facing_point(edge, light_pos),
            "ground should be back-facing for light above it (casts shadow below)"
        );
    }

    let quads = project_point_shadows(light_pos, 10.0, &[ground.clone()]);
    assert!(
        !quads.is_empty(),
        "ground should cast shadow below for light above it"
    );

    // Light below ground at y=-8 → ground should NOT cast shadow (normal points down,
    // same direction as to_light, so front-facing → no shadow)
    let light_below = [0.0, -8.0];
    for edge in &ground.edges {
        assert!(
            !is_back_facing_point(edge, light_below),
            "ground should be front-facing for light below it"
        );
    }
    let quads_below = project_point_shadows(light_below, 10.0, &[ground]);
    assert!(
        quads_below.is_empty(),
        "ground should not cast shadow for light below it"
    );
}

#[test]
fn shadow_quad_vertex_winding() {
    use unison_lighting::shadow::project_point_shadows;
    use unison_lighting::Occluder;

    // Verify that shadow quads form valid convex quadrilaterals
    let occ = Occluder::from_aabb(0.0, 0.0, 1.0, 1.0);
    let quads = project_point_shadows([0.0, 5.0], 10.0, &[occ]);

    for (i, quad) in quads.iter().enumerate() {
        // Quad has 4 vertices: A, B, B', A'
        // Indices: [0,1,2, 0,2,3]
        // This forms two triangles: (A,B,B') and (A,B',A')
        // Both triangles should have non-zero area (not degenerate)
        let ax = quad.positions[0];
        let ay = quad.positions[1];
        let bx = quad.positions[2];
        let by = quad.positions[3];
        let bpx = quad.positions[4];
        let bpy = quad.positions[5];
        let apx = quad.positions[6];
        let apy = quad.positions[7];

        // Triangle 1: (A, B, B')
        let area1 = (bx - ax) * (bpy - ay) - (bpx - ax) * (by - ay);
        assert!(
            area1.abs() > 1e-6,
            "shadow quad {} triangle 1 should have non-zero area, got {}",
            i,
            area1
        );

        // Triangle 2: (A, B', A')
        let area2 = (bpx - ax) * (apy - ay) - (apx - ax) * (bpy - ay);
        assert!(
            area2.abs() > 1e-6,
            "shadow quad {} triangle 2 should have non-zero area, got {}",
            i,
            area2
        );
    }
}

#[test]
fn soft_body_occluder_boundary_edges_correct() {
    use unison_lighting::{Occluder, compute_boundary_edges};
    use unison_physics::mesh::create_ring_mesh;

    let mesh = create_ring_mesh(1.0, 0.25, 16, 6);
    let boundary = compute_boundary_edges(&mesh.triangles);

    // Build occluder from these boundary edges
    let occ = Occluder::from_boundary_edges(&mesh.vertices, &boundary);

    // Every edge should have a unit-length normal
    for edge in &occ.edges {
        let len = (edge.normal[0] * edge.normal[0] + edge.normal[1] * edge.normal[1]).sqrt();
        assert!(
            (len - 1.0).abs() < 0.01,
            "edge normal should be unit length, got {}",
            len
        );
    }

    // For a ring centered at origin, outer boundary normals should point outward
    // (away from center) and inner boundary normals should point inward (toward center).
    // We can verify this by checking that the dot product of the normal with the
    // vector from the center to the edge midpoint has the expected sign.
    let mut outward_count = 0;
    let mut inward_count = 0;
    for edge in &occ.edges {
        let mid_x = (edge.a[0] + edge.b[0]) * 0.5;
        let mid_y = (edge.a[1] + edge.b[1]) * 0.5;
        let dist = (mid_x * mid_x + mid_y * mid_y).sqrt();
        if dist < 0.01 {
            continue;
        }
        // Vector from center to midpoint
        let to_mid = [mid_x / dist, mid_y / dist];
        let dot = edge.normal[0] * to_mid[0] + edge.normal[1] * to_mid[1];
        if dot > 0.0 {
            outward_count += 1;
        } else {
            inward_count += 1;
        }
    }
    // Ring with 16 segments: 16 outer edges (outward normals) + 16 inner edges (inward normals).
    // All normals must be consistent — no flips from winding order errors.
    assert_eq!(
        outward_count, 16,
        "ring should have exactly 16 outward (outer) normals, got {} (inward={})",
        outward_count, inward_count
    );
    assert_eq!(
        inward_count, 16,
        "ring should have exactly 16 inward (inner) normals, got {} (outward={})",
        inward_count, outward_count
    );
}

// ── E2E rendering pipeline tests ──

/// Reproduce the exact game scenario: one shadow-casting point light, one AABB occluder.
/// Verify that render_lightmap produces the correct sequence of operations.
#[test]
fn e2e_render_lightmap_shadow_point_light() {
    use unison_lighting::Occluder;

    let mut renderer = MockRenderer::new(800.0, 600.0);
    let mut sys = LightingSystem::new();
    sys.set_ambient(Color::new(0.05, 0.05, 0.08, 1.0));
    sys.set_enabled(true);

    // Add a shadow-casting point light (like the donut light)
    let _light_id = sys.add_light(PointLight {
        position: Vec2::new(0.0, 3.0),
        color: Color::new(1.0, 0.9, 0.7, 1.0),
        intensity: 1.0,
        radius: 6.0,
        casts_shadows: true,
        shadow_filter: ShadowFilter::Pcf5,
    });

    // Add an occluder (a platform)
    sys.set_occluders(vec![Occluder::from_aabb(0.0, -3.0, 2.5, 0.25)]);
    sys.set_ground_shadow(Some(-4.5));

    // Create resources (lightmap FBO, shadow mask FBO, gradient texture)
    sys.ensure_resources(&mut renderer);

    let camera = Camera::new(20.0, 15.0);
    renderer.clear_ops();

    // Render the lightmap
    sys.render_lightmap(&mut renderer, &camera);

    let ops = renderer.ops();

    // Verify the operation sequence:
    // 1. Bind lightmap FBO
    // 2. BeginFrame
    // 3. Clear to ambient
    // 4. Set blend Additive
    // 5. (for shadow light): Bind shadow mask FBO, BeginFrame, Clear white,
    //    SetBlend Alpha, draw shadow meshes, EndFrame
    // 6. Bind lightmap FBO, BeginFrame, SetBlend Additive, DrawLitSprite
    // 7. SetBlend Alpha, EndFrame

    // Find the DrawLitSprite operation
    let lit_sprite_ops: Vec<_> = ops.iter().filter(|op| {
        matches!(op, RenderOp::DrawLitSprite { .. })
    }).collect();
    assert_eq!(
        lit_sprite_ops.len(), 1,
        "should have exactly 1 DrawLitSprite for the shadow-casting point light"
    );

    // Verify the lit sprite has the correct size (radius * 2 = 12)
    if let RenderOp::DrawLitSprite { position, size, screen_size, shadow_filter, texture, shadow_mask } = &lit_sprite_ops[0] {
        assert_eq!(*size, [12.0, 12.0], "lit sprite size should be radius*2 = 12");
        assert_eq!(*position, [0.0, 3.0], "lit sprite should be at light position");
        assert_eq!(*screen_size, (800.0, 600.0), "screen_size should match renderer");
        assert_eq!(*shadow_filter, 5, "PCF5 should pass filter value 5");
        assert!(texture.is_valid(), "point light should have a gradient texture");
        assert!(shadow_mask.is_valid(), "should have a shadow mask texture");
    }

    // Verify shadow mask was rendered (should have DrawMesh ops with BLACK color)
    let shadow_mesh_ops: Vec<_> = ops.iter().filter(|op| {
        matches!(op, RenderOp::DrawMesh { color, .. } if color.r == 0.0 && color.g == 0.0 && color.b == 0.0)
    }).collect();
    assert!(
        !shadow_mesh_ops.is_empty(),
        "should have shadow mesh draws (black quads on shadow mask)"
    );

    // Verify the sequence has bind_target calls: lightmap, shadow_mask, lightmap
    let bind_ops: Vec<_> = ops.iter().filter_map(|op| {
        if let RenderOp::BindTarget(id) = op { Some(*id) } else { None }
    }).collect();
    assert!(
        bind_ops.len() >= 3,
        "should bind lightmap, shadow mask, lightmap again. Got {} binds: {:?}",
        bind_ops.len(),
        bind_ops
    );
    // First bind = lightmap, second = shadow mask, third = back to lightmap
    assert_ne!(bind_ops[0], bind_ops[1], "first two binds should be different targets");
    assert_eq!(bind_ops[0], bind_ops[2], "third bind should return to lightmap");
}

/// Test that a non-shadow-casting point light uses a regular Sprite, not LitSprite.
#[test]
fn e2e_render_lightmap_no_shadow_uses_sprite() {
    let mut renderer = MockRenderer::new(800.0, 600.0);
    let mut sys = LightingSystem::new();
    sys.set_ambient(Color::new(0.05, 0.05, 0.08, 1.0));
    sys.set_enabled(true);

    // Non-shadow point light
    sys.add_light(PointLight::new(Vec2::new(0.0, 3.0), Color::WHITE, 1.0, 6.0));

    sys.ensure_resources(&mut renderer);
    let camera = Camera::new(20.0, 15.0);
    renderer.clear_ops();
    sys.render_lightmap(&mut renderer, &camera);

    let ops = renderer.ops();

    // Should have a DrawSprite, NOT a DrawLitSprite
    let sprite_ops: Vec<_> = ops.iter().filter(|op| matches!(op, RenderOp::DrawSprite { .. })).collect();
    let lit_ops: Vec<_> = ops.iter().filter(|op| matches!(op, RenderOp::DrawLitSprite { .. })).collect();
    assert_eq!(sprite_ops.len(), 1, "should draw one regular sprite");
    assert_eq!(lit_ops.len(), 0, "should not draw any lit sprites");

    // Verify sprite size
    if let RenderOp::DrawSprite { size, .. } = &sprite_ops[0] {
        assert_eq!(*size, [12.0, 12.0], "sprite size should be radius*2 = 12");
    }
}

/// Test the composite step: verify it uses multiply blending and the correct UVs.
#[test]
fn e2e_composite_lightmap() {
    let mut renderer = MockRenderer::new(800.0, 600.0);
    let mut sys = LightingSystem::new();
    sys.set_ambient(Color::WHITE);
    sys.set_enabled(true);
    sys.add_light(PointLight::new(Vec2::ZERO, Color::WHITE, 1.0, 5.0));

    sys.ensure_resources(&mut renderer);
    let camera = Camera::new(20.0, 15.0);

    renderer.clear_ops();
    sys.composite_lightmap(&mut renderer, &camera);

    let ops = renderer.ops();

    // Should: BeginFrame, SetBlend(Multiply), DrawSprite(lightmap tex), SetBlend(Alpha), EndFrame
    let blend_ops: Vec<_> = ops.iter().filter_map(|op| {
        if let RenderOp::SetBlend(mode) = op { Some(*mode) } else { None }
    }).collect();
    assert!(
        blend_ops.contains(&BlendMode::Multiply),
        "composite should use Multiply blending"
    );

    // Should draw the lightmap texture as a full-viewport sprite
    let sprite_ops: Vec<_> = ops.iter().filter(|op| matches!(op, RenderOp::DrawSprite { .. })).collect();
    assert_eq!(sprite_ops.len(), 1, "composite should draw one sprite");

    if let RenderOp::DrawSprite { size, texture, .. } = &sprite_ops[0] {
        assert!(texture.is_valid(), "composite should use lightmap texture");
        // Size should be camera bounds (20, 15)
        assert_eq!(size[0], 20.0, "composite width should match camera width");
        assert_eq!(size[1], 15.0, "composite height should match camera height");
    }
}

/// Reproduce EXACT game lighting setup: directional (no shadow) + point (shadow) + occluders.
/// Trace the full render sequence to find the bug.
#[test]
fn e2e_exact_game_lighting_setup() {
    use unison_lighting::{DirectionalLight, Occluder};

    let mut renderer = MockRenderer::new(800.0, 600.0);
    let mut sys = LightingSystem::new();

    // Exact game setup from shared.rs new_world()
    sys.set_ambient(Color::new(0.05, 0.05, 0.08, 1.0));
    sys.set_enabled(true);
    sys.set_ground_shadow(Some(-4.5));

    // Directional light (moonlight) — added first, NO shadows
    sys.add_directional_light(DirectionalLight::new(
        Vec2::new(0.3, -1.0),
        Color::new(0.2, 0.2, 0.35, 1.0),
        1.0,
    ));

    // Point light (donut light) — added second, WITH shadows
    sys.add_light(PointLight {
        position: Vec2::new(0.0, 3.0),
        color: Color::new(1.0, 0.9, 0.7, 1.0),
        intensity: 1.0,
        radius: 6.0,
        casts_shadows: true,
        shadow_filter: ShadowFilter::Pcf5,
    });

    // Occluders: ground platform + trigger box (like main_level)
    sys.set_occluders(vec![
        Occluder::from_aabb(0.0, -5.5, 15.0, 1.0),  // ground
        Occluder::from_aabb(6.0, -3.0, 1.0, 1.0),    // trigger box
    ]);

    sys.ensure_resources(&mut renderer);
    let camera = Camera::new(20.0, 15.0);
    renderer.clear_ops();
    sys.render_lightmap(&mut renderer, &camera);

    let ops = renderer.ops();
    eprintln!("\n=== Game lighting render sequence ({} ops) ===", ops.len());
    for (i, op) in ops.iter().enumerate() {
        eprintln!("  [{:2}] {:?}", i, op);
    }
    eprintln!("=== End ===\n");

    // Verify we have both a LitSprite (shadow point light) AND a regular Sprite (directional)
    let lit_sprites: Vec<_> = ops.iter().enumerate().filter(|(_, op)| matches!(op, RenderOp::DrawLitSprite { .. })).collect();
    let sprites: Vec<_> = ops.iter().enumerate().filter(|(_, op)| matches!(op, RenderOp::DrawSprite { .. })).collect();

    assert_eq!(lit_sprites.len(), 1, "exactly 1 LitSprite for point light with shadows");
    assert_eq!(sprites.len(), 1, "exactly 1 Sprite for directional light without shadows");

    // Verify the LitSprite has correct parameters
    if let (_, RenderOp::DrawLitSprite { position, size, texture, shadow_mask, .. }) = &lit_sprites[0] {
        assert_eq!(*position, [0.0, 3.0], "point light at donut position");
        assert_eq!(*size, [12.0, 12.0], "size = radius*2 = 12");
        assert!(texture.is_valid(), "point light needs gradient texture");
        assert!(shadow_mask.is_valid(), "point light needs shadow mask");
    }

    // KEY: Verify the directional light sprite is drawn AFTER the shadow path completes.
    // If the shadow path leaves state dirty, the directional sprite could be affected.
    let lit_idx = lit_sprites[0].0;
    let sprite_idx = sprites[0].0;
    eprintln!("LitSprite at op index {}, directional Sprite at op index {}", lit_idx, sprite_idx);

    // Check that between the shadow mask rendering and the LitSprite draw,
    // we properly re-bind the lightmap and set additive blending
    let shadow_end_frame_idx = ops.iter().enumerate()
        .position(|(i, op)| matches!(op, RenderOp::EndFrame) && i > 3 && i < lit_idx)
        .expect("should have EndFrame for shadow mask pass");

    eprintln!("Shadow mask EndFrame at op index {}", shadow_end_frame_idx);

    // After shadow EndFrame, verify: BindTarget(lightmap), BeginFrame, SetBlend(Additive)
    let post_shadow_ops = &ops[shadow_end_frame_idx + 1..lit_idx];
    eprintln!("Ops between shadow EndFrame and LitSprite:");
    for (i, op) in post_shadow_ops.iter().enumerate() {
        eprintln!("  [{}] {:?}", shadow_end_frame_idx + 1 + i, op);
    }

    assert!(
        post_shadow_ops.iter().any(|op| matches!(op, RenderOp::BindTarget(_))),
        "must rebind lightmap after shadow mask render"
    );
    assert!(
        post_shadow_ops.iter().any(|op| matches!(op, RenderOp::SetBlend(BlendMode::Additive))),
        "must restore additive blend after shadow mask render"
    );
}

// ── Software shader simulator tests ──
// These test the exact logic of the GLSL lit fragment shader in CPU code
// to catch bugs like double-application of shadow, wrong UV mapping, etc.

/// Simulate the lit fragment shader for a single pixel.
///
/// Returns (r, g, b, a) of frag_color.
fn simulate_lit_shader(
    gradient_alpha: f32,     // alpha from the gradient texture at v_uv
    u_color: [f32; 4],      // light color * intensity
    shadow_value: f32,       // shadow mask sample at this pixel
    use_texture: bool,       // whether gradient texture is bound
) -> [f32; 4] {
    let light = if use_texture {
        // texture(u_texture, v_uv) = (1.0, 1.0, 1.0, gradient_alpha) for our gradient
        [
            1.0 * u_color[0],
            1.0 * u_color[1],
            1.0 * u_color[2],
            gradient_alpha * u_color[3],
        ]
    } else {
        u_color
    };

    // frag_color = vec4(light.rgb * shadow, light.a)
    // Shadow only affects RGB, not alpha — prevents double-application
    // through additive blending (SRC_ALPHA, ONE).
    [
        light[0] * shadow_value,
        light[1] * shadow_value,
        light[2] * shadow_value,
        light[3], // alpha unchanged by shadow
    ]
}

/// With additive blending (SRC_ALPHA, ONE), compute the contribution
/// to the lightmap. Returns (r, g, b) contribution added to destination.
fn additive_blend_contribution(frag_color: [f32; 4]) -> [f32; 3] {
    // src.rgb * src.a + dst.rgb * 1
    // The contribution (what gets added to dst) is src.rgb * src.a
    [
        frag_color[0] * frag_color[3],
        frag_color[1] * frag_color[3],
        frag_color[2] * frag_color[3],
    ]
}

#[test]
fn shader_lit_vs_unlit_in_unshadowed_area() {
    // Compare lit shader output to base shader output when shadow=1.0 (fully lit)
    // They should produce the same additive blend contribution.

    let gradient_alpha = 0.8; // Mid-distance from center
    let u_color = [1.0, 0.9, 0.7, 1.0]; // Warm light

    // Base shader path: frag_color = texture(gradient, uv) * u_color
    let base_frag = [
        1.0 * u_color[0],
        1.0 * u_color[1],
        1.0 * u_color[2],
        gradient_alpha * u_color[3],
    ];
    let base_contribution = additive_blend_contribution(base_frag);

    // Lit shader path with shadow=1.0 (no shadow)
    let lit_frag = simulate_lit_shader(gradient_alpha, u_color, 1.0, true);
    let lit_contribution = additive_blend_contribution(lit_frag);

    // Should be identical!
    for i in 0..3 {
        assert!(
            (base_contribution[i] - lit_contribution[i]).abs() < 1e-6,
            "channel {} should match: base={} lit={}",
            i,
            base_contribution[i],
            lit_contribution[i]
        );
    }
}

#[test]
fn shader_shadow_applied_once_not_twice() {
    // Verify that shadow is applied only once with additive blending.
    //
    // With additive blending (SRC_ALPHA, ONE), the contribution is:
    //   src.rgb * src.a + dst.rgb
    //
    // The shader outputs: frag_color = vec4(light.rgb * shadow, light.a)
    // So the contribution is: light.rgb * shadow * light.a
    // Shadow appears exactly once — correct!
    //
    // Previously, the shader did: frag_color = light * shadow
    // Which gave: light.rgb * shadow * (light.a * shadow) = rgb * alpha * shadow²
    // Shadow appeared twice — the bug that caused "tiny cone" lighting.

    let gradient_alpha = 0.8;
    let u_color = [1.0, 0.9, 0.7, 1.0];
    let shadow = 0.5; // Partial shadow (PCF edge)

    let lit_frag = simulate_lit_shader(gradient_alpha, u_color, shadow, true);
    let contribution = additive_blend_contribution(lit_frag);

    // The base shader contribution without shadow would be:
    let base_frag = [1.0 * u_color[0], 1.0 * u_color[1], 1.0 * u_color[2], gradient_alpha * u_color[3]];
    let base_contribution = additive_blend_contribution(base_frag);

    // With the fix, contribution should be base_contribution * shadow (applied once)
    let actual_factor = contribution[0] / base_contribution[0];

    eprintln!("Shadow = {}", shadow);
    eprintln!("Actual factor: {} (should equal shadow)", actual_factor);

    assert!(
        (actual_factor - shadow).abs() < 1e-6,
        "shadow should be applied exactly once. Expected factor={}, got {}",
        shadow,
        actual_factor
    );
}

#[test]
fn shader_shadow_mask_uv_should_not_flip_y() {
    // Verify that shadow mask UV calculation doesn't need a Y-flip
    // when both the lightmap (render target) and shadow mask are FBOs.
    //
    // In OpenGL FBOs:
    // - gl_FragCoord origin is bottom-left of the FBO
    // - Texture UV origin is also bottom-left
    // - So gl_FragCoord.xy / fbo_size maps directly to texture UV without flip

    let screen_w = 800.0;
    let screen_h = 600.0;

    // Fragment at bottom-left of FBO
    let frag_coord = (0.5, 0.5);
    let shadow_uv = (frag_coord.0 / screen_w, frag_coord.1 / screen_h);
    assert!(
        shadow_uv.0 < 0.01 && shadow_uv.1 < 0.01,
        "bottom-left fragment should map to bottom-left UV, got ({}, {})",
        shadow_uv.0,
        shadow_uv.1
    );

    // Fragment at top-right of FBO
    let frag_coord = (799.5, 599.5);
    let shadow_uv = (frag_coord.0 / screen_w, frag_coord.1 / screen_h);
    assert!(
        shadow_uv.0 > 0.99 && shadow_uv.1 > 0.99,
        "top-right fragment should map to top-right UV, got ({}, {})",
        shadow_uv.0,
        shadow_uv.1
    );

    // With incorrect Y-flip: shadow_uv.y = 1.0 - shadow_uv.y
    let frag_coord = (400.0, 300.0); // center
    let correct_uv = (frag_coord.0 / screen_w, frag_coord.1 / screen_h);
    let flipped_uv = (frag_coord.0 / screen_w, 1.0 - frag_coord.1 / screen_h);
    // At center, both map to (0.5, 0.5), so center is unaffected
    assert!(((correct_uv.0 - flipped_uv.0) as f64).abs() < 0.01);
    assert!(((correct_uv.1 - flipped_uv.1) as f64).abs() < 0.01);

    // But at top of screen (y=550), the flip maps to bottom of shadow mask
    let frag_coord = (400.0, 550.0); // near top
    let correct_uv_y = frag_coord.1 / screen_h; // ~0.917
    let flipped_uv_y = 1.0 - frag_coord.1 / screen_h; // ~0.083
    assert!(
        ((correct_uv_y - flipped_uv_y) as f64).abs() > 0.5,
        "Y-flip creates a major UV error for off-center pixels: \
         correct={:.3}, flipped={:.3}",
        correct_uv_y,
        flipped_uv_y
    );
}

#[test]
fn shader_directional_light_no_texture() {
    // Directional lights use TextureId::NONE (u_use_texture = false)
    // The shader should output vec4(u_color.rgb * shadow, u_color.a)
    let u_color = [0.2, 0.2, 0.35, 1.0]; // moonlight color
    let shadow = 0.8;

    let frag = simulate_lit_shader(0.0, u_color, shadow, false);
    // Without texture: light = u_color
    // frag_color = vec4(u_color.rgb * shadow, u_color.a)
    assert_eq!(frag[0], u_color[0] * shadow);
    assert_eq!(frag[1], u_color[1] * shadow);
    assert_eq!(frag[2], u_color[2] * shadow);
    assert_eq!(frag[3], u_color[3]); // alpha NOT affected by shadow
}

// ── Pixel-level lightmap simulation tests ──
// Simulate what the lightmap FBO would contain at specific world positions.

/// Compute the gradient texture alpha at a given world position for a point light.
/// Returns 0.0-1.0 (0 = outside light radius, 1 = at light center).
fn point_light_gradient_alpha(light_pos: [f32; 2], light_radius: f32, world_pos: [f32; 2]) -> f32 {
    let dx = world_pos[0] - light_pos[0];
    let dy = world_pos[1] - light_pos[1];
    let dist = (dx * dx + dy * dy).sqrt();
    let normalized_dist = dist / light_radius;
    if normalized_dist >= 1.0 {
        return 0.0;
    }
    // Quadratic falloff: 1 - dist²
    (1.0 - normalized_dist * normalized_dist).max(0.0)
}

/// Check if a world position is in shadow from a point light,
/// given a set of occluders.
fn is_in_point_shadow(
    light_pos: [f32; 2],
    world_pos: [f32; 2],
    occluders: &[unison_lighting::Occluder],
) -> bool {
    use unison_lighting::shadow::{is_back_facing_point, project_point_shadows};

    // Project shadows and check if world_pos is inside any shadow quad
    let quads = project_point_shadows(light_pos, 100.0, occluders); // large radius to get all shadows

    for quad in &quads {
        // Check point-in-quad using cross product winding test
        let vertices = [
            [quad.positions[0], quad.positions[1]], // A
            [quad.positions[2], quad.positions[3]], // B
            [quad.positions[4], quad.positions[5]], // B'
            [quad.positions[6], quad.positions[7]], // A'
        ];
        if point_in_quad(world_pos, &vertices) {
            return true;
        }
    }
    false
}

/// Point-in-convex-polygon test (cross product winding).
fn point_in_quad(point: [f32; 2], vertices: &[[f32; 2]; 4]) -> bool {
    let n = vertices.len();
    let mut positive = 0;
    let mut negative = 0;
    for i in 0..n {
        let j = (i + 1) % n;
        let dx = vertices[j][0] - vertices[i][0];
        let dy = vertices[j][1] - vertices[i][1];
        let px = point[0] - vertices[i][0];
        let py = point[1] - vertices[i][1];
        let cross = dx * py - dy * px;
        if cross > 0.0 { positive += 1; }
        if cross < 0.0 { negative += 1; }
    }
    positive == 0 || negative == 0
}

/// Compute the lightmap contribution of a point light at a world position,
/// accounting for gradient falloff and shadow occlusion.
/// Returns (r, g, b) additive contribution.
fn point_light_contribution(
    light: &PointLight,
    world_pos: [f32; 2],
    occluders: &[unison_lighting::Occluder],
) -> [f32; 3] {
    let gradient_alpha = point_light_gradient_alpha(
        [light.position.x, light.position.y],
        light.radius,
        world_pos,
    );
    if gradient_alpha <= 0.0 {
        return [0.0, 0.0, 0.0];
    }

    let shadow = if light.casts_shadows && !occluders.is_empty() {
        if is_in_point_shadow([light.position.x, light.position.y], world_pos, occluders) {
            0.0
        } else {
            1.0
        }
    } else {
        1.0
    };

    // Matches the shader: light = gradient * u_color, then * shadow (RGB only)
    // With additive blend: contribution = light.rgb * shadow * gradient_alpha
    let r = light.color.r * light.intensity * shadow * gradient_alpha;
    let g = light.color.g * light.intensity * shadow * gradient_alpha;
    let b = light.color.b * light.intensity * shadow * gradient_alpha;
    [r, g, b]
}

#[test]
fn point_light_emits_in_all_directions() {
    // Test that the point light at (0,3) illuminates positions in all 4 directions.
    let light = PointLight {
        position: Vec2::new(0.0, 3.0),
        color: Color::new(1.0, 0.9, 0.7, 1.0),
        intensity: 1.0,
        radius: 6.0,
        casts_shadows: false,
        shadow_filter: ShadowFilter::None,
    };

    let no_occluders: Vec<unison_lighting::Occluder> = vec![];

    // Sample positions at 3 world units from light center in each direction
    let positions = [
        ([3.0, 3.0], "right"),
        ([-3.0, 3.0], "left"),
        ([0.0, 6.0], "above"),
        ([0.0, 0.0], "below"),
        ([2.0, 5.0], "upper-right"),
        ([-2.0, 1.0], "lower-left"),
    ];

    for (pos, dir) in &positions {
        let contrib = point_light_contribution(&light, *pos, &no_occluders);
        assert!(
            contrib[0] > 0.0 && contrib[1] > 0.0 && contrib[2] > 0.0,
            "light should illuminate {} at {:?}, got contribution {:?}",
            dir, pos, contrib
        );
    }

    // Sample position far outside the radius should have zero contribution
    let far_contrib = point_light_contribution(&light, [20.0, 3.0], &no_occluders);
    assert_eq!(far_contrib, [0.0, 0.0, 0.0], "no light far outside radius");
}

#[test]
fn point_light_shadow_creates_dark_area_behind_box() {
    use unison_lighting::Occluder;

    // Point light at (0, 5), box at (0, 0) with half-extents (1, 1).
    // Shadow should appear BELOW the box (away from light).
    let light = PointLight {
        position: Vec2::new(0.0, 5.0),
        color: Color::new(1.0, 1.0, 1.0, 1.0),
        intensity: 1.0,
        radius: 12.0,
        casts_shadows: true,
        shadow_filter: ShadowFilter::None,
    };
    let occluders = vec![Occluder::from_aabb(0.0, 0.0, 1.0, 1.0)];

    // Point directly above box (inside box) — NOT in shadow from the box
    // (light comes from above, hits the top face)
    let above_box = [0.0, 2.0];
    let above_contrib = point_light_contribution(&light, above_box, &occluders);
    assert!(
        above_contrib[0] > 0.0,
        "point above box should be lit: {:?}",
        above_contrib
    );

    // Point directly below box — should be in shadow
    let below_box = [0.0, -3.0];
    let below_contrib = point_light_contribution(&light, below_box, &occluders);
    assert_eq!(
        below_contrib, [0.0, 0.0, 0.0],
        "point below box should be in shadow"
    );

    // Point to the side (not blocked) — should be lit
    let side = [5.0, 0.0];
    let side_contrib = point_light_contribution(&light, side, &occluders);
    assert!(
        side_contrib[0] > 0.0,
        "point to the side should be lit: {:?}",
        side_contrib
    );
}

#[test]
fn point_light_shadow_matches_game_scenario() {
    use unison_lighting::Occluder;

    // Exact game scenario: donut light at (0, 3), ground at y=-4.5,
    // platform at (-5, -2) with half-extents (2.5, 0.25).
    let light = PointLight {
        position: Vec2::new(0.0, 3.0),
        color: Color::new(1.0, 0.9, 0.7, 1.0),
        intensity: 1.0,
        radius: 6.0,
        casts_shadows: true,
        shadow_filter: ShadowFilter::Pcf5,
    };

    let occluders = vec![
        Occluder::from_aabb(0.0, -5.5, 15.0, 1.0),  // ground platform
        Occluder::from_aabb(6.0, -3.0, 1.0, 1.0),    // trigger box
    ];

    // Near the donut (light center) — should be well-lit
    let near_donut = [0.0, 3.0];
    let near_contrib = point_light_contribution(&light, near_donut, &occluders);
    eprintln!("Near donut contribution: {:?}", near_contrib);
    assert!(
        near_contrib[0] > 0.5,
        "near donut should be brightly lit"
    );

    // Midway between light and ground — should be lit (not blocked)
    // Point at (0, 0) is 3 units from light center, well within radius 6
    let midway = [0.0, 0.0];
    let midway_contrib = point_light_contribution(&light, midway, &occluders);
    eprintln!("Midway contribution: {:?}", midway_contrib);
    assert!(
        midway_contrib[0] > 0.0,
        "midway point should be lit"
    );

    // Below ground (y=-6) — should be in shadow from ground
    let below_ground = [0.0, -7.0];
    let below_contrib = point_light_contribution(&light, below_ground, &occluders);
    eprintln!("Below ground contribution: {:?}", below_contrib);
    // Below ground AND outside radius (distance = 10 > radius 6), so zero regardless
    assert_eq!(
        below_contrib, [0.0, 0.0, 0.0],
        "below ground should have no light"
    );

    // Behind the trigger box (x=8, y=-3) relative to light at (0,3)
    let behind_trigger = [8.0, -5.0];
    let behind_contrib = point_light_contribution(&light, behind_trigger, &occluders);
    eprintln!("Behind trigger box contribution: {:?}", behind_contrib);
    // This point is ~10 units from light, outside radius 6
    assert_eq!(
        behind_contrib, [0.0, 0.0, 0.0],
        "far behind trigger box should have no light (outside radius)"
    );
}

#[test]
fn shadow_creates_detectable_dark_edge() {
    use unison_lighting::Occluder;

    // Simple test: light at (0, 5), single box at (3, 0).
    // Sample a line of points at y=-2. Points behind the box (from light's POV)
    // should be dark; points not behind the box should be lit.
    let light = PointLight {
        position: Vec2::new(0.0, 5.0),
        color: Color::WHITE,
        intensity: 1.0,
        radius: 15.0,
        casts_shadows: true,
        shadow_filter: ShadowFilter::None,
    };
    let occluders = vec![Occluder::from_aabb(3.0, 0.0, 1.0, 1.0)];

    // Sample points at y=-2, from x=-5 to x=10
    let mut lit_count = 0;
    let mut shadow_count = 0;

    for ix in -50..100 {
        let x = ix as f32 / 10.0;
        let pos = [x, -2.0];
        let contrib = point_light_contribution(&light, pos, &occluders);
        if contrib[0] > 0.01 {
            lit_count += 1;
        } else {
            shadow_count += 1;
        }
    }

    eprintln!("At y=-2: {} lit samples, {} shadow samples", lit_count, shadow_count);
    assert!(lit_count > 0, "some points should be lit");
    assert!(shadow_count > 0, "some points should be in shadow");
    assert!(
        lit_count > shadow_count,
        "more points should be lit than shadowed (box is small)"
    );

    // Specifically: x=3.0, y=-2 is directly below the box center —
    // should be in shadow (bottom edge of box is back-facing from light above)
    let directly_below = point_light_contribution(&light, [3.0, -2.0], &occluders);
    assert_eq!(
        directly_below, [0.0, 0.0, 0.0],
        "directly below box should be in shadow"
    );

    // x=-2.0, y=-2 is to the left — should be lit (not behind box from light's POV)
    let to_the_left = point_light_contribution(&light, [-2.0, -2.0], &occluders);
    assert!(
        to_the_left[0] > 0.0,
        "to the left should be lit: {:?}",
        to_the_left
    );
}

/// Full World rendering pipeline test — reproduces the donut game setup.
#[test]
fn e2e_world_auto_render_with_shadows() {
    use unison_lighting::Occluder;

    let mut world = unison2d::World::new();
    world.set_background(Color::from_hex(0x1a1a2e));
    world.objects.set_gravity(Vec2::new(0.0, -9.8));
    world.objects.set_ground(-4.5);

    // Add a platform (creates an occluder)
    world.spawn_static_rect(
        Vec2::new(0.0, -5.5),
        Vec2::new(30.0, 2.0),
        Color::from_hex(0x2d5016),
    );

    // Enable lighting with shadow-casting point light
    world.lighting.set_ambient(Color::new(0.05, 0.05, 0.08, 1.0));
    world.lighting.set_enabled(true);
    world.lighting.set_ground_shadow(Some(-4.5));

    let _light = world.lighting.add_light(PointLight {
        position: Vec2::new(0.0, 3.0),
        color: Color::new(1.0, 0.9, 0.7, 1.0),
        intensity: 1.0,
        radius: 6.0,
        casts_shadows: true,
        shadow_filter: ShadowFilter::Pcf5,
    });

    // Add directional moonlight (no shadows for simplicity)
    world.lighting.add_directional_light(DirectionalLight::new(
        Vec2::new(0.3, -1.0),
        Color::new(0.2, 0.2, 0.35, 1.0),
        1.0,
    ));

    let mut renderer = MockRenderer::new(800.0, 600.0);

    // Render the world
    world.auto_render(&mut renderer);

    let ops = renderer.ops();

    // Verify the full pipeline executed:
    // 1. Scene render (BeginFrame, Clear, draws, EndFrame)
    // 2. Lighting (render_lightmap with shadows, then composite)
    let begin_count = ops.iter().filter(|op| matches!(op, RenderOp::BeginFrame { .. })).count();
    let end_count = ops.iter().filter(|op| matches!(op, RenderOp::EndFrame)).count();

    // At minimum: scene begin/end + lightmap begin/end + shadow mask begin/end + composite begin/end
    assert!(
        begin_count >= 4,
        "should have at least 4 begin_frame calls (scene, lightmap, shadow, composite), got {}",
        begin_count
    );
    assert!(
        end_count >= 4,
        "should have at least 4 end_frame calls, got {}",
        end_count
    );

    // Verify we have both a LitSprite (shadow point light) and a regular Sprite (directional)
    let lit_count = ops.iter().filter(|op| matches!(op, RenderOp::DrawLitSprite { .. })).count();
    let sprite_count = ops.iter().filter(|op| matches!(op, RenderOp::DrawSprite { .. })).count();

    assert!(
        lit_count >= 1,
        "should have at least 1 LitSprite for shadow-casting point light, got {}",
        lit_count
    );
    // Directional light (no shadow) + composite overlay = at least 2 sprites
    assert!(
        sprite_count >= 2,
        "should have at least 2 sprites (directional light + composite), got {}",
        sprite_count
    );
}
