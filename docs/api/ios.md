# unison-ios

iOS platform crate -- Metal renderer, touch input, FFI game loop, and Swift host package. Depends on: `unison2d`, `unison-render`, `unison-input`, `unison-profiler`, `metal`, `objc`, `block`.

## Usage

Games need exactly one line of Rust code to export to iOS:

```rust
// In your game crate (e.g., lib.rs or ios_ffi.rs):
#[cfg(feature = "ios")]
unison_ios::export_game!(MyGame, MyGame::new());
```

The Swift host app (provided by the `UnisoniOS` Swift package) handles MTKView setup, CADisplayLink rendering, and touch forwarding. The game crate compiles to a static library that the Xcode project links.

## Architecture

```
Swift (UnisoniOS package)              Rust (unison-ios crate)
┌───────────────────────┐              ┌───────────────────────┐
│ GameViewController    │──FFI calls──▶│ export_game! macro    │
│   ├─ Renderer (Swift) │              │   ├─ game_init        │
│   ├─ JoystickView     │              │   ├─ game_frame       │
│   └─ touch forwarding │              │   └─ game_destroy     │
└───────────────────────┘              │                       │
                                       │ GameState<G>          │
                                       │   ├─ Engine<A>        │
                                       │   ├─ MetalRenderer    │
                                       │   └─ InputBuffer      │
                                       └───────────────────────┘
```

## export_game! (macro)

Generates 9 `#[no_mangle] pub unsafe extern "C"` FFI entry points that bridge a concrete `Game` type to the Swift host app.

```rust
unison_ios::export_game!($game_type, $constructor);
```

**Arguments:**
- `$game_type` -- the concrete struct that implements `unison2d::Game`
- `$constructor` -- an expression that creates a new instance (e.g., `MyGame::new()`)

**Generated FFI functions:**

| Function | Signature | Description |
|----------|-----------|-------------|
| `game_init` | `(device, layer, width, height) -> *mut c_void` | Create renderer + game state; returns opaque pointer |
| `game_frame` | `(state, dt, drawable)` | Run one frame: fixed-timestep updates + render + present |
| `game_resize` | `(state, width, height)` | Notify game of screen size change (point-based) |
| `game_touch_began` | `(state, id, x, y)` | Feed touch-began event |
| `game_touch_moved` | `(state, id, x, y)` | Feed touch-moved event |
| `game_touch_ended` | `(state, id)` | Feed touch-ended event |
| `game_touch_cancelled` | `(state, id)` | Feed touch-cancelled event |
| `game_set_axis` | `(state, x, y)` | Set virtual joystick axis (-1.0 to 1.0) |
| `game_destroy` | `(state)` | Drop the game state and free memory |

All functions take the opaque `*mut c_void` returned by `game_init` as their first argument. The Swift side stores this as `UnsafeMutableRawPointer?`.

## GameState\<G\>

Owns the game, engine, renderer, and input buffer. One instance per app lifetime. Generic over any `Game` type -- the concrete type is supplied by the game crate via `export_game!`.

```rust
pub struct GameState<G: Game> {
    game: G,
    engine: Engine<G::Action>,
    input: InputBuffer,
    accumulator: f32,
    initialized: bool,
    metal_renderer: *mut MetalRenderer,  // raw pointer for Metal-specific calls
}
```

| Method | Description |
|--------|-------------|
| `new(renderer, game)` | Create game state; moves renderer into engine, keeps raw pointer for Metal-specific calls |
| `init()` | Initialize profiler (mach_absolute_time), set fixed timestep, call `game.init()`. Idempotent |
| `input_mut()` | Access `&mut InputBuffer` for feeding touch/axis events from FFI |
| `engine_mut()` | Access `&mut Engine<A>` (e.g., to update screen size) |
| `frame(dt, drawable)` | Run one display frame (unsafe -- drawable must be valid `CAMetalDrawable`) |

### Frame Loop

