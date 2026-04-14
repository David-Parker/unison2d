/// <reference types="@typescript-to-lua/language-extensions" />

// This file is now superseded by unison.d.ts.
// Engine services (assets, renderer, scenes, UI) live under `unison.*`.
// Kept here only for shared type aliases referenced by other .d.ts files.

/** Anti-aliasing modes accepted by unison.renderer.set_anti_aliasing. */
declare type AntiAliasingMode = "none" | "msaa2x" | "msaa4x" | "msaa8x";

/** Opaque texture ID returned by unison.assets.load_texture. */
declare type TextureId = number;

/** Opaque render target ID returned by unison.renderer.create_target. */
declare type RenderTargetId = number;
