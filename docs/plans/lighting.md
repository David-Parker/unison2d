# 2D Lighting System — Design & Plan

## Motivation

Add a forward-rendered 2D lighting system to the engine, inspired by
Godot 4's approach. The system should support point lights, directional
lights, ambient light, and eventually shadows and normal maps.

## Design Approach

### Why Godot-style forward lighting?

We evaluated six common 2D lighting techniques:

| Technique | Description | Tradeoff |
|-----------|-------------|----------|
| **Multiplicative lightmap** | Render lights as blobs to FBO, multiply over scene | Simplest, but no surface detail |
| **Deferred / G-buffer** | Render normals+albedo to G-buffer, light in screen space | Scales well, but complex and poor transparency handling |
| **Ray-based shadows** | Cast rays from lights to geometry edges | Accurate shadows, but CPU-heavy and scales poorly |
| **SDF shadows** | Signed distance field + raymarching | Beautiful soft shadows, but SDF must regenerate for moving geometry |
| **Baked lighting** | Pre-rendered light in assets | Zero runtime cost, but fully static |
| **Screen-space normal maps** | Per-pixel Blinn-Phong with normal maps | Rich surface detail, but requires normal map assets |

We chose **single-pass forward rendering with a lightmap FBO**, the same
core approach Godot 4 uses for its 2D pipeline. Key reasons:

- **Pragmatic middle ground** — more capable than a simple multiply-blend,
  simpler than full deferred rendering
- **WebGL2 compatible** — no MRT (multiple render targets) required for the
  base system
- **Incremental** — start with lightmap compositing, layer on shadows and
  normal maps later without rewriting the foundation
- **Fits our architecture** — we already have FBO render targets and a
  compositing pipeline in `Engine`

### How Godot 4 does it (reference)

Godot 4's 2D lighting uses forward rendering with a single-pass atlas
optimization. Their shader pipeline has three stages:

1. `vertex()` — transforms vertices
2. `fragment()` — outputs base color/texture
3. `light()` — called once per pixel per light, accumulates into `COLOR`

Key details:
- **Light types:** `PointLight2D` (local, radial falloff) and `DirectionalLight2D` (global, parallel)
- **Blend modes:** Add, Mix, Subtract for light compositing
- **Shadows:** Depth-based shadow mapping from `LightOccluder2D` nodes, with PCF filtering (None/PCF5/PCF13)
- **SDF layer:** Optional screen-space signed distance field for advanced soft shadows (queryable in shaders via `texture_sdf()`)
- **Normal maps:** `CanvasTexture` pairs diffuse + normal map, shader does Blinn-Phong per-pixel lighting

