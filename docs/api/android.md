# unison-android

Android platform crate -- OpenGL ES 3.0 renderer, touch input, JNI frame loop, and Kotlin host library. Depends on: `unison2d`, `unison-render`, `unison-input`, `unison-profiler`, `glow`, `jni`, `libc`.

## Usage

Games need exactly one line of Rust code to export to Android. Use `ScriptedGame` as the game type — it loads all gameplay from Lua at runtime:

```rust
// In your game crate (e.g., lib.rs):
#[cfg(feature = "android")]
unison_android::export_game!(
    unison_scripting::ScriptedGame,
    unison_scripting::ScriptedGame::from_asset("scripts/main.lua", assets::ASSETS)
);
```

The Kotlin host app (provided by the `UnisonAndroid` library module) handles GLSurfaceView setup, continuous rendering, and touch forwarding. The game crate compiles to a shared library (`.so`) that the Android app loads via `System.loadLibrary`.

## Architecture

```
Kotlin (UnisonAndroid library)           Rust (unison-android crate)
┌───────────────────────┐               ┌───────────────────────┐
│ GameActivity          │──JNI calls───▶│ export_game! macro    │
│   ├─ GameSurfaceView  │               │   ├─ gameInit         │
│   ├─ JoystickView     │               │   ├─ gameFrame        │
│   └─ touch forwarding │               │   └─ gameDestroy      │
└───────────────────────┘               │                       │
                                        │ GameState<G>          │
                                        │   ├─ Engine<A>        │
                                        │   ├─ GlesRenderer     │
                                        │   └─ InputBuffer      │
                                        └───────────────────────┘
```

## export_game! (macro)

Generates 9 `#[no_mangle] pub unsafe extern "system"` JNI entry points that bridge a concrete `Game` type to the Kotlin host app. Pass `ScriptedGame` as the game type.

```rust
unison_android::export_game!($game_type, $constructor);
```

**Arguments:**
- `$game_type` -- the concrete struct that implements `unison2d::Game` (use `ScriptedGame`)
- `$constructor` -- an expression that creates a new instance

**Generated JNI functions:**

| Function | Signature | Description |
|----------|-----------|-------------|
| `gameInit` | `(width, height) -> jlong` | Create renderer + game state; returns opaque pointer |
| `gameFrame` | `(state, dt)` | Run one frame: fixed-timestep updates + render |
| `gameResize` | `(state, width, height)` | Notify game of screen size change (dp-based) |
| `gameTouchBegan` | `(state, id, x, y)` | Feed touch-began event (ACTION_DOWN) |
| `gameTouchMoved` | `(state, id, x, y)` | Feed touch-moved event (ACTION_MOVE) |
| `gameTouchEnded` | `(state, id)` | Feed touch-ended event (ACTION_UP) |
| `gameTouchCancelled` | `(state, id)` | Feed touch-cancelled event (ACTION_CANCEL) |
| `gameSetAxis` | `(state, x, y)` | Set virtual joystick axis (-1.0 to 1.0) |
| `gameDestroy` | `(state)` | Drop the game state and free memory |

All functions are namespaced to `com.unison2d.UnisonNative` (JNI naming convention). The `gameInit` function wraps initialization in `catch_unwind` to prevent panics from aborting the process.

## GameState\<G\>

Owns the game, engine, renderer, and input buffer. One instance per app lifetime. Generic over any `Game` type -- the concrete type is supplied by the game crate via `export_game!`.

```rust
pub struct GameState<G: Game> {
    game: G,
    engine: Engine<G::Action>,
    input: InputBuffer,
    accumulator: f32,
    initialized: bool,
    gles_renderer: *mut GlesRenderer,  // raw pointer for GLES-specific calls
}
```

| Method | Description |
|--------|-------------|
| `new(renderer, game)` | Create game state; moves renderer into engine, keeps raw pointer for GLES-specific calls |
| `init()` | Initialize android_logger + profiler (monotonic clock), set fixed timestep, call `game.init()`. Idempotent |
| `input_mut()` | Access `&mut InputBuffer` for feeding touch/axis events from JNI |
| `engine_mut()` | Access `&mut Engine<A>` (e.g., to update screen size) |
| `frame(dt)` | Run one display frame (called from Kotlin on each `onDrawFrame` callback) |

