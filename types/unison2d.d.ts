/**
 * Unison 2D — TypeScript type definitions for Lua scripting via TypeScriptToLua (TSTL).
 *
 * Import this file via tsconfig.json `paths` mapping:
 *   "unison2d": ["../unison2d/types/unison2d.d.ts"]
 *
 * Then in your TypeScript:
 *   import {} from "unison2d";  // side-effect import to pull in globals
 *
 * @see https://typescripttolua.github.io/
 */

import { LuaMultiReturn } from "@typescript-to-lua/language-extensions";

// =============================================================================
// Touch input
// =============================================================================

/** A touch point from a touchscreen input event. */
interface TouchPoint {
    x: number;
    y: number;
}

// =============================================================================
// Color class
// =============================================================================

/**
 * RGBA color. All channels are in the 0–1 range.
 *
 * @noSelf
 */
declare class Color {
    r: number;
    g: number;
    b: number;
    a: number;

    /** Create a Color from a packed 0xRRGGBB hex integer (alpha = 1.0). */
    static hex(value: number): Color;

    /** Create a Color from individual 0–1 channel values. */
    static rgba(r: number, g: number, b: number, a: number): Color;

    /** Linearly interpolate between this color and `other` by factor `t` (0–1). */
    lerp(other: Color, t: number): Color;
}

// =============================================================================
// Rng class
// =============================================================================

/**
 * Deterministic pseudo-random number generator.
 *
 * @noSelf
 */
declare class Rng {
    /** Create a new RNG seeded with `seed`. */
    static new(seed: number): Rng;

    /** Return a random float in [min, max]. */
    range(min: number, max: number): number;

    /** Return a random integer in [min, max] (inclusive). */
    range_int(min: number, max: number): number;
}

// =============================================================================
// World: descriptor types
// =============================================================================

/** Soft body material presets or a custom material table. */
type SoftBodyMaterial =
    | "rubber"
    | "jello"
    | "slime"
    | "wood"
    | "metal"
    | { density?: number; edge_compliance?: number; area_compliance?: number };

/** Descriptor for `World.spawn_soft_body`. */
interface SoftBodyDesc {
    /** Mesh shape: "ring", "square", "star", "ellipse", or custom. */
    mesh: string;
    /**
     * Mesh parameters — meaning depends on mesh type:
     * - ring:    [outer_radius, thickness, segments, rings]
     * - square:  [size, subdivisions]
     * - star:    [outer_radius, inner_radius, points, rings]
     * - ellipse: [rx, ry, segments, rings]
     */
    mesh_params?: number[];
    material: SoftBodyMaterial;
    position: [number, number];
    /** Packed 0xRRGGBB hex color. */
    color?: number;
    /** Texture handle from `engine.load_texture`. */
    texture?: number;
}

/** Descriptor for `World.spawn_rigid_body`. */
interface RigidBodyDesc {
    /** "aabb" (axis-aligned box) or "circle". */
    collider: "aabb" | "circle";
    /** Half-width for AABB colliders. */
    half_width?: number;
    /** Half-height for AABB colliders. */
    half_height?: number;
    /** Radius for circle colliders. */
    radius?: number;
    position: [number, number];
    /** Packed 0xRRGGBB hex color. */
    color?: number;
    /** Whether the body is immovable. Default: false. */
    is_static?: boolean;
}

/** Shadow quality descriptor for lights. */
type ShadowDesc =
    | "soft"
    | "hard"
    | {
          filter?: "none" | "pcf5" | "pcf13";
          distance?: number;
          strength?: number;
          attenuation?: number;
      };

/** Descriptor for `World.add_point_light`. */
interface PointLightDesc {
    position: [number, number];
    /** Packed 0xRRGGBB hex color. */
    color: number;
    intensity: number;
    radius: number;
    casts_shadows?: boolean;
    shadow?: ShadowDesc;
}

/** Descriptor for `World.add_directional_light`. */
interface DirectionalLightDesc {
    /** Normalized direction vector [dx, dy]. */
    direction: [number, number];
    /** Packed 0xRRGGBB hex color. */
    color: number;
    intensity: number;
    casts_shadows?: boolean;
    shadow?: ShadowDesc;
}

