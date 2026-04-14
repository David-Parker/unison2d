/// <reference types="@typescript-to-lua/language-extensions" />

// ===================================================================
// unison.* root namespace — all engine services and types
// ===================================================================

/** Anti-aliasing modes accepted by unison.renderer.set_anti_aliasing. */
declare type AntiAliasingMode = "none" | "msaa2x" | "msaa4x" | "msaa8x";

/** Opaque texture ID returned by unison.assets.load_texture. */
declare type TextureId = number;

/** Opaque render target ID returned by unison.renderer.create_target. */
declare type RenderTargetId = number;

/** Asset loading service. */
declare interface UnisonAssets {
  /** Load a texture from embedded assets. Returns a texture ID. Call in init or on_enter. */
  load_texture(this: void, path: string): TextureId;
}

/** Renderer configuration and screen info. */
declare interface UnisonRenderer {
  /** Current screen dimensions in logical points. Returns [width, height]. */
  screen_size(this: void): LuaMultiReturn<[number, number]>;
  /** Current anti-aliasing mode, or nil if not set. */
  anti_aliasing(this: void): AntiAliasingMode | undefined;
  /** Request AA mode for this session. Applied after init returns. */
  set_anti_aliasing(this: void, mode: AntiAliasingMode): void;
  /** Create an offscreen render target. Returns [target_id, texture_id]. Call in init or on_enter. */
  create_target(this: void, w: number, h: number): LuaMultiReturn<[RenderTargetId, TextureId]>;
}

/** Raw input state, refreshed automatically before each update. */
declare interface UnisonInput {
  /** True while the key is held down. */
  is_key_pressed(this: void, key: KeyName): boolean;
  /** True only on the frame the key was first pressed. */
  is_key_just_pressed(this: void, key: KeyName): boolean;
  /** True only on the frame the key was released. */
  is_key_just_released(this: void, key: KeyName): boolean;
  /** Horizontal axis in [-1, 1] from joystick or touch joystick. */
  axis_x(this: void): number;
  /** Vertical axis in [-1, 1] from joystick or touch joystick. */
  axis_y(this: void): number;
  /** Array of new touch-start positions this frame. */
  touches_just_began(this: void): TouchPosition[];
  /** True on the frame the primary (left) mouse button was first pressed. */
  is_mouse_just_pressed(this: void): boolean;
  /** True on the frame the primary (left) mouse button was released. */
  is_mouse_button_just_released(this: void): boolean;
  /** Current mouse position in screen space: `[x, y]`. */
  mouse_position(this: void): LuaMultiReturn<[number, number]>;
  /**
   * Cross-platform tap/click: returns `[x, y]` of a just-began touch, or the
   * mouse position if the primary button was just pressed. Returns
   * `[undefined, undefined]` when neither happened this frame.
   */
  pointer_just_pressed(this: void): LuaMultiReturn<[number | undefined, number | undefined]>;
  /**
   * Cross-platform "pointer is currently held" position: returns `[x, y]` of
   * an active touch, or the mouse position if the primary button is held.
   * Returns `[undefined, undefined]` when no pointer is active.
   */
  pointer_position(this: void): LuaMultiReturn<[number | undefined, number | undefined]>;
}

/** Scene management service. */
declare interface UnisonScenes {
  /**
   * Set or switch the active scene. Calls on_exit on the previous scene (if any),
   * then on_enter on the new scene.
   */
  set(this: void, scene: Scene): void;
  /** Returns the current scene table, or nil if no scene is active. */
  current(this: void): Scene | undefined;
}

/** String-keyed pub/sub event bus. */
declare interface UnisonEvents {
  /** Register a callback for a named event. Multiple listeners are allowed. */
  on(this: void, name: string, callback: (data?: any) => void): void;
  /** Emit a named event with optional data. Callbacks fire at end of frame. */
  emit(this: void, name: string, data?: any): void;
  /** Clear all string-keyed event handlers and pending events. */
  clear(this: void): void;
}

/** UI factory — create a UI handle for rendering declarative UI. */
declare interface UnisonUI {
  /** Create a UI handle for the given font asset. Reuse the handle across frames. */
  new: (this: void, font_path: string) => UI;
}

/** Development utilities. */
declare interface UnisonDebug {
  /** Print varargs to stderr, joined with tabs. Values are converted via `tostring`. */
  log(this: void, ...args: any[]): void;
  /** Draw a 0.1-unit point at world position (x, y). Color is a hex integer. */
  draw_point(this: void, x: number, y: number, color: number): void;
  /** Toggle physics debug visualization. Currently a no-op; reserved for future engine support. */
  show_physics(this: void, enabled: boolean): void;
  /** Toggle FPS counter overlay. Currently a no-op; reserved for future engine support. */
  show_fps(this: void, enabled: boolean): void;
}

/** Math utility extensions. */
declare interface UnisonMath {
  /** Linear interpolation: a + (b - a) * t. */
  lerp(this: void, a: number, b: number, t: number): number;
  /** Smooth Hermite interpolation, clamped to [0, 1]. */
  smoothstep(this: void, edge0: number, edge1: number, x: number): number;
  /** Clamp x to [min, max]. */
  clamp(this: void, x: number, min: number, max: number): number;
}

/** World constructor table. */
declare interface UnisonWorldConstructor {
  /** Create a new World. Default: "main" camera, gravity -9.8. */
  new: (this: void) => World;
}

/** Color constructor table. */
declare interface UnisonColorConstructor {
  /** Create a Color from a hex integer (e.g. `0xFF8800`). */
  hex: (this: void, hex: number) => Color;
  /** Create a Color from RGBA floats in [0, 1]. */
  rgba: (this: void, r: number, g: number, b: number, a: number) => Color;
}

/** Rng constructor table. */
declare interface UnisonRngConstructor {
  /** Create a new RNG with the given seed. Seed 0 is treated as 1. */
  new: (this: void, seed: number) => Rng;
}

/** The single root namespace — all engine services and types. */
declare const unison: {
  /** Asset loading service. */
  assets: UnisonAssets;
  /** Renderer configuration and screen info. */
  renderer: UnisonRenderer;
  /** Raw input state. */
  input: UnisonInput;
  /** Scene management service. */
  scenes: UnisonScenes;
  /** String-keyed pub/sub event bus (on, emit, clear). */
  events: UnisonEvents;
  /** UI factory. */
  UI: UnisonUI;
  /** Development utilities. */
  debug: UnisonDebug;
  /** Math utility extensions. */
  math: UnisonMath;
  /** World constructor. */
  World: UnisonWorldConstructor;
  /** Color constructor. */
  Color: UnisonColorConstructor;
  /** Rng constructor. */
  Rng: UnisonRngConstructor;
};
