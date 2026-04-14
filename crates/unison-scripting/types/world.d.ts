/// <reference types="@typescript-to-lua/language-extensions" />

// ===================================================================
// Mesh types — soft body shape definitions
// ===================================================================

/** Mesh shape type for soft bodies. */
declare type MeshType = "ring" | "square" | "ellipse" | "star" | "blob" | "rounded_box";

/**
 * Ring mesh params: [outer_radius, inner_radius, segments, radial_divisions].
 * All four values are required.
 */
declare type RingMeshParams = [number, number, number, number];

/**
 * Square mesh params: [size, divisions?].
 * Divisions defaults to 4 if omitted.
 */
declare type SquareMeshParams = [number] | [number, number];

/**
 * Ellipse mesh params: [radius_x, radius_y, segments, rings].
 * All four values are required.
 */
declare type EllipseMeshParams = [number, number, number, number];

/**
 * Star mesh params: [outer_radius, inner_radius, points, divisions?].
 * Divisions defaults to 4 if omitted.
 */
declare type StarMeshParams = [number, number, number] | [number, number, number, number];

/**
 * Blob mesh params: [radius, variation, segments, rings, seed?].
 * Seed defaults to 42 if omitted.
 */
declare type BlobMeshParams =
  | [number, number, number, number]
  | [number, number, number, number, number];

/**
 * Rounded box mesh params: [width, height, corner_radius, corner_segments].
 * All four values are required.
 */
declare type RoundedBoxMeshParams = [number, number, number, number];

/** Union of all mesh parameter tuple types. */
declare type MeshParams =
  | RingMeshParams | SquareMeshParams | EllipseMeshParams
  | StarMeshParams | BlobMeshParams | RoundedBoxMeshParams;

// ===================================================================
// Material types
// ===================================================================

/** Built-in material presets for soft bodies. */
declare type MaterialPreset = "rubber" | "jello" | "wood" | "metal" | "slime";

/** Custom material with explicit physics parameters. */
declare interface CustomMaterial {
  /** Mass density. */
  density: number;
  /** Edge compliance (inverse stiffness for edges). */
  edge_compliance: number;
  /** Area compliance (inverse stiffness for area preservation). */
  area_compliance: number;
}

// ===================================================================
// Spawn descriptors
// ===================================================================

/** Descriptor for spawning a deformable soft body. */
declare interface SoftBodyDescriptor {
  /** Mesh shape type. */
  mesh: MeshType;
  /** Shape parameters as a positional array (varies by mesh type). */
  mesh_params: MeshParams;
  /** Material preset name or custom material table. */
  material: MaterialPreset | CustomMaterial;
  /** World position as [x, y]. */
  position: [number, number];
  /** Optional hex color tint. Defaults to white. */
  color?: number;
  /** Optional texture ID from unison.assets.load_texture(). */
  texture?: TextureId;
}

/** Rigid body collider type. */
declare type ColliderType = "circle" | "aabb";

/** Descriptor for spawning a rigid body with a collider. */
declare interface RigidBodyDescriptor {
  /** Collider shape type. */
  collider: ColliderType;
  /** Radius for circle colliders. Required when collider is "circle". */
  radius?: number;
  /** Half-width for AABB colliders. Required when collider is "aabb". */
  half_width?: number;
  /** Half-height for AABB colliders. Required when collider is "aabb". */
  half_height?: number;
  /** World position as [x, y]. */
  position: [number, number];
  /** Optional hex color. Defaults to white. */
  color?: number;
  /** Whether the body is immovable. Defaults to false. */
  is_static?: boolean;
}

/** Descriptor for spawning a visual-only sprite (no physics). */
declare interface SpriteDescriptor {
  /** Texture ID from unison.assets.load_texture(). */
  texture?: TextureId;
  /** World position as [x, y]. */
  position: [number, number];
  /** Size as [width, height]. Defaults to [1, 1]. */
  size?: [number, number];
  /** Rotation in radians. Defaults to 0. */
  rotation?: number;
  /** Optional hex color tint. Defaults to white. */
  color?: number;
}

