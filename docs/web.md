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

Implements `Renderer` trait from `unison-render`. Supports all 5 `RenderCommand` variants:

| Command | Implementation |
|---------|---------------|
| `Mesh` | Dynamic buffer upload, draw triangles |
| `Sprite` | Textured quad with rotation + UV |
| `Rect` | Two-triangle filled quad |
| `Line` | Thin rectangle along line direction |
| `Terrain` | Fan triangulation of polygon |

Two shader programs: solid-color and textured (with tint). Camera transform applied as a 3x3 view-projection matrix uniform.

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
- Accumulates real delta time each frame
- Steps `game.update()` at 60Hz intervals (multiple steps if needed)
- Caps accumulator at 100ms to prevent spiral of death
- Calls `game.render()` once per frame after all updates

## Dependencies

- `web-sys` — DOM and WebGL2 bindings
- `wasm-bindgen` — Rust↔JS interop
- `js-sys` — JavaScript type bindings

## HTML Requirements

The canvas element must have `id="canvas"`:

```html
<canvas id="canvas" width="800" height="600"></canvas>
```