`frame()` implements the same fixed-timestep accumulator as the web crate:

1. `Profiler::begin_frame()`
2. Accumulate `dt` (capped at 100ms to prevent spiral of death)
3. Transfer touch/input events into the engine's `InputState`
4. Fixed-timestep updates at 60Hz: `engine.pre_update()` + `game.update()` (multiple ticks if needed)
5. `metal_renderer.begin_display_frame(drawable)` -- create command buffer, acquire drawable
6. `game.render(&mut engine)` -- engine render cycle (may call `begin_frame`/`end_frame`/`clear`/`bind_render_target` multiple times)
7. `metal_renderer.end_display_frame()` -- present drawable, commit command buffer
8. `Profiler::end_frame()` -- every 120 frames, log stats to Xcode console and reset

Constants:

| Constant | Value | Description |
|----------|-------|-------------|
| `FIXED_DT` | `1.0 / 60.0` | Fixed timestep (60 updates/second) |
| `MAX_ACCUMULATOR` | `0.1` | Max accumulated time per frame (prevents spiral of death) |
| `PROFILER_LOG_INTERVAL` | `120` | Frames between profiler stat dumps to Xcode console |

## MetalRenderer

Implements the `Renderer` trait from `unison-render`. Maps platform-agnostic render commands to Metal draw calls with triple-buffered vertex/index data.

### Construction

```rust
// Safety: raw_device must be a valid MTLDevice, raw_layer a valid CAMetalLayer.
let renderer = unsafe {
    MetalRenderer::new(raw_device, raw_layer, width, height)?
};
```

Created by the `game_init` FFI function. Receives raw Metal pointers from Swift via `Unmanaged.passUnretained`.

### Renderer Trait Implementation

All 6 `RenderCommand` variants are supported:

| Command | Implementation |
|---------|---------------|
| `Sprite` | Textured quad with rotation + UV |
| `Mesh` | Dynamic vertex/index upload, draw indexed triangles |
| `LitSprite` | Textured quad with shadow mask sampling + PCF filtering |
| `Rect` | Two-triangle filled quad |
| `Line` | Thin rectangle along line direction |
| `Terrain` | Strip triangulation extruded downward + surface lines |

