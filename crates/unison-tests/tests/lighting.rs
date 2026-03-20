//! Integration tests for the lighting system.

use unison_lighting::{DirectionalLight, LightingSystem, PointLight};
use unison_lighting::gradient::generate_radial_gradient;
use unison_math::{Color, Vec2};
use unison_render::TextureFormat;

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

// ── World integration ──

#[test]
fn world_has_lighting() {
    let world = unison2d::World::new();
    assert_eq!(world.lighting.light_count(), 0);
    assert!(!world.lighting.is_enabled());
}