// ===================================================================
// Shadow types
// ===================================================================

/** Shadow filter algorithm. */
declare type ShadowFilter = "none" | "pcf5" | "soft" | "pcf13";

/** Custom shadow configuration table. */
declare interface ShadowConfig {
  /** Shadow filter algorithm. */
  filter: ShadowFilter;
  /** Shadow strength multiplier. Defaults to 1.0. */
  strength?: number;
  /** Shadow distance. Defaults to 0.0. */
  distance?: number;
  /** Shadow attenuation factor. Defaults to 1.0. */
  attenuation?: number;
}

/** Shadow option: a preset string or a custom configuration table. */
declare type ShadowOption = "hard" | "soft" | ShadowConfig;

// ===================================================================
// Light descriptors
// ===================================================================

/** Descriptor for adding a point light to the world. */
declare interface PointLightDescriptor {
  /** Light position as [x, y]. */
  position: [number, number];
  /** Light color as a hex integer. Defaults to white. */
  color?: number;
  /** Intensity multiplier. Defaults to 1.0. */
  intensity?: number;
  /** Light radius in world units. Defaults to 5.0. */
  radius?: number;
  /** Whether this light casts shadows. Defaults to false. */
  casts_shadows?: boolean;
  /** Shadow settings: "hard", "soft", or a custom config table. */
  shadow?: ShadowOption;
}

/** Descriptor for adding a directional light to the world. */
declare interface DirectionalLightDescriptor {
  /** Light direction as [x, y]. Will be normalized internally. */
  direction: [number, number];
  /** Light color as a hex integer. Defaults to white. */
  color?: number;
  /** Intensity multiplier. Defaults to 1.0. */
  intensity?: number;
  /** Whether this light casts shadows. Defaults to false. */
  casts_shadows?: boolean;
  /** Shadow settings: "hard", "soft", or a custom config table. */
  shadow?: ShadowOption;
}

/** Opaque light handle returned by add_point_light / add_directional_light. */
declare type LightId = number;

// ===================================================================
// Render layer types
// ===================================================================

/** Opaque render layer handle. */
declare type RenderLayerId = number;

/** Options for creating a render layer. */
declare interface RenderLayerOptions {
  /** Whether the layer is affected by the lightmap. Defaults to true. */
  lit?: boolean;
  /** Clear color as a hex integer. Defaults to black. */
  clear_color?: number;
}

// ===================================================================
// Draw command types
// ===================================================================

/** Shape type for world:draw, world:draw_to, and world:draw_unlit. */
declare type DrawShape = "rect" | "line" | "circle" | "gradient_circle";

/** Color for draw params: a hex integer or an [r, g, b] / [r, g, b, a] float array. */
declare type DrawColor = number | [number, number, number] | [number, number, number, number];

/** Parameters for drawing a rectangle. */
declare interface DrawParamsRect {
  /** Center X position. */
  x: number;
  /** Center Y position. */
  y: number;
  /** Rectangle width. */
  width: number;
  /** Rectangle height. */
  height: number;
  /** Fill color as hex integer or [r, g, b, a?] float array. */
  color: DrawColor;
}

/** Parameters for drawing a line. */
declare interface DrawParamsLine {
  /** Start X position. */
  x1: number;
  /** Start Y position. */
  y1: number;
  /** End X position. */
  x2: number;
  /** End Y position. */
  y2: number;
  /** Line color as hex integer or [r, g, b, a?] float array. */
  color: DrawColor;
  /** Line width. Defaults to 1.0. */
  width?: number;
}

/** Parameters for drawing a circle or gradient circle. */
declare interface DrawParamsCircle {
  /** Center X position. */
  x: number;
  /** Center Y position. */
  y: number;
  /** Circle radius. */
  radius: number;
  /** Fill color as hex integer or [r, g, b, a?] float array. */
  color: DrawColor;
}

