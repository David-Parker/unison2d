# unison-scripting

Lua 5.4 scripting for Unison 2D. Implements the `Game` trait internally, forwarding lifecycle calls into an embedded Lua VM. Game code is written in Lua (or TypeScript via [TypeScriptToLua](https://github.com/TypeScriptToLua/TypescriptToLua)) rather than Rust.

## Purpose

- Embed a full Lua 5.4 VM in the game binary (vendored C source, no system Lua required)
- Implement `Game` trait so the scripting layer is a drop-in replacement for Rust game code
- Expose engine functionality to Lua via registered globals (`engine`, `input`, `world`, etc.)
- Support all three platforms: Web (wasm32), iOS (aarch64-apple-ios), Android

## Key Types

### `ScriptedGame`

```rust
pub struct ScriptedGame { /* ... */ }

impl ScriptedGame {
    pub fn new(script_src: impl Into<String>) -> Self;
}

impl Game for ScriptedGame {
    type Action = NoAction;
    fn init(&mut self, engine: &mut Engine<NoAction>);
    fn update(&mut self, engine: &mut Engine<NoAction>);
    fn render(&mut self, engine: &mut Engine<NoAction>);
}
```

`ScriptedGame` owns the Lua VM. Pass it to a platform's `run()` function just like any other `Game` implementation.

### `NoAction`

```rust
pub enum NoAction {}
```

Unit action enum for scripted games. Scripted games query input directly via the `input` Lua global rather than using Rust action mapping.

## Lua Lifecycle

The script passed to `ScriptedGame::new()` is executed once during `init()`. It **must return a table** with at least `init`, `update`, and `render` keys. Missing functions are silently ignored (no panic).

```lua
local game = {}

function game.init()
    -- Called once after engine init. Set up world, load assets.
    engine.set_background(0.1, 0.1, 0.12)
end

function game.update(dt)
    -- Called every fixed timestep. dt is the delta in seconds.
end

function game.render()
    -- Called every frame. Draw calls are buffered and submitted after return.
    engine.draw_rect(0, 0, 2, 2, 1, 0.2, 0.2)
end

return game
```

## Engine Globals (Phase 1b — minimal bridge)

These are registered before the script runs:

| Global | Signature | Description |
|--------|-----------|-------------|
| `engine.set_background` | `(r, g, b: number)` | Set the clear color (0–1 per channel) |
| `engine.draw_rect` | `(x, y, w, h, r, g, b: number)` | Draw a colored rectangle in world space |
| `engine.screen_size` | `() → (width, height: number)` | Get current screen dimensions |

Additional globals (`World`, `input`, `camera`, textures, events, scenes) are added in Phase 2+.

## Error Handling

- **Syntax errors** in the script: logged to stderr, `init`/`update`/`render` become no-ops.
- **Runtime errors** in lifecycle functions: logged to stderr, game continues.
- Neither type causes a panic.

## WASM Notes

Compiling for `wasm32-unknown-unknown` requires LLVM clang (Apple Clang lacks the WebAssembly backend):

```
brew install llvm
```

The `CC_wasm32_unknown_unknown` env var is pre-configured in the root `.cargo/config.toml`. A patched `lua-src` (at `vendor/lua-src/`) adds `wasm32` build support and includes a minimal libc sysroot (`vendor/lua-src/wasm-sysroot/`).

## Script Loading

Scripts are loaded from embedded assets at runtime. Place Lua scripts in `project/assets/scripts/` — they are embedded at build time by `build.rs`. The entry point is `scripts/main.lua`.

```rust
// In project/lib.rs:
let script = assets::ASSETS.iter()
    .find(|(path, _)| *path == "scripts/main.lua")
    .map(|(_, bytes)| std::str::from_utf8(bytes).unwrap().to_string())
    .unwrap_or_default();
let game = ScriptedGame::new(script);
```
