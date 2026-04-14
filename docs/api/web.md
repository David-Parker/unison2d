# unison-web

Web platform crate — WebGL2 renderer, DOM input wiring, and
`requestAnimationFrame` game loop.

## Usage

Game entry points use `unison_scripting::scripted_game_entry!`, which expands
to a `#[wasm_bindgen(start)]` function that calls `unison_web::run()`:

```rust
// project/lib.rs — same pattern on web, iOS, and Android
unison_scripting::scripted_game_entry!("scripts/main.lua", assets::ASSETS);
```

`run()` handles everything:
1. Gets the `<canvas id="canvas">` element
2. Creates a WebGL2 context and renderer
3. Wires keyboard, mouse, and touch DOM events into `InputState`
4. Sets up the profiler time function
5. Starts a `requestAnimationFrame` loop with fixed timestep (60 Hz)

## WebGL2 Renderer

`WebGlRenderer` implements the `Renderer` trait from `unison-render`. It
supports all `RenderCommand` variants:

| Command | Implementation |
|---------|---------------|
| `Mesh` | Dynamic buffer upload, draw triangles |
| `Sprite` | Textured quad with rotation + UV |
| `LitSprite` | Textured quad with shadow mask sampling + PCF filtering |
| `Rect` | Two-triangle filled quad |
| `Line` | Thin rectangle along line direction |
| `Terrain` | Fan triangulation of polygon |

Three shader programs: solid-color, textured (with tint), and lit sprite
(samples both a gradient texture and a shadow mask with optional PCF filtering).
Camera transform applied as a 3×3 view-projection matrix uniform.

### Render Targets (FBOs)

| Method | Description |
|--------|-------------|
| `create_render_target(w, h)` | Creates FBO + RGBA8 texture attachment |
| `bind_render_target(target)` | Binds FBO; sets viewport to target size |
| `destroy_render_target(target)` | Deletes FBO; keeps the texture |

`RenderTargetId::SCREEN` binds the default framebuffer and restores the canvas
viewport.

## Input Wiring

DOM events mapped to `InputState`:

| DOM Event | `InputState` method |
|-----------|---------------------|
| `keydown` | `key_pressed(KeyCode)` |
| `keyup` | `key_released(KeyCode)` |
| `mousemove` | `mouse_moved(x, y)` |
| `mousedown` | `mouse_button_pressed(btn)` |
| `mouseup` | `mouse_button_released(btn)` |
| `touchstart` | `touch_started(id, x, y)` |
| `touchmove` | `touch_moved(id, x, y)` |
| `touchend` | `touch_ended(id)` |
| `touchcancel` | `touch_cancelled(id)` |

Arrow keys, Space, and Tab have `preventDefault()` to suppress page scrolling.

## Game Loop

Fixed-timestep accumulator pattern:

- `Profiler::begin_frame()` at the start of each `rAF` callback
- Accumulates real delta time; steps at 60 Hz (multiple steps per frame if
  needed); caps accumulator at 100 ms to prevent spiral of death
- `game.update()` called once per tick; `game.render()` once per frame
- Every 120 frames, profiler stats are logged to the browser console and reset

## HTML Requirements

The canvas element must have `id="canvas"`:

```html
<canvas id="canvas" width="800" height="600"></canvas>
```

## Dependencies

- `web-sys` — DOM and WebGL2 bindings
- `wasm-bindgen` — Rust ↔ JS interop
- `js-sys` — JavaScript type bindings