Sources:
- [Godot 2D engine improvements for 4.0](https://godotengine.org/article/godots-2d-engine-gets-several-improvements-upcoming-40/)
- [Godot 2D lights and shadows docs](https://docs.godotengine.org/en/stable/tutorials/2d/2d_lights_and_shadows.html)
- [Godot CanvasItem shader reference](https://docs.godotengine.org/en/stable/tutorials/shaders/shader_reference/canvas_item_shader.html)
- [Godot canvas.glsl source](https://github.com/godotengine/godot/blob/master/servers/rendering/renderer_rd/shaders/canvas.glsl)

### Other useful references

- [slembcke — 2D Lighting Techniques](https://www.slembcke.net/blog/2DLightingTechniques/) — lightmap approach explained with diagrams
- [slembcke — Super Fast Soft Shadows](https://www.slembcke.net/blog/SuperFastSoftShadows/) — gradient-based penumbras at 1/8th resolution
- [SIGHT & LIGHT](https://ncase.me/sight-and-light/) — interactive raycasting shadow demo
- [Red Blob Games — 2D Visibility](https://www.redblobgames.com/articles/visibility/) — visibility polygon tutorial
- [Ronja — 2D SDF Shadows](https://www.ronja-tutorials.com/post/037-2d-shadows/) — SDF raymarching with adjustable hardness
- [Dead Cells art deep dive](https://www.gamedeveloper.com/production/art-design-deep-dive-using-a-3d-pipeline-for-2d-animation-in-i-dead-cells-i-) — hand-drawn normal maps + toon shader

### Our approach (simplified Godot model)

We take the same core idea but simplify for our engine's scale:

1. **Lightmap FBO** — render all light contributions to an offscreen texture
2. **Composite** — multiply-blend the lightmap over the scene
3. **Ambient light** — base illumination level (so unlit areas aren't pure black)
4. **No atlas** — we don't need Godot's atlas optimization yet; one FBO is sufficient
5. **Additive light accumulation** — each light adds its contribution to the lightmap

The rendering flow becomes:

```
1. Render scene normally (objects, sprites, terrain) → scene FBO or screen
2. Render lights to lightmap FBO:
   - Clear to ambient color
   - For each light: draw additive radial gradient (point) or full-screen quad (directional)
3. Composite lightmap over scene with multiply blend
```

---

## Phase 1 — Point Lights ✓ complete

**Goal:** Get colored point lights affecting the scene, with the full
plumbing in place (`unison-lighting` crate, integration into World,
tests, game integration).

### Deliverables

- [x] **`unison-lighting` crate** — new workspace member
  - `PointLight` struct: position, color, intensity, radius
  - `LightingSystem`: manages light collection + ambient color
  - `LightingSystem::render_lightmap()` + `composite_lightmap()`: renders lightmap to FBO
- [x] **`BlendMode` enum + `set_blend_mode()`** on the `Renderer` trait
  - `Alpha` (current default), `Additive`, `Multiply`
  - WebGL implementation switches `gl.blendFunc` accordingly
- [x] **Radial gradient texture** — 64×64 procedural texture with quadratic falloff
  - Uses existing shader + sprite rendering, no new shader program needed
- [x] **World integration** — `World` owns a `LightingSystem`
  - `auto_render` and `render_to_targets` composite the lightmap when lighting is enabled
- [x] **`unison-tests` integration tests** (10 tests)
  - Add/remove point lights, get/mutate, clear, ambient, enabled flag
  - Gradient texture validation (dimensions, center bright, edge dark)
  - World integration (lighting system present on new World)
- [x] **donut-game integration** — warm point light follows the donut in MainLevel
- [x] **Documentation** — `docs/api/lighting.md`, updated CLAUDE.md, INDEX.md, render.md

### API sketch

```rust
// In unison-lighting
pub struct PointLight {
    pub position: Vec2,
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
}

pub struct LightingSystem {
    lights: Vec<PointLight>,
    ambient: Color,
}

impl LightingSystem {
    pub fn new() -> Self;
    pub fn set_ambient(&mut self, color: Color);
    pub fn add_point_light(&mut self, light: PointLight) -> LightId;
    pub fn remove_light(&mut self, id: LightId);
    pub fn get_light_mut(&mut self, id: LightId) -> Option<&mut PointLight>;

    /// Render lightmap: clears to ambient, draws each light additively.
    pub fn render_lightmap(
        &self,
        renderer: &mut dyn Renderer<Error = String>,
        camera: &Camera,
    );
}
```

```rust
// Game usage
world.lighting.set_ambient(Color::new(0.1, 0.1, 0.15, 1.0));
let torch = world.lighting.add_point_light(PointLight {
    position: Vec2::new(5.0, 3.0),
    color: Color::new(1.0, 0.8, 0.5, 1.0),
    intensity: 1.5,
    radius: 8.0,
});
```

---

## Phase 2 — Directional Lights ✓ complete

**Goal:** Add directional lights (sun/moon) that illuminate the entire
scene from a given angle.

### Deliverables

- [x] `DirectionalLight` struct: direction, color, intensity
- [x] Render directional lights as full-camera-bounds solid-color quads (additive) on the lightmap
- [x] Separate storage in `LightingSystem` with shared `LightId` counter
- [x] `has_lights()` convenience method for World integration
- [x] Integration tests for directional lights (5 tests)
- [x] donut-game: moonlight directional light in MainLevel and RandomSpawnsLevel
- [x] Update docs

### Design notes

Directional lights are simpler than point lights — they don't have
position or radius, just a direction and color. They render as a
full-camera-bounds solid-color quad (`TextureId::NONE`) added to the
lightmap with additive blending. Without normal maps, directional lights
are effectively a uniform color wash — their real value comes in Phase 4
when normals let surfaces respond to light direction. The `direction`
field is stored for forward compatibility.

### API sketch

```rust
pub struct DirectionalLight {
    pub direction: Vec2,   // Direction light shines FROM (for Phase 4)
    pub color: Color,
    pub intensity: f32,
}

// Management methods on LightingSystem:
pub fn add_directional_light(&mut self, light: DirectionalLight) -> LightId;
pub fn remove_directional_light(&mut self, id: LightId);
pub fn get_directional_light(&self, id: LightId) -> Option<&DirectionalLight>;
pub fn get_directional_light_mut(&mut self, id: LightId) -> Option<&mut DirectionalLight>;
pub fn directional_light_count(&self) -> usize;
pub fn clear_directional_lights(&mut self);
pub fn has_lights(&self) -> bool;  // point or directional
```

```rust
// Game usage
world.lighting.add_directional_light(DirectionalLight::new(
    Vec2::new(0.3, -1.0),               // moonlight angle
    Color::new(0.15, 0.15, 0.25, 1.0),  // cool blue tint
    1.0,
));
```

---

## Phase 3 — Shadow Casting ✓ complete

**Goal:** Geometry blocks light, casting shadows.

### Deliverables

- [x] `Occluder` / `OccluderEdge` types for shadow-casting shapes
- [x] `ShadowFilter` enum (None, Pcf5, Pcf13) — Godot-style PCF modes
- [x] `casts_shadows` + `shadow_filter` fields on `PointLight` and `DirectionalLight`
- [x] Per-object `casts_shadow` flag + `ObjectSystem::set_casts_shadow()`
- [x] `ObjectSystem::collect_occluders()` — edges from rigid bodies, soft bodies, ground
- [x] Soft body boundary edge precomputation (`Mesh::ensure_boundary_edges()`)
- [x] Shadow projection: `project_point_shadows()`, `project_directional_shadows()`
- [x] Silhouette edge selection (back-facing test)
- [x] `DrawLitSprite` render command (gradient × shadow mask × PCF)
- [x] Lit sprite shader with PCF (new WebGL shader program)
- [x] Multi-program support in WebGL renderer
- [x] Shadow mask FBO in `LightingSystem`
- [x] `LightingSystem::set_occluders()` + `set_ground_shadow()`
- [x] World integration: automatic occluder collection + shadow rendering
- [x] Integration tests (18 shadow-specific tests)
- [x] Game integration: shadows on donut point light in all levels
- [x] Documentation updated

### Design notes

Shadow casting uses a **per-light shadow mask** approach:

1. For each shadow-casting light, compute shadow quads on the CPU by
   projecting back-facing occluder edges away from the light
2. Render shadow quads as black geometry to a shadow mask FBO (white=lit, black=shadow)
3. Draw the light to the lightmap using a `LitSprite` shader that samples
   both the gradient texture and shadow mask, with optional PCF filtering

This avoids custom shadow map shaders while still producing quality results.
PCF filtering (5-tap or 13-tap) softens shadow edges by sampling multiple
shadow mask texels. The approach handles deformable soft body meshes
naturally — boundary edges are precomputed at mesh creation and positions
are updated from physics each frame.

### API sketch

```rust
// Shadow-casting point light
let light = world.lighting.add_light(PointLight {
    position: Vec2::new(0.0, 3.0),
    color: Color::new(1.0, 0.9, 0.7, 1.0),
    intensity: 1.0,
    radius: 6.0,
    casts_shadows: true,
    shadow_filter: ShadowFilter::Pcf5,
});

// Ground shadow
world.lighting.set_ground_shadow(Some(-4.5));

// Disable shadow casting for a specific object
world.objects.set_casts_shadow(particle_id, false);
```

---

## Phase 4 — Normal Maps (future)

**Goal:** Per-pixel surface lighting using normal maps.

Not yet planned in detail. Likely approach:
- `CanvasTexture`-style pairing of diffuse + normal map
- Extend fragment shader with Blinn-Phong calculation
- Normal map support for sprites and soft body meshes
- Requires normal map assets (can use auto-generation tools like SpriteIlluminator)

---

## Architecture Decisions

### Why a separate `unison-lighting` crate?

Follows the engine's existing pattern — each subsystem is its own crate
(`unison-physics`, `unison-render`, `unison-input`, etc.). Keeps
lighting code isolated, testable, and optional.

### Why lightmap FBO instead of per-object lighting?

- Decouples lighting from the object rendering pipeline
- Works with all render command types (sprites, meshes, terrain) without
  modifying each one
- Single composite pass is cheap
- Mirrors Godot's approach at a simpler scale

### Why not deferred rendering?

- Requires MRT (multiple render targets) for G-buffer — adds complexity
- Poor transparency handling
- Overkill for our current needs
- Can migrate later if needed — the lightmap approach is a natural
  stepping stone
