# unison-lighting

2D dynamic lighting with soft shadows.

## LightingManager

Central coordinator for all lights and shadow maps.

```rust
let mut manager = LightingManager::new();
manager.set_ambient(0.1, 0.1, 0.15);
manager.set_shadow_quality(ShadowQuality::High);
```

### Adding & Managing Lights

```rust
let handle = manager.add_light(Light::point((5.0, 3.0), 10.0)
    .with_color(1.0, 0.9, 0.7)
    .with_intensity(1.5)
    .with_shadows(true));

manager.get_light(handle)          // -> Option<&Light>
manager.get_light_mut(handle)      // -> Option<&mut Light>
manager.remove_light(handle);
manager.light_count()              // -> usize
manager.all_lights()               // -> impl Iterator<Item = &Light>
```

### Shadow Updates

```rust
// Mark dirty when lights or occluders move
manager.mark_dirty(handle);
manager.mark_all_dirty();

// Recompute shadow maps
manager.update_shadows(&occluders); // &[&dyn ShadowCaster]
```

### Frustum Culling

```rust
let bounds = CameraBounds::from_center((cam_x, cam_y), width, height);
let visible = manager.get_visible_lights(&bounds); // -> Vec<&Light>
```

## Light

Factory methods for each light type:

```rust
Light::point(position, radius)
Light::spot(position, radius, angle, direction)
Light::directional(direction)
Light::area(position, width, height)
```

Builder methods (chainable):

```rust
.with_color(r, g, b)
.with_intensity(intensity)
.with_shadows(bool)
```

Properties: `light_type`, `position`, `color`, `intensity`, `shadows`, `enabled`.

Query methods: `effective_radius()`, `affects_point(point)`.

## LightType

```rust
enum LightType {
    Point { radius: f32 },
    Spot { radius: f32, angle: f32, direction: (f32, f32) },
    Directional { direction: (f32, f32) },
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
    fn get_occluder_segments(&self) -> Vec<((f32, f32), (f32, f32))>;
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

Helper for building occluder geometry.

```rust
let mut occ = OccluderData::new();
occ.add_segment(start, end);
occ.add_rect(x, y, width, height);
occ.add_polygon(&vertices);
```

## LightingRenderer (trait)

Platform crates implement this for GPU-based lighting.

```rust
trait LightingRenderer {
    fn create_shadow_map(&mut self, resolution: u32) -> ShadowMapId;
    fn update_shadow_map(&mut self, id: ShadowMapId, light: &Light, occluders: &[OccluderData]);
    fn destroy_shadow_map(&mut self, id: ShadowMapId);
    fn bind_lighting(&mut self, lights: &[&Light], ambient: (f32, f32, f32), shadow_maps: &[ShadowMapId]);
    fn begin_lighting_pass(&mut self);
    fn end_lighting_pass(&mut self);
}
```

`NullLightingRenderer` is provided for testing.

## Config (serde)

For loading lights from scene files:

```rust
SceneLightingConfig {
    ambient: Option<AmbientConfig>,  // { color: [f32; 3] }
    lights: Vec<LightConfig>,        // light_type, position, color, intensity, etc.
}

// Convert config to light
let light = config.to_light()?;
```
