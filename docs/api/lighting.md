# unison-lighting

2D lighting with lightmap compositing and shadow casting.

Renders point lights and directional lights to an offscreen FBO (the "lightmap"), then composites it over lit render layers with multiply blending. Unlit areas are darkened to the ambient color; lit areas are tinted by the light's color and intensity. Shadow-casting lights use a per-light shadow mask with optional PCF filtering for soft edges. Unlit render layers (e.g. sky) are not affected by the lightmap.

## How it works

1. **Lightmap FBO** — cleared to the ambient color each frame
2. **Additive light pass** — each light is drawn additively to the lightmap:
   - **Without shadows:** point lights as radial gradient sprites, directional lights as full-viewport quads
   - **With shadows:** occluder geometry is projected into shadow quads, rendered to a shadow mask FBO (white=lit, black=shadow), then the light is drawn as a `LitSprite` that samples both the gradient and shadow mask with optional PCF
3. **Multiply composite** — the lightmap is drawn over lit render layers with multiply blending (unlit layers are not affected)

## Types

### `ShadowSettings`

Shadow appearance configuration shared by all light types. Controls PCF filtering, shadow darkness, fade distance, and fade curve.

```rust
pub struct ShadowSettings {
    pub filter: ShadowFilter,   // PCF mode for shadow edges (default: None)
    pub strength: f32,          // Shadow darkness: 0.0=invisible, 1.0=full black (default: 1.0)
    pub distance: f32,          // Max shadow distance in world units: 0.0=full radius (default: 0.0)
    pub attenuation: f32,       // Fade curve: 1.0=linear, >1.0=faster fade, <1.0=slower (default: 1.0)
}

impl ShadowSettings {
    pub fn hard() -> Self;      // Hard shadows, default settings
    pub fn soft() -> Self;      // Soft shadows with PCF5
}
```

### `PointLight`

A point light that emits in all directions with radial falloff.

```rust
pub struct PointLight {
    pub position: Vec2,            // World-space position
    pub color: Color,              // Light color
    pub intensity: f32,            // Multiplier applied to color
    pub radius: f32,               // Radius of influence in world units
    pub casts_shadows: bool,       // Whether this light casts shadows (default: false)
    pub shadow: ShadowSettings,    // Shadow appearance settings
}

impl PointLight {
    pub fn new(position: Vec2, color: Color, intensity: f32, radius: f32) -> Self;
}
```

### `DirectionalLight`

A directional light that illuminates the entire scene uniformly. The direction is used for shadow projection — shadows are cast along this direction. Without normal maps, direction has no effect on shading.

```rust
pub struct DirectionalLight {
    pub direction: Vec2,           // Direction light shines FROM
    pub color: Color,              // Light color
    pub intensity: f32,            // Multiplier applied to color
    pub casts_shadows: bool,       // Whether this light casts shadows (default: false)
    pub shadow: ShadowSettings,    // Shadow appearance settings
}

impl DirectionalLight {
    pub fn new(direction: Vec2, color: Color, intensity: f32) -> Self;
}
```

### `ShadowFilter`

PCF (Percentage Closer Filtering) mode for shadow edge softness. Mirrors Godot 4's shadow filter options.

```rust
pub enum ShadowFilter {
    None,   // Hard shadows — crisp edges
    Pcf5,   // 5-tap PCF — cardinal + center
    Pcf13,  // 13-tap PCF — 3×3 grid + 4 extended
}

impl ShadowFilter {
    /// Get the integer value passed to the shader uniform (0, 5, or 13).
    pub fn as_uniform_value(&self) -> u32;
}
```

### `LightId`

Opaque handle returned by `add_light` or `add_directional_light`. Used to query, mutate, or remove a light. IDs are globally unique across both light types.

### `Occluder` / `OccluderEdge`

Shadow-casting shapes extracted from game objects.

