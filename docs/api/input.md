# unison-input crate — raw input + Lua action map

## Overview

Two layers:

1. **Raw input** — `InputState`. Platform crates feed key, mouse, and touch
   events in each frame; game code reads the snapshot via `unison.input.*`.
2. **Lua action map** — `unison.input.bind_action`, `bind_axis`, and the
   `is_action_pressed` / `axis` queries. Implemented in `unison-scripting`;
   keeps game code free of hard-coded key constants.

## Supported keys

Key name strings accepted by `parse_key_code` (from
`crates/unison-scripting/src/bindings/input.rs`):

```
ArrowUp  ArrowDown  ArrowLeft  ArrowRight
Space  Enter  Escape  Tab  Backspace
ShiftLeft  ShiftRight  ControlLeft  ControlRight  AltLeft  AltRight
A B C D E F G H I J K L M N O P Q R S T U V W X Y Z   (single letter, case-insensitive)
Digit0 Digit1 Digit2 Digit3 Digit4 Digit5 Digit6 Digit7 Digit8 Digit9
0 1 2 3 4 5 6 7 8 9                                    (digit aliases)
```

## Mouse buttons

`0` = Left, `1` = Right, `2` = Middle

## Touch

| Method | Description |
|--------|-------------|
| `unison.input.touches_started()` | Array of `{x, y}` touch-start positions this frame |
| `unison.input.is_pointer_just_pressed()` | True if a touch started OR left mouse button was just pressed |
| `unison.input.pointer_position()` | `[x, y]` of active touch or held mouse; `[nil, nil]` when inactive |

## Action map

Bind named actions or axes in `init` (or `on_enter`), then query each frame:

```lua
-- Bind in init
unison.input.bind_action("jump", { keys = {"Space", "W"} })
unison.input.bind_action("fire", { keys = {"F"}, mouse_buttons = {0} })
unison.input.bind_axis("move_x", { negative = "ArrowLeft", positive = "ArrowRight",
                                    joystick_axis = "x" })

-- Query in update
if unison.input.is_action_just_pressed("jump") then
    -- ...
end
local vx = unison.input.axis("move_x")  -- -1.0 to 1.0
```

| Method | Description |
|--------|-------------|
| `bind_action(name, opts)` | Bind keys / mouse buttons to a named action |
| `bind_axis(name, opts)` | Bind negative/positive keys and optional joystick axis |
| `is_action_pressed(name)` | True while any bound input is held |
| `is_action_just_pressed(name)` | True only on the frame the action was first triggered |
| `is_action_just_released(name)` | True only on the frame all bound inputs were released |
| `axis(name)` | Digital axis value in `[-1, 1]`; includes raw joystick when bound |

## Raw queries

All on `unison.input`:

| Method | Description |
|--------|-------------|
| `is_key_pressed(key)` | True while the key is held |
| `is_key_just_pressed(key)` | True only on the frame the key was first pressed |
| `is_key_just_released(key)` | True only on the frame the key was released |
| `is_mouse_button_pressed(btn)` | True while mouse button is held |
| `is_mouse_button_just_pressed(btn)` | True on the frame the button was first pressed |
| `is_mouse_button_just_released(btn)` | True on the frame the button was released |
| `mouse_position()` | Current mouse position `[x, y]` in screen space |

## Internal: InputState

For engine contributors only. `InputState` is the double-buffered raw input
snapshot. Platform crates call `key_pressed`, `key_released`, `mouse_moved`,
`mouse_button_pressed`, `touch_started`, etc. to feed events in. `InputBuffer`
wraps two `InputState`s (shared and engine-side) to ensure per-frame events
(`just_pressed` / `just_released`) are not consumed on frames where no update
tick runs. See `crates/unison-input/src/` for details.