| Trait Method | Description |
|--------------|-------------|
| `init()` | No-op (initialization handled in `new()`) |
| `begin_frame(camera)` | Compute 3x3 view-projection matrix from camera |
| `clear(color)` | End current encoder, create new with `MTLLoadAction::Clear` |
| `draw(command)` | Dispatch to sprite/mesh/lit/rect/line/terrain draw methods |
| `end_frame()` | End current render command encoder |
| `create_texture(desc)` | Create Metal texture, upload pixels, generate mipmaps on GPU |
| `destroy_texture(id)` | Remove texture from internal map |
| `screen_size()` | Returns point-based dimensions (matches UIKit touch coordinates) |
| `drawable_size()` | Returns pixel-based dimensions (physical Metal drawable size) |
| `fbo_origin_top_left()` | Returns `true` (Metal's coordinate convention) |
| `set_screen_size(w, h)` | Update point-based dimensions (called by `game_resize`) |
| `set_blend_mode(mode)` | Switch pipeline state (Alpha, Additive, Multiply) |
| `create_render_target(w, h)` | Create offscreen texture + optional MSAA texture |
| `bind_render_target(target)` | End encoder, switch target (lazy encoder creation on next draw) |
| `destroy_render_target(target)` | Remove render target from internal map |
| `set_anti_aliasing(mode)` | Clamp to device capability, create MSAA pipeline set |
| `anti_aliasing()` | Current AA mode |

### Display Frame Lifecycle

These Metal-specific methods are called by `GameState`, not by the engine's `Renderer` trait:

| Method | Description |
|--------|-------------|
| `begin_display_frame(drawable)` | Wait on triple-buffer semaphore, create command buffer, retain drawable, sync pixel dimensions, clear to black, reset vertex/index offsets |
| `end_display_frame()` | End encoder, present drawable, commit command buffer, signal semaphore on GPU completion, advance frame index |

### Triple Buffering

Three sets of vertex/index buffers rotate each frame to prevent CPU/GPU contention. A GCD dispatch semaphore (initialized to 3) gates frame starts -- the GPU completion handler signals the semaphore, and `begin_display_frame` waits on it.

| Constant | Value | Description |
|----------|-------|-------------|
| `MAX_FRAMES_IN_FLIGHT` | `3` | Number of buffered frames |
| `INITIAL_VERTEX_BUFFER_SIZE` | `1 MB` | Per-frame vertex buffer allocation |
| `INITIAL_INDEX_BUFFER_SIZE` | `256 KB` | Per-frame index buffer allocation |

### Pipeline States

Pipelines are organized into `PipelineSet` groups by MSAA sample count:

```
PipelineSet {
    base_alpha,      // standard transparency
    base_additive,   // glow / light accumulation
    base_multiply,   // darkening / lightmap compositing
    lit_alpha,       // lit sprite with shadow mask (alpha blend)
    lit_additive,    // lit sprite with shadow mask (additive blend)
}
```

Two sets are maintained: `screen_pipelines` (sample_count=1 for screen rendering) and `msaa_pipelines` (sample_count=N for offscreen MSAA render targets). The active set is chosen by the current render target.

### Coordinate System

Two separate size tracking values:

- **Point dimensions** (`screen_width_points`, `screen_height_points`) -- matches UIKit touch coordinates and game logic. Updated by `game_resize` / `set_screen_size`.
- **Pixel dimensions** (`screen_width`, `screen_height`) -- physical Metal drawable resolution. Synced automatically from the drawable texture at each `begin_display_frame`.

`screen_size()` returns points. `drawable_size()` returns pixels.

## Touch Input Helpers

Convenience functions in the `input` module that feed UIKit touch events into `InputBuffer`. Called from the FFI functions generated by `export_game!`.

| Function | Description |
|----------|-------------|
| `touch_began(input, id, x, y)` | Feed a touch-began event |
| `touch_moved(input, id, x, y)` | Feed a touch-moved event |
| `touch_ended(input, id)` | Feed a touch-ended event |
| `touch_cancelled(input, id)` | Feed a touch-cancelled event |
| `set_axis(input, x, y)` | Set virtual joystick axis value |

All functions delegate to `input.shared_mut()` methods on `InputBuffer`.

## Metal Shaders (MSL)

Ported from the GLSL shaders in `unison-web`. Embedded as Rust string constants and compiled at runtime via `device.new_library_with_source()`.

### Shader Sources

| Constant / Function | Description |
|---------------------|-------------|
| `SHADER_TYPES` | Shared vertex types (`Vertex`, `Uniforms`, `FragmentUniforms`, `LitFragmentUniforms`, `VertexOut`) |
| `VERTEX_SHADER` | Transforms 2D positions by 3x3 view-projection matrix |
| `FRAGMENT_SHADER` | Solid color with optional texture sampling and per-vertex color |
| `LIT_FRAGMENT_SHADER` | Lit sprite with shadow mask sampling + PCF5/PCF13 soft shadows |
| `base_shader_source()` | Concatenates types + vertex + base fragment |
| `lit_shader_source()` | Concatenates types + vertex + lit fragment |

### MSL Vertex Layout

```c
struct Vertex {
    float2 position [[attribute(0)]];  // 8 bytes
    float2 uv       [[attribute(1)]];  // 8 bytes
    float4 color    [[attribute(2)]];  // 16 bytes
};                                     // 32 bytes per vertex
```

### Uniform Structs

```c
struct Uniforms {
    float3x3 view_projection;  // 3 columns x float4 (padded) = 48 bytes
};

struct FragmentUniforms {
    float4 color;
    int use_texture;
    int _pad0, _pad1, _pad2;  // 16-byte alignment
};

struct LitFragmentUniforms {
    float4 color;
    int use_texture;
    int _pad0;
    float2 screen_size;
    int shadow_filter;     // 0 = none, 5 = PCF5, 13 = PCF13
    float shadow_strength; // 0.0 = no shadow, 1.0 = full shadow
    float2 _pad1;
};
```

## Swift Host Package (UnisoniOS)

A Swift Package (`UnisoniOS`) that provides the UIKit integration layer. The game's Xcode project adds this as a local package dependency.

```
UnisoniOS/
├── Package.swift
└── Sources/
    ├── UnisonGameFFI/
    │   └── include/UnisonGameFFI.h   # C header for FFI functions
    └── UnisoniOS/
        ├── GameViewController.swift  # MTKView setup + touch forwarding
        ├── Renderer.swift            # MTKViewDelegate, calls Rust via FFI
        └── JoystickView.swift        # Virtual joystick overlay
```

Platform requirement: iOS 15+.

### UnisonGameFFI.h

C header declaring the 9 FFI functions generated by `export_game!`. The actual function bodies are in the game's static library (linked by Xcode). This target exists solely so Swift can import the header.

### GameViewController

`open class GameViewController: UIViewController` -- generic view controller for any Unison 2D game.

| Responsibility | Details |
|---------------|---------|
| MTKView setup | Creates Metal device, configures `BGRA8Unorm` pixel format, no depth/stencil |
| Renderer creation | Instantiates `Renderer(metalKitView:)`, sets as MTKView delegate |
| Touch forwarding | Overrides `touchesBegan/Moved/Ended/Cancelled`, forwards to `game_touch_*` FFI functions using `touch.hash` as the touch ID |
| Joystick | Creates a `JoystickView` anchored to bottom-left safe area with 20pt margin |
| Multi-touch | Enables `isMultipleTouchEnabled` on the view |

Subclassable (`open class`) -- games can override to customize UI layout.

### Renderer (Swift)

`public class Renderer: NSObject, MTKViewDelegate` -- thin delegate that bridges MTKView to Rust.

| Method | Description |
|--------|-------------|
| `init?(metalKitView:)` | Pass device + layer pointers to `game_init`, then call `game_resize` with point-based bounds |
| `draw(in:)` | Compute delta time from `CACurrentMediaTime()`, call `game_frame(state, dt, drawable)` |
| `mtkView(_:drawableSizeWillChange:)` | Call `game_resize` with point-based view bounds (not pixel-based drawable size) |
| `deinit` | Call `game_destroy` to free the Rust game state |

Stores the opaque game state pointer as `UnsafeMutableRawPointer?`.

### JoystickView

`open class JoystickView: UIView` -- virtual joystick overlay for touch-based movement input.

| Property / Method | Description |
|-------------------|-------------|
| `gameState` | `UnsafeMutableRawPointer?` -- opaque pointer to Rust `GameState` |
| `defaultSize` | `120` points (static) |
| Touch handling | Tracks a single touch, computes normalized axis from drag distance |
| Axis output | Sends horizontal axis (`-1.0` to `1.0`) via `game_set_axis(state, x, 0)` |
| Hit testing | Circular hit area matching the base radius |
| Visual | Semi-transparent base circle + thumb circle using `CAShapeLayer` |

The joystick currently sends only horizontal axis (y is always 0), designed for side-scrolling platformers. Games needing 2D joystick input can subclass and override `handleThumbMove`.

## Dependencies

### Rust

- `metal` -- Metal API bindings
- `objc` -- Objective-C runtime interop
- `block` -- Objective-C block support (GPU completion handlers)
- `foreign-types` -- Safe wrappers for foreign type pointers
- `unison2d`, `unison-render`, `unison-input`, `unison-profiler` -- engine crates

### Swift

- `MetalKit` -- `MTKView` and `MTKViewDelegate`
- `UIKit` -- `UIViewController`, touch events
- `Metal` -- device, command queue