/** Union of all draw parameter types. */
declare type DrawParams = DrawParamsRect | DrawParamsLine | DrawParamsCircle;

// ===================================================================
// Render-to-targets mapping
// ===================================================================

/** A single camera-to-target mapping: [camera_name, target_id_or_screen]. */
declare type RenderTargetMapping = [string, RenderTargetId | "screen"];

// ===================================================================
// World interface
// ===================================================================

/** World instance with physics, objects, cameras, lighting, and rendering. */
declare interface World {
  // --- World Configuration ---

  /** Set background clear color as a hex integer (e.g. 0x1a1a2e). */
  set_background(this: World, hex: number): void;
  /** Set gravity strength (negative = downward, e.g. -9.8). */
  set_gravity(this: World, g: number): void;
  /** Add a flat ground plane at the given world Y coordinate. */
  set_ground(this: World, y: number): void;
  /** Set ground bounciness: 0 = no bounce, 1 = perfect elastic. */
  set_ground_restitution(this: World, r: number): void;
  /** Set ground friction: 0 = frictionless, 1 = sticky. */
  set_ground_friction(this: World, f: number): void;

  // --- Simulation & Rendering ---

  /** Advance physics by dt seconds. Call in update. */
  step(this: World, dt: number): void;
  /** Render all objects and lighting through the main camera. Call in render. */
  render(this: World): void;
  /** Render each named camera to a specific render target. */
  render_to_targets(this: World, mapping: RenderTargetMapping[]): void;
  /** Composite a render-target texture onto the screen. Coordinates are in screen-space. */
  draw_overlay(this: World, texture_id: TextureId, x: number, y: number, w: number, h: number): void;
  /** Like draw_overlay but with a colored border. border_color is a hex integer. */
  draw_overlay_bordered(
    this: World,
    texture_id: TextureId,
    x: number, y: number, w: number, h: number,
    border_width: number, border_color: number
  ): void;

  // --- Object Spawning ---

  /** Spawn a deformable soft body. Returns an object ID. */
  spawn_soft_body(this: World, desc: SoftBodyDescriptor): ObjectId;
  /** Spawn a rigid body with an AABB or circle collider. Returns an object ID. */
  spawn_rigid_body(this: World, desc: RigidBodyDescriptor): ObjectId;
  /** Spawn an immovable rectangle. pos and size are [x, y] arrays; color is hex. Returns an object ID. */
  spawn_static_rect(this: World, pos: [number, number], size: [number, number], color: number): ObjectId;
  /** Spawn a visual-only sprite (no physics). Returns an object ID. */
  spawn_sprite(this: World, desc: SpriteDescriptor): ObjectId;
  /** Remove an object from the world. */
  despawn(this: World, id: ObjectId): void;

  // --- Physics Interaction ---

  /** Apply a continuous force (call each frame in update). */
  apply_force(this: World, id: ObjectId, fx: number, fy: number): void;
  /** Apply an instantaneous velocity change. */
  apply_impulse(this: World, id: ObjectId, ix: number, iy: number): void;
  /** Apply rotational torque. */
  apply_torque(this: World, id: ObjectId, torque: number, dt: number): void;

  // --- Queries ---

  /** Get object center position. Returns [x, y]. */
  get_position(this: World, id: ObjectId): LuaMultiReturn<[number, number]>;
  /** Get object velocity. Returns [vx, vy]. */
  get_velocity(this: World, id: ObjectId): LuaMultiReturn<[number, number]>;
  /** True if the object is resting on the ground plane. */
  is_grounded(this: World, id: ObjectId): boolean;
  /** True if objects a and b are in contact. */
  is_touching(this: World, a: ObjectId, b: ObjectId): boolean;

  // --- Display Properties ---

