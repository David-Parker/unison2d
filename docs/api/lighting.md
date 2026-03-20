# unison-lighting

2D lighting with lightmap compositing.

Renders point lights to an offscreen FBO (the "lightmap"), then composites it over the scene with multiply blending. Unlit areas are darkened to the ambient color; lit areas are tinted by the light's color and intensity.

## How it works

1. **Lightmap FBO** — cleared to the ambient color each frame
2. **Additive light pass** — each point light is drawn as a radial gradient sprite with additive blending
3. **Multiply composite** — the lightmap is drawn over the scene with multiply blending, darkening unlit areas

## Types

### `PointLight`

A point light that emits in all directions with radial falloff.

```rust
pub struct PointLight {
    pub position: Vec2,   // World-space position
    pub color: Color,     // Light color
    pub intensity: f32,   // Multiplier applied to color
    pub radius: f32,      // Radius of influence in world units
}

impl PointLight {
    pub fn new(position: Vec2, color: Color, intensity: f32, radius: f32) -> Self;
}
```

### `LightId`

Opaque handle returned by `add_light`. Used to query, mutate, or remove a light.

### `LightingSystem`

Manages lights, ambient color, and the lightmap FBO.

```rust
impl LightingSystem {
    pub fn new() -> Self;

    // Ambient
    pub fn set_ambient(&mut self, color: Color);
    pub fn ambient(&self) -> Color;

    // Enable/disable
    pub fn set_enabled(&mut self, enabled: bool);
    pub fn is_enabled(&self) -> bool;

    // Light management
    pub fn add_light(&mut self, light: PointLight) -> LightId;
    pub fn remove_light(&mut self, id: LightId);
    pub fn get_light(&self, id: LightId) -> Option<&PointLight>;
    pub fn get_light_mut(&mut self, id: LightId) -> Option<&mut PointLight>;
    pub fn light_count(&self) -> usize;
    pub fn clear_lights(&mut self);

    // Rendering (called by World automatically)
    pub fn ensure_resources(&mut self, renderer: &mut dyn Renderer<Error = String>);
    pub fn render_lightmap(&self, renderer: &mut dyn Renderer<Error = String>, camera: &Camera);
    pub fn composite_lightmap(&self, renderer: &mut dyn Renderer<Error = String>, camera: &Camera);
    pub fn lightmap_texture(&self) -> Option<TextureId>;
}
```

## Usage

Lighting is integrated into `World`. Enable it, set an ambient color, and add lights:

```rust
// In level setup
world.lighting.set_ambient(Color::new(0.1, 0.1, 0.15, 1.0));
world.lighting.set_enabled(true);

let light = world.lighting.add_light(PointLight::new(
    Vec2::new(5.0, 3.0),
    Color::new(1.0, 0.9, 0.7, 1.0),  // warm white
    1.0,
    6.0,
));

// In update — move the light
let pos = world.objects.get_position(player_id);
if let Some(l) = world.lighting.get_light_mut(light) {
    l.position = pos;
}
```

`World::auto_render` and `World::render_to_targets` handle the lightmap rendering and compositing automatically when lighting is enabled and at least one light exists.

## Gradient texture

Point lights are rendered using a 64×64 radial gradient texture generated at runtime. The falloff is quadratic: `alpha = 1 - dist²`. The `gradient` module exposes `generate_radial_gradient(size)` for testing.

## Dependencies

- `unison-math` — Vec2, Color
- `unison-render` — Renderer trait, BlendMode, Camera, RenderCommand