### Frame Loop

`frame()` implements the same fixed-timestep accumulator as the web and iOS crates:

1. `Profiler::begin_frame()`
2. Accumulate `dt` (capped at 100ms to prevent spiral of death)
3. Guarantee at least one update per display frame (prevents UI flicker from timing jitter)
4. Transfer touch/input events into the engine's `InputState`
5. Fixed-timestep updates at 60Hz: `engine.pre_update()` + `game.update()` (multiple ticks if needed)
6. `gles_renderer.begin_display_frame()` -- bind default FBO, clear screen
7. `game.render(&mut engine)` -- engine render cycle (may call `begin_frame`/`end_frame`/`clear`/`bind_render_target` multiple times)
8. `gles_renderer.end_display_frame()` -- flush (GLSurfaceView handles eglSwapBuffers)
9. `Profiler::end_frame()` -- every 120 frames, log stats to logcat via `android_logger` and reset

Constants:

| Constant | Value | Description |
|----------|-------|-------------|
| `FIXED_DT` | `1.0 / 60.0` | Fixed timestep (60 updates/second) |
| `MAX_ACCUMULATOR` | `0.1` | Max accumulated time per frame (prevents spiral of death) |
| `PROFILER_LOG_INTERVAL` | `120` | Frames between profiler stat dumps to logcat |

## GlesRenderer

Implements the `Renderer` trait from `unison-render`. Maps platform-agnostic render commands to OpenGL ES 3.0 draw calls via the `glow` crate.

### Construction

```rust
let renderer = GlesRenderer::new(width, height)?;
```

Created by the `gameInit` JNI function. GL function pointers are loaded dynamically via `dlopen("libEGL.so")` + `eglGetProcAddress`, with `dlsym(RTLD_DEFAULT)` as fallback.

### Renderer Trait Implementation

All 6 `RenderCommand` variants are supported:

| Command | Implementation |
|---------|---------------|
| `Sprite` | Textured quad with rotation + UV |
| `Mesh` | Dynamic vertex/index upload, draw indexed triangles |
| `LitSprite` | Textured quad with shadow mask sampling + PCF filtering |
| `Rect` | Two-triangle filled quad |
| `Line` | Thin rectangle along line direction |
| `Terrain` | Fan triangulation of polygon |

| Trait Method | Description |
|--------------|-------------|
| `init()` | Set initial viewport from physical pixel dimensions |
| `begin_frame(camera)` | Compute 3x3 view-projection matrix, set on both shader programs |
| `clear(color)` | `glClearColor` + `glClear` |
| `draw(command)` | Dispatch to sprite/mesh/lit/rect/line/terrain draw methods |
| `end_frame()` | `glFlush` |
| `create_texture(desc)` | Create GL texture, upload pixels, configure filtering/wrapping, generate mipmaps |
| `destroy_texture(id)` | Delete GL texture |
| `screen_size()` | Returns dp-based dimensions (matches touch coordinate space) |
| `drawable_size()` | Returns pixel-based dimensions (physical GL surface size) |
| `fbo_origin_top_left()` | Returns `false` (OpenGL has origin at bottom-left) |
| `set_screen_size(w, h)` | Update dp-based dimensions only (pixel dimensions set at init) |
| `set_blend_mode(mode)` | Switch blend function (Alpha, Additive, Multiply) |
| `create_render_target(w, h)` | Create FBO + RGBA8 texture, with optional MSAA renderbuffer |
| `bind_render_target(target)` | Resolve previous MSAA FBO if needed, bind new FBO + viewport |
| `destroy_render_target(target)` | Delete FBOs and renderbuffers (avoids double-delete when no MSAA) |
| `set_anti_aliasing(mode)` | Set MSAA sample count (clamped to device `MAX_SAMPLES`) |
| `anti_aliasing()` | Current AA mode |