  /** Set draw order. Higher values draw on top. */
  set_z_order(this: World, id: ObjectId, z: number): void;
  /** Enable or disable shadow casting for this object. */
  set_casts_shadow(this: World, id: ObjectId, casts: boolean): void;
  /** Teleport object to an exact position. */
  set_position(this: World, id: ObjectId, x: number, y: number): void;

  // --- Camera ---

  /** Make a named camera follow an object. smoothing: 0 = frozen, 1 = instant snap. */
  camera_follow(this: World, name: string, id: ObjectId, smoothing: number): void;
  /** Follow an object with a world-space offset applied to the look-at point. */
  camera_follow_with_offset(this: World, name: string, id: ObjectId, smoothing: number, ox: number, oy: number): void;
  /** Add a named camera with the given viewport size in world units. */
  camera_add(this: World, name: string, width: number, height: number): void;
  /** Get the current camera center position. Returns [x, y]. */
  camera_get_position(this: World, name: string): LuaMultiReturn<[number, number]>;
  /**
   * Convert a screen-space point (e.g. from `unison.input.pointer_just_pressed()`) to
   * world-space using the `"main"` camera. Returns [world_x, world_y].
   */
  screen_to_world(this: World, screen_x: number, screen_y: number): LuaMultiReturn<[number, number]>;

  // --- Lighting: System Configuration ---

  /** Enable or disable the entire lighting system. */
  lighting_set_enabled(this: World, enabled: boolean): void;
  /** Set ambient light color as RGBA floats in [0, 1]. */
  lighting_set_ambient(this: World, r: number, g: number, b: number, a: number): void;
  /** Add a ground shadow plane at Y, or pass nil/false to disable. */
  lighting_set_ground_shadow(this: World, y: number | undefined | false): void;

  // --- Lighting: Point Lights ---

  /** Add a point light. Returns a light handle. */
  add_point_light(this: World, desc: PointLightDescriptor): LightId;
  /** Update light intensity (multiplier). Works for both point and directional lights. */
  set_light_intensity(this: World, handle: LightId, intensity: number): void;
  /** Make the light track an object each frame. */
  light_follow(this: World, handle: LightId, id: ObjectId): void;
  /** Track an object with a world-space offset. */
  light_follow_with_offset(this: World, handle: LightId, id: ObjectId, ox: number, oy: number): void;
  /** Stop the light from tracking an object. */
  light_unfollow(this: World, handle: LightId): void;

  // --- Lighting: Directional Lights ---

  /** Add a directional light. Returns a light handle. */
  add_directional_light(this: World, desc: DirectionalLightDescriptor): LightId;
  /** Update a directional light's direction vector. Will be normalized internally. */
  set_directional_light_direction(this: World, handle: LightId, dx: number, dy: number): void;

  // --- Render Layers ---

  /** Create a new named render layer, appended after existing layers. Returns a layer handle. */
  create_render_layer(this: World, name: string, desc: RenderLayerOptions): RenderLayerId;
  /** Insert a new named render layer before an existing layer by handle. Returns a layer handle. */
  create_render_layer_before(this: World, name: string, before: RenderLayerId, desc: RenderLayerOptions): RenderLayerId;
  /** Update a layer's clear color at runtime. */
  set_layer_clear_color(this: World, handle: RenderLayerId, hex: number): void;
  /** Get the handle for the default scene layer. */
  default_layer(this: World): RenderLayerId;
  /** Draw a shape to a specific layer at the given depth. */
  draw_to(this: World, layer: RenderLayerId, shape: DrawShape, params: DrawParams, z: number): void;
  /** Draw a shape to the default layer at the given depth. */
  draw(this: World, shape: DrawShape, params: DrawParams, z: number): void;
  /** Draw a shape to the default layer, unaffected by the lightmap. */
  draw_unlit(this: World, shape: DrawShape, params: DrawParams, z: number): void;
}

// World constructor is now unison.World.new()
// See unison.d.ts.