```rust
pub struct OccluderEdge {
    pub a: [f32; 2],       // First endpoint (world space)
    pub b: [f32; 2],       // Second endpoint (world space)
    pub normal: [f32; 2],  // Outward-facing normal
}

pub struct Occluder {
    pub edges: Vec<OccluderEdge>,
}

impl Occluder {
    pub fn new(edges: Vec<OccluderEdge>) -> Self;
    pub fn from_aabb(cx: f32, cy: f32, hw: f32, hh: f32) -> Self;
    pub fn from_ground(y: f32, x_min: f32, x_max: f32) -> Self;
    pub fn from_boundary_edges(positions: &[f32], boundary_edges: &[(u32, u32)]) -> Self;
}
```

### `ShadowQuad`

A projected shadow polygon (2 triangles, 4 vertices) produced by the shadow projection functions. Vertices are in world space and should be drawn as solid black geometry to the shadow mask FBO. Near vertices carry full shadow alpha; far vertices may fade depending on attenuation settings.

```rust
pub struct ShadowQuad {
    pub positions: [f32; 8],       // 4 vertices × 2 components: [ax, ay, bx, by, b'x, b'y, a'x, a'y]
    pub indices: [u32; 6],         // Triangle indices (always [0,1,2, 0,2,3])
    pub vertex_colors: [f32; 16],  // Per-vertex RGBA (4 verts × 4 components)
}
```

### Shadow projection functions

Public functions in the `shadow` module for computing shadow geometry from lights and occluders.

```rust
/// Check if an occluder edge is back-facing relative to a point light.
pub fn is_back_facing_point(edge: &OccluderEdge, light_pos: [f32; 2]) -> bool;

/// Check if an occluder edge is back-facing relative to a directional light.
pub fn is_back_facing_directional(edge: &OccluderEdge, light_direction: [f32; 2]) -> bool;

/// Compute shadow quads for a point light.
/// Projects back-facing occluder edges radially away from the light.
/// `shadow_distance` caps how far shadows extend (0.0 = full radius).
/// `shadow_attenuation` controls fade curve: alpha = (1 - t)^attenuation.
pub fn project_point_shadows(
    light_pos: [f32; 2],
    light_radius: f32,
    occluders: &[Occluder],
    shadow_distance: f32,
    shadow_attenuation: f32,
) -> Vec<ShadowQuad>;

/// Compute shadow quads for a directional light.
/// Projects back-facing occluder edges along the light direction.
/// `cast_distance` is typically the camera diagonal.
pub fn project_directional_shadows(
    light_direction: [f32; 2],
    cast_distance: f32,
    occluders: &[Occluder],
    shadow_distance: f32,
    shadow_attenuation: f32,
) -> Vec<ShadowQuad>;

/// Compute boundary edges from a triangle mesh.
/// Returns vertex index pairs (v0, v1) for edges belonging to exactly one triangle.
/// These form the outer silhouette used as occluder edges for shadow casting.
pub fn compute_boundary_edges(triangles: &[u32]) -> Vec<(u32, u32)>;
```

### `generate_radial_gradient`

Generates the radial gradient texture used for point light rendering.

```rust
/// Produces a `size × size` RGBA texture with quadratic falloff: alpha = 1 - dist².
pub fn generate_radial_gradient(size: u32) -> TextureDescriptor;
```

### `LightingSystem`

Manages lights, ambient color, shadows, and the lightmap FBO.

```rust
impl LightingSystem {
    pub fn new() -> Self;

    // Ambient
    pub fn set_ambient(&mut self, color: Color);
    pub fn ambient(&self) -> Color;

    // Enable/disable
    pub fn set_enabled(&mut self, enabled: bool);
    pub fn is_enabled(&self) -> bool;

    // Point light management
    pub fn add_light(&mut self, light: PointLight) -> LightId;
    pub fn remove_light(&mut self, id: LightId);
    pub fn get_light(&self, id: LightId) -> Option<&PointLight>;
    pub fn get_light_mut(&mut self, id: LightId) -> Option<&mut PointLight>;
    pub fn light_count(&self) -> usize;
    pub fn clear_lights(&mut self);

    // Directional light management
    pub fn add_directional_light(&mut self, light: DirectionalLight) -> LightId;
    pub fn remove_directional_light(&mut self, id: LightId);
    pub fn get_directional_light(&self, id: LightId) -> Option<&DirectionalLight>;
    pub fn get_directional_light_mut(&mut self, id: LightId) -> Option<&mut DirectionalLight>;
    pub fn directional_light_count(&self) -> usize;
    pub fn clear_directional_lights(&mut self);

    // Combined queries
    pub fn has_lights(&self) -> bool;

    // Shadows
    pub fn set_occluders(&mut self, occluders: Vec<Occluder>);
    pub fn set_ground_shadow(&mut self, y: Option<f32>);

    // Rendering (called by World automatically)
    pub fn ensure_resources(&mut self, renderer: &mut dyn Renderer<Error = String>);
    pub fn render_lightmap(&self, renderer: &mut dyn Renderer<Error = String>, camera: &Camera);
    pub fn composite_lightmap(&self, renderer: &mut dyn Renderer<Error = String>, camera: &Camera);
    pub fn lightmap_texture(&self) -> Option<TextureId>;
}
```

