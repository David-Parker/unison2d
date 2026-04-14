/// <reference types="@typescript-to-lua/language-extensions" />

/** Anti-aliasing modes accepted by engine.set_anti_aliasing. */
declare type AntiAliasingMode = "none" | "msaa2x" | "msaa4x" | "msaa8x";

/** Opaque texture ID returned by engine.load_texture. */
declare type TextureId = number;

/** Opaque render target ID returned by engine.create_render_target. */
declare type RenderTargetId = number;

/** Engine configuration, texture loading, screen info, scene management, UI, and render targets. */
declare const engine: {
  // --- Textures & Screen ---

  /** Load a texture from embedded assets. Returns a texture ID. Call in init or on_enter. */
  load_texture(this: void, path: string): TextureId;
  /** Current screen dimensions in logical points. Returns [width, height]. */
  screen_size(this: void): Tuple<[number, number]>;
  /** Request AA mode for this session. Applied after init returns. */
  set_anti_aliasing(this: void, mode: AntiAliasingMode): void;

  // --- Scene Management ---

  /** Activate a scene. Calls `scene.on_enter()` if present. */
  set_scene(this: void, scene: Scene): void;
  /** Transition to a new scene. Calls on_exit on the current scene, then on_enter on the new one. */
  switch_scene(this: void, scene: Scene): void;

  // --- UI ---

  /** Create a UI handle for the given font asset. Reuse the handle across frames. */
  create_ui(this: void, font_path: string): UI;

  // --- Render Targets ---

  /** Create an offscreen render target. Returns [target_id, texture_id]. Call in init or on_enter. */
  create_render_target(this: void, w: number, h: number): Tuple<[RenderTargetId, TextureId]>;
  /** Composite a render-target texture onto the screen. Coordinates are in screen-space. */
  draw_overlay(this: void, texture_id: TextureId, x: number, y: number, w: number, h: number): void;
  /** Like draw_overlay but with a colored border. border_color is a hex integer. */
  draw_overlay_bordered(
    this: void,
    texture_id: TextureId,
    x: number, y: number, w: number, h: number,
    border_width: number, border_color: number
  ): void;

};