/** Descriptor for `World.lighting_set_ground_shadow`. Pass `null` to disable. */
type GroundShadowDesc = number | null;

/** Descriptor for `World.create_render_layer`. */
interface RenderLayerDesc {
    /** Whether objects on this layer receive lighting. Default: true. */
    lit?: boolean;
    /** Initial clear color as a packed 0xRRGGBB hex integer. */
    clear_color?: number;
}

/** Shape name for `World.draw_to`. */
type DrawShape =
    | "rect"
    | "circle"
    | "gradient_circle"
    | "line"
    | "triangle"
    | string;

/** Parameters for `World.draw_to`. */
type DrawParams = {
    x?: number;
    y?: number;
    width?: number;
    height?: number;
    radius?: number;
    x1?: number;
    y1?: number;
    x2?: number;
    y2?: number;
    /** Packed hex integer OR [r, g, b, a] array. */
    color?: number | [number, number, number, number];
    [key: string]: unknown;
};

/** Render target specification: [camera_name, target_name]. */
type RenderTargetSpec = [string, string];

// =============================================================================
// Sprite descriptor
// =============================================================================

/** Descriptor for `World.spawn_sprite`. */
interface SpriteDesc {
    texture: number;
    position: [number, number];
    width?: number;
    height?: number;
    color?: number;
}

// =============================================================================
// Overlay params
// =============================================================================

/** Parameters for `engine.draw_overlay` / `engine.draw_overlay_bordered`. */
interface OverlayParams {
    x?: number;
    y?: number;
    width?: number;
    height?: number;
    border?: number;
    border_color?: number;
}

// =============================================================================
// World class
// =============================================================================

/**
 * A self-contained physics + rendering simulation.
 *
 * Methods use Lua colon-call syntax (`world:method()`), which TSTL maps to
 * regular instance method calls (`world.method()`).
 *
 * @noSelf
 */
declare class World {
    /** Create a new empty World. */
    static new(): World;

    // -------------------------------------------------------------------------
    // World configuration
    // -------------------------------------------------------------------------

    /** Set the background clear color (packed 0xRRGGBB). */
    set_background(color: number): void;

    /** Set world gravity (negative = downward, e.g. -9.8). */
    set_gravity(g: number): void;

    /** Set the Y coordinate of the ground plane. */
    set_ground(y: number): void;

    /** Set the restitution (bounciness) of the ground. */
    set_ground_restitution(r: number): void;

    // -------------------------------------------------------------------------
    // Physics
    // -------------------------------------------------------------------------

    /** Advance the simulation by `dt` seconds. */
    step(dt: number): void;

    /** Render all cameras to their default targets (convenience). */
    auto_render(): void;

    // -------------------------------------------------------------------------
    // Object spawning
    // -------------------------------------------------------------------------

    /** Spawn a soft body and return its object ID. */
    spawn_soft_body(desc: SoftBodyDesc): number;

    /** Spawn a rigid body and return its object ID. */
    spawn_rigid_body(desc: RigidBodyDesc): number;

    /**
     * Spawn a static (immovable) rectangular object and return its ID.
     * @param pos World-space center [x, y].
     * @param size Full size [width, height].
     * @param color Packed 0xRRGGBB.
     */
    spawn_static_rect(pos: [number, number], size: [number, number], color: number): number;

    /** Spawn a textured sprite and return its object ID. */
    spawn_sprite(desc: SpriteDesc): number;

    /** Remove an object from the world. */
    despawn(id: number): void;

    // -------------------------------------------------------------------------
    // Object manipulation
    // -------------------------------------------------------------------------

    /** Apply a continuous force to an object this frame. */
    apply_force(id: number, fx: number, fy: number): void;

    /** Apply an instantaneous impulse to an object. */
    apply_impulse(id: number, ix: number, iy: number): void;

    /** Apply a rotational torque to an object. */
    apply_torque(id: number, t: number, dt: number): void;

    /** Get the world-space position of an object. */
    get_position(id: number): LuaMultiReturn<[number, number]>;

    /** Get the current velocity of an object. */
    get_velocity(id: number): LuaMultiReturn<[number, number]>;

    /** Whether the object is resting on the ground or another surface. */
    is_grounded(id: number): boolean;

    /** Whether two objects are currently in contact. */
    is_touching(id: number, other: number): boolean;