### Display Frame Lifecycle

These GLES-specific methods are called by `GameState`, not by the engine's `Renderer` trait:

| Method | Description |
|--------|-------------|
| `begin_display_frame()` | Bind default FBO, set viewport to physical pixel dimensions, clear to black |
| `end_display_frame()` | `glFlush` (GLSurfaceView handles `eglSwapBuffers` automatically) |

### Anti-Aliasing

MSAA defaults to **None** on Android. Budget Adreno/Mali GPUs take a significant performance hit from multisample renderbuffer storage and blit resolve. Games can opt in via `set_anti_aliasing()` for higher-end devices.

When MSAA is disabled (`msaa_samples <= 1`), render targets draw directly to the texture FBO -- no separate MSAA renderbuffer or blit resolve step. The `bind_render_target` and `destroy_render_target` methods handle both paths correctly.

### Coordinate System

Two separate size tracking values:

- **Point dimensions** (`screen_width_points`, `screen_height_points`) -- matches Android dp touch coordinates and game logic. Updated by `gameResize` / `set_screen_size`.
- **Pixel dimensions** (`screen_width`, `screen_height`) -- physical GL surface resolution. Set once at `gameInit` from the GLSurfaceView dimensions.

`screen_size()` returns points. `drawable_size()` returns pixels.

### Shader Programs

Two shader programs using GLSL ES 3.0 (identical to WebGL2 shaders):

| Program | Description |
|---------|-------------|
| Base | Solid color with optional texture sampling and per-vertex color |
| Lit | Light gradient texture x shadow mask with PCF5/PCF13 soft shadows |

Both share the same vertex shader (3x3 view-projection matrix transform). Per-vertex colors are disabled by default and use a constant `(1,1,1,1)`.

## Touch Input Helpers

Convenience functions in the `input` module that feed Android `MotionEvent` data into `InputBuffer`. Called from the JNI functions generated by `export_game!`.

| Function | Description |
|----------|-------------|
| `touch_began(input, id, x, y)` | Feed a touch-began event |
| `touch_moved(input, id, x, y)` | Feed a touch-moved event |
| `touch_ended(input, id)` | Feed a touch-ended event |
| `touch_cancelled(input, id)` | Feed a touch-cancelled event |
| `set_axis(input, x, y)` | Set virtual joystick axis value |

All functions delegate to `input.shared_mut()` methods on `InputBuffer`.

## GLSL ES 3.0 Shaders

Identical to the WebGL2 shaders in `unison-web` -- both target GLSL ES 3.0. Embedded as Rust string constants.

| Constant | Description |
|----------|-------------|
| `VERTEX_SHADER` | Transforms 2D positions by 3x3 view-projection matrix, passes through UV + vertex color |
| `FRAGMENT_SHADER` | Solid color with optional texture sampling and per-vertex color |
| `LIT_FRAGMENT_SHADER` | Lit sprite with shadow mask sampling + PCF5/PCF13 soft shadows |

### Vertex Layout

```glsl
layout(location = 0) in vec2 a_position;   // 8 bytes
layout(location = 1) in vec2 a_uv;         // 8 bytes
layout(location = 2) in vec4 a_vertex_color; // 16 bytes
```

## Kotlin Host Library (UnisonAndroid)

A Kotlin Android library module (`UnisonAndroid/`) that provides the Android integration layer. The game's Gradle project includes this as a module dependency.

```
UnisonAndroid/
├── build.gradle.kts
└── src/main/java/com/unison2d/
    ├── UnisonNative.kt        # JNI method declarations
    ├── GameActivity.kt        # Base Activity (GL surface + joystick + touch)
    ├── GameSurfaceView.kt     # GLSurfaceView with GLES 3.0 + frame loop
    └── JoystickView.kt        # Virtual joystick overlay
```

Platform requirement: Android API 24+ (Android 7.0).

### UnisonNative

Kotlin `object` declaring the 9 `external` JNI methods matching the Rust `export_game!` macro output. The actual function bodies are in the game's `.so` library (loaded by `System.loadLibrary` in `GameActivity`).

