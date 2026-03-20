# unison-lighting

2D dynamic lighting with soft shadows. Uses `Vec2`, `Color`, and `Rect` from `unison-math`.

## LightingSystem

Central coordinator for all lights and shadow maps.

```rust
use unison_math::{Color, Rect, Vec2};

let mut lighting = LightingSystem::new();
lighting.set_ambient(Color::rgb(0.1, 0.1, 0.15));
lighting.set_shadow_quality(ShadowQuality::High);
```

### Adding & Managing Lights

```rust
let handle = lighting.add_light(Light::point(Vec2::new(5.0, 3.0), 10.0)
    .with_color(Color::rgb(1.0, 0.9, 0.7))
    .with_intensity(1.5)
    .with_shadows(true));

lighting.get_light(handle)          // -> Option<&Light>
lighting.get_light_mut(handle)      // -> Option<&mut Light>
lighting.remove_light(handle);
lighting.light_count()              // -> usize
lighting.all_lights()               // -> impl Iterator<Item = &Light>
```

### Shadow Updates

```rust
// Mark dirty when lights or occluders move
lighting.mark_dirty(handle);
lighting.mark_all_dirty();

// Recompute shadow maps
lighting.update_shadows(&occluders); // &[&dyn ShadowCaster]
```

### Frustum Culling

```rust
let bounds = Rect::from_center(Vec2::new(cam_x, cam_y), Vec2::new(width, height));
let visible = lighting.get_visible_lights(&bounds); // -> Vec<&Light>
```

## Light

Factory methods for each light type (all positions/directions use `Vec2`):

```rust
Light::point(position, radius)
Light::spot(position, radius, angle, direction)
Light::directional(direction)
Light::area(position, width, height)
```

Builder methods (chainable):

```rust
.with_color(color)       // Color
.with_intensity(intensity)
.with_shadows(bool)
```

Properties: `light_type`, `position: Vec2`, `color: Color`, `intensity`, `shadows`, `enabled`.

Query methods: `effective_radius()`, `affects_point(point: Vec2)`.

## LightType

```rust
enum LightType {
    Point { radius: f32 },
    Spot { radius: f32, angle: f32, direction: Vec2 },
    Directional { direction: Vec2 },
    Area { width: f32, height: f32 },
}
```

## ShadowQuality

| Preset | Resolution | PCF Samples |
|--------|-----------|-------------|
| `Off` | — | — |
| `Low` | 128 | 2 |
| `Medium` (default) | 256 | 4 |
| `High` | 512 | 8 |

## ShadowCaster (trait)

Implement for objects that block light.

```rust
trait ShadowCaster {
    fn get_occluder_segments(&self) -> Vec<(Vec2, Vec2)>;
}
```

## ShadowMap

1D distance map (angle → nearest occluder distance).

```rust
shadow_map.sample(angle)                        // direct lookup
shadow_map.sample_pcf(angle, samples, spread)   // soft shadow
shadow_map.clear();
shadow_map.mark_dirty();
```

## ShadowMapCache

Manages shadow map allocation with slot reuse.

```rust
let mut cache = ShadowMapCache::new();
let id = cache.allocate(256);
cache.get(id)       // -> Option<&ShadowMap>
cache.get_mut(id)   // -> Option<&mut ShadowMap>
cache.mark_all_dirty();
cache.get_dirty()   // -> Vec<ShadowMapId>
cache.free(id);
```

## OccluderData

Helper for building occluder geometry (uses `Vec2`).

```rust
let mut occ = OccluderData::new();
occ.add_segment(start, end);           // Vec2, Vec2
occ.add_rect(x, y, width, height);     // f32s
occ.add_polygon(&vertices);            // &[Vec2]
```

## LightingRenderer (trait)

Platform crates implement this for GPU-based lighting.

```rust
trait LightingRenderer {
    fn create_shadow_map(&mut self, resolution: u32) -> ShadowMapId;
    fn update_shadow_map(&mut self, id: ShadowMapId, light: &Light, occluders: &[OccluderData]);
    fn destroy_shadow_map(&mut self, id: ShadowMapId);
    fn bind_lighting(&mut self, lights: &[&Light], ambient: Color, shadow_maps: &[ShadowMapId]);
    fn begin_lighting_pass(&mut self);
    fn end_lighting_pass(&mut self);
}
```

`NullLightingRenderer` is provided for testing.
