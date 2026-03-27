# unison-input

Two-layer platform-agnostic input system.

## Layer 1: Raw Input State (`InputState`)

Platform crates feed native events into `InputState`. Game code usually reads actions (layer 2) instead.

### Keyboard

| Method | Description |
|--------|-------------|
| `is_key_pressed(key)` | Key is held this frame |
| `is_key_just_pressed(key)` | Key was pressed this frame (not held last frame) |
| `is_key_just_released(key)` | Key was released this frame |

### Mouse

| Method | Description |
|--------|-------------|
| `mouse_position()` | Current position in screen coords (`Vec2`) |
| `is_mouse_pressed(button)` | Button is held |
| `is_mouse_just_pressed(button)` | Button pressed this frame |
| `is_mouse_just_released(button)` | Button released this frame |

### Touch

| Method | Description |
|--------|-------------|
| `active_touches()` | All currently active touches |
| `touches_just_began()` | Touches that started this frame |
| `touches_just_ended()` | Touches that ended this frame |
| `get_touch(id)` | Get specific touch by ID |

### Axis

| Method | Description |
|--------|-------------|
| `axis()` | Current axis value (`Vec2`, each component -1.0 to 1.0) |

Used for analog input such as virtual joysticks on mobile. Continuous state — persists across frames until explicitly changed.

### Platform Mutation API

Platform crates call these to feed events:

```rust
input.key_pressed(KeyCode::Space);
input.key_released(KeyCode::Space);
input.mouse_moved(x, y);
input.mouse_button_pressed(MouseButton::Left);
input.mouse_button_released(MouseButton::Left);
input.touch_started(id, x, y);
input.touch_moved(id, x, y);
input.touch_ended(id);
input.touch_cancelled(id);
input.set_axis(x, y);             // virtual joystick / analog input
input.begin_frame(); // call at start of each frame
```

## Layer 2: Action Mapping (`ActionMap<A>`)

Maps raw inputs to game-defined actions. The `A` type parameter is the game's action enum (`A: Copy + Eq + Hash`).

### Construction

| Method | Description |
|--------|-------------|
| `ActionMap::new()` | Create a new empty action map with no bindings |
| `ActionMap::default()` | Same as `new()` (implements `Default`) |

### Binding

```rust
actions.bind_key(KeyCode::Space, Action::Jump);
actions.bind_mouse_button(MouseButton::Left, Action::Shoot);
actions.bind_touch_region(rect, Action::Jump); // for mobile
actions.clear_bindings();
```

### Querying

```rust
actions.is_action_active(action)       // held
actions.is_action_just_started(action) // pressed this frame
actions.is_action_just_ended(action)   // released this frame
actions.axis_value(negative, positive) // -1.0, 0.0, or 1.0
```

### Frame Update

```rust
actions.update(&input); // evaluate bindings against current input state
```

## Types

- `KeyCode` — Arrow keys, WASD, Space, Enter, Escape, Tab, Backspace, Shift/Ctrl/Alt, A-Z, Digit0-9
- `MouseButton` — Left, Right, Middle
- `TouchPhase` — Began, Moved, Stationary, Ended, Cancelled
- `Touch` — `{ id: u64, position: Vec2, phase: TouchPhase }`
- `InputBuffer` — Double-buffered input for fixed-timestep game loops (see below)

## Input Buffering (`InputBuffer`)

Double-buffered input state for platform game loops with fixed-timestep update ticks. Platform code pushes events into a **shared** buffer asynchronously; at the top of each frame the game loop calls `transfer`, which swaps shared into engine only when an update tick will actually run. This prevents per-frame events (`just_pressed` / `just_released`) from being consumed on a frame where no update processes them.

### Struct

```rust
pub struct InputBuffer {
    shared: InputState,  // platform event callbacks write here
    engine: InputState,  // game code reads from here during update ticks
}
```

### Construction

| Method | Description |
|--------|-------------|
| `InputBuffer::new()` | Create a new double buffer with empty input state on both sides |
| `InputBuffer::default()` | Same as `new()` (implements `Default`) |

### Methods

| Method | Description |
|--------|-------------|
| `shared_mut()` | Mutable access to the shared buffer — platform event handlers push events here |
| `transfer(will_update: bool)` | Swap shared→engine when `will_update` is true; otherwise events stay in shared and accumulate |
| `begin_tick()` | Clear per-frame flags on the engine buffer between fixed-timestep ticks (call before the 2nd+ tick in a frame) |
| `engine()` | Read access to the engine-side `InputState` — game code reads this during update ticks |
| `swap_into(target: &mut InputState)` | Swap the engine-side buffer into an external `InputState` without cloning |

### Usage Pattern

```rust
// Platform game loop (simplified):
let mut input_buffer = InputBuffer::new();

// Platform event callbacks push into the shared side:
input_buffer.shared_mut().key_pressed(KeyCode::Space);
input_buffer.shared_mut().mouse_moved(x, y);

// At the top of each frame:
let will_update = accumulator >= dt;
input_buffer.transfer(will_update);

// Fixed-timestep ticks:
while accumulator >= dt {
    let input = input_buffer.engine();
    // ... run game update using input ...
    input_buffer.begin_tick(); // clear just_pressed/just_released for next tick
    accumulator -= dt;
}
```

### Transfer Semantics

- When `will_update` is **true**: swaps shared↔engine, then resets the shared buffer while preserving continuous state (held keys, mouse position, active touches, axis) via `copy_held_from`.
- When `will_update` is **false**: does nothing — events stay in the shared buffer and accumulate until the next frame where a tick runs.

## Additional `InputState` API

### Construction

| Method | Description |
|--------|-------------|
| `InputState::new()` | Create a new empty input state (all keys up, no touches, axis at zero) |
| `InputState::default()` | Same as `new()` (implements `Default`) |

### Transfer

| Method | Description |
|--------|-------------|
| `copy_held_from(other: &InputState)` | Copy held-key, held-mouse-button, mouse position, active touches, and axis state from another `InputState`. Used after swapping input buffers so the shared buffer starts with correct held state. |