### GameActivity

`open class GameActivity : Activity()` -- base Activity for any Unison 2D game.

| Responsibility | Details |
|---------------|---------|
| Library loading | `System.loadLibrary(nativeLibraryName)` in `onCreate` |
| GLSurfaceView setup | Creates `GameSurfaceView` (GLES 3.0, continuous rendering) |
| Fullscreen immersive | `WindowInsetsController` (API 30+) or `SYSTEM_UI_FLAG_IMMERSIVE_STICKY` (older) |
| Touch forwarding | Overrides `onTouchEvent`, forwards multi-touch to `gameTouchBegan/Moved/Ended/Cancelled` via `queueEvent` |
| Joystick | Creates a `JoystickView` anchored to bottom-left with dp-based sizing |
| Lifecycle | Forwards `onPause`/`onResume` to GLSurfaceView, destroys game state in `onDestroy` |

Subclassable (`open class`) -- games override `nativeLibraryName` to specify their `.so`:

```kotlin
class MainActivity : GameActivity() {
    override val nativeLibraryName = "donut_game"
}
```

All JNI calls are dispatched to the GL thread via `queueEvent` to ensure thread safety.

### GameSurfaceView

`class GameSurfaceView : GLSurfaceView` -- drives the game loop via `GLSurfaceView.Renderer` callbacks.

| Callback | Description |
|----------|-------------|
| `onSurfaceCreated` | Initialize Rust game state via `gameInit(width, height)`, send dp dimensions via `gameResize` |
| `onSurfaceChanged` | Forward new dp dimensions via `gameResize` |
| `onDrawFrame` | Compute delta time from `System.nanoTime()`, call `gameFrame(state, dt)` |

Handles GL context loss by destroying the old game state and creating fresh in `onSurfaceCreated`.

### JoystickView

`class JoystickView : View` -- virtual joystick overlay for touch-based movement input.

| Property / Method | Description |
|-------------------|-------------|
| `onAxisChanged` | Lambda `(Float, Float) -> Unit` called when axis changes |
| `DEFAULT_SIZE_DP` | `120` dp (base circle diameter) |
| `PADDING_DP` | `22f` dp (extra padding so stroke/thumb aren't clipped) |
| `VIEW_SIZE_DP` | `164` dp (total view size including padding) |
| Touch handling | Tracks a single pointer, computes normalized axis from drag distance |
| Axis output | Horizontal axis (`-1.0` to `1.0`), vertical always `0` (side-scrolling platformer) |
| Visual | Semi-transparent base circle (15% white fill + 30% white stroke) + thumb circle (50% white) |

## Build System

### Cross-compilation

The game's `platform/android/build-rust.sh` script handles cross-compilation:

```bash
./build-rust.sh           # Release build (default)
./build-rust.sh debug     # Debug build
```

Uses `cargo-ndk` to build for `aarch64-linux-android` (arm64-v8a) and `x86_64-linux-android` (emulator). Copies the resulting `.so` files to `app/src/main/jniLibs/<abi>/`.

RUSTFLAGS include `-Clink-arg=-z -Clink-arg=max-page-size=16384` for 16KB page alignment (required by Android 15+).

### Gradle Integration

The app's `build.gradle.kts` includes a `buildRust` task that runs `build-rust.sh` automatically before packaging. Hitting Play in Android Studio recompiles Rust code (mirrors Xcode's Run Script build phase). Debug vs release profile is auto-detected from the Gradle task name.

## Dependencies

### Rust

- `glow` -- OpenGL ES 3.0 function bindings
- `jni` -- JNI interop (types and function signatures)
- `libc` -- `dlopen`/`dlsym` for loading EGL/GLES function pointers
- `log` + `android_logger` -- Rust logging routed to Android logcat
- `unison2d`, `unison-render`, `unison-input`, `unison-profiler` -- engine crates

### Kotlin

- `android.opengl.GLSurfaceView` -- GL surface with EGL context management
- `android.app.Activity` -- base Activity class
- `android.view.MotionEvent` -- touch event handling
