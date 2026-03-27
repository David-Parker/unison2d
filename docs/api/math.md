# unison-math

Shared math and color types used across all Unison 2D engine crates. Zero dependencies.

Accessible via `unison2d::math` or directly as `unison_math`.

## Types

### `Vec2`

2D vector with `f32` components.

```rust
use unison_math::Vec2;

let v = Vec2::new(3.0, 4.0);
let v = Vec2::splat(1.0);        // both components set to 1.0
```

**Constants:** `ZERO`, `ONE`, `UP`, `DOWN`, `LEFT`, `RIGHT`

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `const fn new(x: f32, y: f32) -> Vec2` | Create a new Vec2 |
| `splat` | `const fn splat(v: f32) -> Vec2` | Both components set to the same value |
| `length` | `fn length(self) -> f32` | Length of the vector |
| `length_squared` | `fn length_squared(self) -> f32` | Squared length (avoids sqrt) |
| `normalized` | `fn normalized(self) -> Vec2` | Unit vector; returns `ZERO` for zero-length |
| `dot` | `fn dot(self, other: Vec2) -> f32` | Dot product |
| `cross` | `fn cross(self, other: Vec2) -> f32` | 2D cross product (scalar z-component) |
| `distance` | `fn distance(self, other: Vec2) -> f32` | Euclidean distance to another point |
| `distance_squared` | `fn distance_squared(self, other: Vec2) -> f32` | Squared distance (avoids sqrt) |
| `lerp` | `fn lerp(self, other: Vec2, t: f32) -> Vec2` | Linear interpolation (`t=0` → self, `t=1` → other) |
| `min` | `fn min(self, other: Vec2) -> Vec2` | Per-component minimum |
| `max` | `fn max(self, other: Vec2) -> Vec2` | Per-component maximum |
| `clamp` | `fn clamp(self, min: Vec2, max: Vec2) -> Vec2` | Per-component clamp |
| `to_array` | `const fn to_array(self) -> [f32; 2]` | Convert to array |
| `to_tuple` | `const fn to_tuple(self) -> (f32, f32)` | Convert to tuple |

**Operators:** `+`, `-`, `*f32`, `f32*`, `/f32`, `-` (negate), with assign variants (`+=`, `-=`, `*=`, `/=`).

**Conversions** (all via `From`/`Into`):
- `[f32; 2]` ↔ `Vec2`
- `(f32, f32)` ↔ `Vec2`

### `Color`

RGBA color with `f32` components (0.0–1.0).

```rust
use unison_math::Color;

Color::rgb(1.0, 0.5, 0.0);           // orange, alpha=1
Color::new(1.0, 0.5, 0.0, 0.8);      // with alpha
Color::from_hex(0xFF8000);            // from hex
Color::from_rgba8(255, 128, 0, 255);  // from u8
```

**Constants:** `WHITE`, `BLACK`, `RED`, `GREEN`, `BLUE`, `TRANSPARENT`

**Default:** `Color::default()` returns `WHITE`.

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `const fn new(r: f32, g: f32, b: f32, a: f32) -> Color` | Create from RGBA (0.0–1.0) |
| `rgb` | `const fn rgb(r: f32, g: f32, b: f32) -> Color` | Create from RGB, alpha = 1.0 |
| `from_rgba8` | `fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Color` | Create from u8 values (0–255) |
| `from_hex` | `fn from_hex(hex: u32) -> Color` | From `0xRRGGBB` or `0xRRGGBBAA` |
| `to_array` | `fn to_array(self) -> [f32; 4]` | Convert to `[r, g, b, a]` array |
| `to_rgba8` | `fn to_rgba8(self) -> [u8; 4]` | Convert to `[u8; 4]` (0–255) |
| `to_rgb_tuple` | `const fn to_rgb_tuple(self) -> (f32, f32, f32)` | RGB tuple (discards alpha) |
| `lerp` | `fn lerp(self, other: Color, t: f32) -> Color` | Component-wise linear interpolation |

**Conversions** (all via `From`/`Into`):
- `[f32; 4]` ↔ `Color`
- `[f32; 3]` ↔ `Color` (alpha=1 on input, drops alpha on output)
- `(f32, f32, f32)` ↔ `Color` (alpha=1 on input, drops alpha on output)
- `(f32, f32, f32, f32)` → `Color`

### `Rect`

Axis-aligned rectangle defined by `min` and `max` corners (both `Vec2`).

```rust
use unison_math::{Rect, Vec2};

Rect::new(Vec2::ZERO, Vec2::new(10.0, 8.0));              // from min/max corners
Rect::from_center(Vec2::new(5.0, 5.0), Vec2::new(10.0, 8.0));
Rect::from_position(Vec2::ZERO, Vec2::new(10.0, 8.0));    // from bottom-left + size
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `const fn new(min: Vec2, max: Vec2) -> Rect` | Create from min/max corners |
| `from_center` | `fn from_center(center: Vec2, size: Vec2) -> Rect` | Create from center + full size |
| `from_position` | `fn from_position(position: Vec2, size: Vec2) -> Rect` | Create from bottom-left + size |
| `width` | `fn width(&self) -> f32` | Width of the rect |
| `height` | `fn height(&self) -> f32` | Height of the rect |
| `size` | `fn size(&self) -> Vec2` | Size as `(width, height)` Vec2 |
| `center` | `fn center(&self) -> Vec2` | Center point |
| `contains` | `fn contains(&self, point: Vec2) -> bool` | Point-in-rect test |
| `intersects_circle` | `fn intersects_circle(&self, center: Vec2, radius: f32) -> bool` | Circle-rect overlap test |
| `intersects` | `fn intersects(&self, other: &Rect) -> bool` | Rect-rect overlap test |

**Conversions:**
- `(f32, f32, f32, f32)` → `Rect` — maps `(min_x, min_y, max_x, max_y)`, compatible with `Camera::bounds()`

## Free Functions

### `lerp(a, b, t) -> f32`

Linear interpolation between two f32 values. `t=0` returns `a`, `t=1` returns `b`.

### `smoothstep(t) -> f32`

Hermite smoothstep: smooth ease-in/ease-out for `t` in [0, 1]. Returns 0 at `t=0`, 1 at `t=1`, with zero derivative at both endpoints.

```rust
use unison_math::{lerp, smoothstep};

lerp(0.0, 10.0, 0.5);    // 5.0
smoothstep(0.5);          // 0.5 (but non-linear)
```

## `Rng`

Deterministic xorshift32 pseudo-random number generator. Zero dependencies, suitable for procedural content, particle effects, and reproducible randomness.

```rust
use unison_math::Rng;

let mut rng = Rng::new(42);
rng.next();                    // raw u32
rng.range_f32(0.0, 1.0);      // f32 in [0, 1)
rng.range_u32(1, 7);           // u32 in [1, 7) (i.e., 1..6)
```

A seed of 0 is replaced with 1 (xorshift has a fixed point at 0).