## Usage

### Basic lighting

```rust
world.lighting.set_ambient(Color::new(0.1, 0.1, 0.15, 1.0));
world.lighting.set_enabled(true);

let light = world.lighting.add_light(PointLight::new(
    Vec2::new(5.0, 3.0),
    Color::new(1.0, 0.9, 0.7, 1.0),
    1.0,
    6.0,
));
```

### Light following an object

Use `World::light_follow` to have a point light automatically track an object's position each `step()`. This is the preferred approach over manually syncing positions in `update()`.

```rust
let light = world.lighting.add_light(PointLight::new(pos, color, 1.0, 6.0));
world.light_follow(light, player_id);

// Optional: follow with an offset
world.light_follow_with_offset(light, player_id, Vec2::new(0.0, 2.0));

// Stop following
world.light_unfollow(light);
```

The position is synced instantly (no smoothing) to avoid shadow artifacts from position lag.

### Shadow casting

Enable shadows on a light by setting `casts_shadows: true`. Configure appearance via `ShadowSettings`:

```rust
use unison2d::lighting::{PointLight, ShadowFilter, ShadowSettings};

let light = world.lighting.add_light(PointLight {
    position: Vec2::new(0.0, 3.0),
    color: Color::new(1.0, 0.9, 0.7, 1.0),
    intensity: 1.0,
    radius: 6.0,
    casts_shadows: true,
    shadow: ShadowSettings {
        filter: ShadowFilter::Pcf5,
        strength: 0.8,        // slightly transparent shadows
        distance: 5.0,        // shadows fade over 5 world units
        attenuation: 1.0,     // linear fade
    },
});

// Clip shadows at ground surface so they don't bleed below
world.lighting.set_ground_shadow(Some(-4.5));
```

Use the convenience constructors for common setups:

```rust
// Hard shadows with defaults
shadow: ShadowSettings::hard(),

// Soft shadows with PCF5
shadow: ShadowSettings::soft(),
```

### Per-object shadow control

All rigid bodies and soft bodies cast shadows by default. Disable on specific objects:

```rust
world.objects.set_casts_shadow(particle_id, false);
```

`World::auto_render` and `World::render_to_targets` collect occluders and render shadows automatically for lit render layers. Unlit layers bypass the lighting system entirely.

## Shadow architecture

For each shadow-casting light:

1. **Occluder collection** — `ObjectSystem::collect_occluders()` extracts edges from rigid bodies (AABB), soft bodies (boundary edges), and the ground plane
2. **Shadow projection** — for each occluder edge facing away from the light, project a shadow quad away from the light
3. **Shadow mask** — render shadow quads as black geometry on a white FBO
4. **Lit sprite** — draw the light to the lightmap using a shader that samples both the gradient texture and shadow mask, with optional PCF filtering

## Gradient texture

Point lights are rendered using a 64×64 radial gradient texture generated at runtime. The falloff is quadratic: `alpha = 1 - dist²`. The `gradient` module exposes `generate_radial_gradient(size)` for testing.

## Dependencies

- `unison-math` — Vec2, Color
- `unison-render` — Renderer trait, BlendMode, Camera, RenderCommand, DrawLitSprite
