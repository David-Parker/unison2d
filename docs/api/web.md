# unison-web

Web platform crate — WebGL2 renderer, DOM input wiring, and requestAnimationFrame game loop.

## Usage

```rust
use unison_web::run;

#[wasm_bindgen(start)]
pub fn main() {
    run(MyGame { /* ... */ });
}
```

`run()` handles everything:
1. Gets the `<canvas id="canvas">` element
2. Creates a WebGL2 context and renderer
3. Wires keyboard, mouse, and touch events into `InputState`
4. Sets up the profiler time function
5. Starts a `requestAnimationFrame` loop with fixed timestep (60Hz)

## WebGL2 Renderer

Implements `Renderer` trait from `unison-render`. Supports all 6 `RenderCommand` variants:

| Command | Implementation |
|---------|---------------|
| `Mesh` | Dynamic buffer upload, draw triangles |
| `Sprite` | Textured quad with rotation + UV |
| `LitSprite` | Textured quad with shadow mask sampling + PCF filtering |
| `Rect` | Two-triangle filled quad |
| `Line` | Thin rectangle along line direction |
| `Terrain` | Fan triangulation of polygon |

Three shader programs: solid-color, textured (with tint), and lit sprite (samples both a gradient texture and a shadow mask with optional PCF filtering). Camera transform applied as a 3x3 view-projection matrix uniform.

### Render Targets (FBOs)

The WebGL2 renderer implements offscreen render targets:

| Method | Description |
|--------|-------------|
| `create_render_target(w, h)` | Creates FBO + RGBA8 texture attachment |
| `bind_render_target(target)` | Binds FBO, sets viewport to target size |
| `destroy_render_target(target)` | Deletes FBO, keeps the texture |

`RenderTargetId::SCREEN` binds the default framebuffer and restores canvas viewport.

Render target textures are registered in the renderer's texture map and can be used directly in `DrawSprite` commands for compositing.

## Input Wiring

DOM events mapped to `InputState`:

| DOM Event | InputState Method |
|-----------|------------------|
| `keydown` | `key_pressed(KeyCode)` |
| `keyup` | `key_released(KeyCode)` |
| `mousemove` | `mouse_moved(x, y)` |
| `mousedown` | `mouse_button_pressed(btn)` |
| `mouseup` | `mouse_button_released(btn)` |
| `touchstart` | `touch_started(id, x, y)` |
| `touchmove` | `touch_moved(id, x, y)` |
| `touchend` | `touch_ended(id)` |
| `touchcancel` | `touch_cancelled(id)` |

Game keys (arrows, space, tab) have `preventDefault()` to avoid page scrolling.

## Game Loop

Fixed timestep accumulator pattern:
- Calls `Profiler::begin_frame()` at the start of each frame
- Accumulates real delta time each frame
- Calls `engine.pre_update()` then `game.update()` at 60Hz intervals (multiple steps if needed)
- Caps accumulator at 100ms to prevent spiral of death
- Calls `game.render()` once per frame after all updates
- Calls `Profiler::end_frame()` to accumulate frame statistics
- Every 120 frames, logs profiler stats to the browser console via `web_sys::console::log_1` and resets

The game loop does NOT auto-render or auto-step physics — the game controls both via `world.step(dt)` and `world.auto_render(renderer)` in its `update()` and `render()` callbacks.

## Dependencies

- `web-sys` — DOM and WebGL2 bindings
- `wasm-bindgen` — Rust↔JS interop
- `js-sys` — JavaScript type bindings

## HTML Requirements

The canvas element must have `id="canvas"`:

```html
<canvas id="canvas" width="800" height="600"></canvas>
```