    /** Teleport an object to a new position. */
    set_position(id: number, x: number, y: number): void;

    /** Set the z-order (draw depth) of an object. */
    set_z_order(id: number, z: number): void;

    /** Control whether this object casts shadows. */
    set_casts_shadow(id: number, casts: boolean): void;

    // -------------------------------------------------------------------------
    // Camera
    // -------------------------------------------------------------------------

    /**
     * Make the named camera smoothly follow an object.
     * @param name Camera name (e.g. "main").
     * @param id Object ID to follow.
     * @param damping Follow lag (0 = instant, higher = slower).
     */
    camera_follow(name: string, id: number, damping: number): void;

    /**
     * Follow an object with a positional offset.
     * @param ox Horizontal offset in world units.
     * @param oy Vertical offset in world units.
     */
    camera_follow_with_offset(
        name: string,
        id: number,
        damping: number,
        ox: number,
        oy: number,
    ): void;

    /**
     * Register an additional named camera.
     * @param w Viewport width in world units.
     * @param h Viewport height in world units.
     */
    camera_add(name: string, w: number, h: number): void;

    /** Get the current world-space position of the named camera. */
    camera_get_position(name: string): LuaMultiReturn<[number, number]>;

    // -------------------------------------------------------------------------
    // Lighting
    // -------------------------------------------------------------------------

    /** Enable or disable the lighting system. */
    lighting_set_enabled(enabled: boolean): void;

    /** Set the ambient light color (r, g, b, a all in 0–1 range). */
    lighting_set_ambient(r: number, g: number, b: number, a: number): void;

    /**
     * Set the ground shadow plane Y coordinate. Pass `null` or omit to disable.
     */
    lighting_set_ground_shadow(y: GroundShadowDesc): void;

    /** Add a point light and return its handle. */
    add_point_light(desc: PointLightDesc): number;

    /** Add a directional light and return its handle. */
    add_directional_light(desc: DirectionalLightDesc): number;

    /** Set the intensity of a light (point or directional). */
    set_light_intensity(handle: number, intensity: number): void;

    /** Update the direction of a directional light. */
    set_directional_light_direction(handle: number, dx: number, dy: number): void;

    /** Make a light follow an object. */
    light_follow(handle: number, id: number): void;

    /** Make a light follow an object with a positional offset. */
    light_follow_with_offset(handle: number, id: number, ox: number, oy: number): void;

    /** Stop a light from following its target. */
    light_unfollow(handle: number): void;

    // -------------------------------------------------------------------------
    // Render layers
    // -------------------------------------------------------------------------

    /** Get the handle of the default render layer. */
    default_layer(): number;

    /**
     * Create a named render layer (appended after all existing layers).
     * Returns a layer handle.
     */
    create_render_layer(name: string, desc: RenderLayerDesc): number;

    /**
     * Create a named render layer inserted before an existing layer.
     * Returns a layer handle.
     */
    create_render_layer_before(name: string, before: number, desc: RenderLayerDesc): number;

    /** Update the clear color of a render layer at runtime. */
    set_layer_clear_color(layer: number, color: number): void;

    /**
     * Submit a draw call to a specific render layer.
     * @param layer Layer handle from `create_render_layer`.
     * @param shape Shape type string.
     * @param params Shape parameters.
     * @param z Z-order within the layer.
     */
    draw_to(layer: number, shape: DrawShape, params: DrawParams, z: number): void;

    /**
     * Submit a draw call to the default lit render layer.
     */
    draw(shape: DrawShape, params: DrawParams, z: number): void;

    /**
     * Submit a draw call to the default unlit render layer.
     */
    draw_unlit(shape: DrawShape, params: DrawParams, z: number): void;

    /**
     * Render each camera to the specified targets.
     * Each entry is [camera_name, target_name].
     */
    render_to_targets(targets: RenderTargetSpec[]): void;
}

// =============================================================================
// engine global
// =============================================================================

/** UI handle returned by `engine.create_ui`. */
interface UiHandle {
    /**
     * Submit a UI frame for rendering.
     * `tree` is an array of UI node descriptor tables.
     */
    frame(tree: object[]): void;
}

/** Render target handle returned by `engine.create_render_target`. */
type RenderTarget = number;

