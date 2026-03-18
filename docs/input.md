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

### Platform Mutation API

Platform crates call these to feed events:

```rust
input.key_pressed(KeyCode::Space);
input.key_released(KeyCode::Space);
input.mouse_moved(x, y);
input.mouse_button_pressed(MouseButton::Left);
input.touch_started(id, x, y);
input.touch_moved(id, x, y);
input.touch_ended(id);
input.touch_cancelled(id);
input.begin_frame(); // call at start of each frame
```

## Layer 2: Action Mapping (`ActionMap<A>`)

Maps raw inputs to game-defined actions. The `A` type parameter is the game's action enum.

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