/**
 * Engine global — asset loading, screen utilities, scene management.
 * Exposed as the Lua global `engine`.
 */
declare const engine: {
    /**
     * Load a texture from the asset path and return its handle.
     * The handle is a numeric ID usable in spawn descriptors.
     */
    load_texture(path: string): number;

    /**
     * Return the screen dimensions in pixels.
     * Usage: `const [w, h] = engine.screen_size();`
     */
    screen_size(): LuaMultiReturn<[number, number]>;

    /**
     * Enable or disable anti-aliasing.
     * Valid string values: "none", "msaa4x", "msaa8x".
     */
    set_anti_aliasing(mode: string | boolean): void;

    /**
     * Create a UI rendering context.
     * @param fontPath Path to the font asset (e.g. "fonts/MyFont.ttf").
     */
    create_ui(fontPath: string): UiHandle;

    /**
     * Create an off-screen render target with the given dimensions.
     */
    create_render_target(w: number, h: number): RenderTarget;

    /**
     * Draw a render target as a fullscreen or partial overlay.
     */
    draw_overlay(rt: RenderTarget, params: OverlayParams): void;

    /**
     * Draw a render target overlay with a colored border.
     */
    draw_overlay_bordered(rt: RenderTarget, params: OverlayParams): void;

    /**
     * Immediately transition to a new scene (calls `on_enter`).
     */
    set_scene(scene: SceneTable): void;

    /**
     * Transition to a new scene: calls `on_exit` on the current scene,
     * then `on_enter` on the new one.
     */
    switch_scene(scene: SceneTable): void;
};

// =============================================================================
// Scene table
// =============================================================================

/**
 * A scene is a plain Lua table / TypeScript object with optional lifecycle hooks.
 * Passed to `engine.set_scene` / `engine.switch_scene`.
 */
interface SceneTable {
    on_enter?: () => void;
    on_exit?: () => void;
    update?: (dt: number) => void;
    render?: () => void;
}

// =============================================================================
// input global
// =============================================================================

/**
 * Input global — keyboard, analog axis, and touch queries.
 * Exposed as the Lua global `input`.
 */
declare const input: {
    /** Whether the named key is held down this frame. */
    is_key_pressed(key: string): boolean;

    /** Whether the named key was pressed this frame (rising edge). */
    is_key_just_pressed(key: string): boolean;

    /** Analog horizontal axis value in [-1, 1] (gamepad / on-screen stick). */
    axis_x(): number;

    /** Analog vertical axis value in [-1, 1]. */
    axis_y(): number;

    /** Array of touch points that began this frame. Empty if none. */
    touches_just_began(): TouchPoint[];
};

// =============================================================================
// events global
// =============================================================================

/** Generic event handler — receives an optional data payload. */
type EventHandler = (data?: unknown) => void;

/** Collision info passed to collision handlers. */
interface CollisionInfo {
    normal?: [number, number];
    depth?: number;
}

/**
 * Events global — string-keyed event bus + physics collision hooks.
 * Exposed as the Lua global `events`.
 */
declare const events: {
    /** Emit a named event with an optional data payload. */
    emit(name: string, data?: unknown): void;

    /** Register a handler for the named event. */
    on(name: string, handler: EventHandler): void;

    /** Register a handler called whenever any two objects collide. */
    on_collision(handler: (a: number, b: number, info: CollisionInfo) => void): void;

    /** Register a handler called when object `id` is involved in any collision. */
    on_collision_for(id: number, handler: (other: number, info: CollisionInfo) => void): void;

    /** Register a handler called when objects `id1` and `id2` collide with each other. */
    on_collision_between(
        id1: number,
        id2: number,
        handler: (info: CollisionInfo) => void,
    ): void;
};

// =============================================================================
// math extensions
// =============================================================================

/**
 * Extended math table — adds lerp, smoothstep, clamp to Lua's standard `math`.
 * The standard `math` global is augmented with these methods.
 */
declare namespace math {
    /** Linear interpolation: a + (b - a) * t. */
    function lerp(a: number, b: number, t: number): number;

    /** Hermite interpolation between a and b using t (clamped to [0,1]). */
    function smoothstep(a: number, b: number, t: number): number;

    /** Clamp `v` to [min, max]. */
    function clamp(v: number, min: number, max: number): number;
}
