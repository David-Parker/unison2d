# unison-math

Shared math and color types used across all Unison 2D engine crates. Zero dependencies.

Accessible via `unison2d::math` or directly as `unison_math`.

## Types

### `Vec2`

2D vector with `f32` components.

```rust
use unison_math::Vec2;

let v = Vec2::new(3.0, 4.0);
v.length();           // 5.0
v.normalized();       // unit vector
v.dot(other);         // dot product
v.cross(other);       // 2D cross (scalar)
v.distance(other);    // euclidean distance
v.lerp(other, 0.5);   // linear interpolation
v.clamp(min, max);    // per-component clamp
```

**Constants:** `ZERO`, `ONE`, `UP`, `DOWN`, `LEFT`, `RIGHT`

**Operators:** `+`, `-`, `*f32`, `/f32`, `-` (negate), with assign variants.

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

**Conversions** (all via `From`/`Into`):
- `[f32; 4]` ↔ `Color`
- `[f32; 3]` → `Color` (alpha=1)
- `(f32, f32, f32)` ↔ `Color` (alpha=1)
- `(f32, f32, f32, f32)` → `Color`

**Methods:** `to_array()`, `to_rgba8()`, `to_rgb_tuple()`

### `Rect`

Axis-aligned rectangle defined by `min` and `max` corners (both `Vec2`).

```rust
use unison_math::{Rect, Vec2};

Rect::from_center(Vec2::new(5.0, 5.0), Vec2::new(10.0, 8.0));
Rect::from_position(Vec2::ZERO, Vec2::new(10.0, 8.0));

rect.contains(point);                // point-in-rect
rect.intersects_circle(center, r);   // circle overlap
rect.intersects(&other_rect);        // rect-rect overlap
rect.width(); rect.height();
rect.center(); rect.size();
```

**Conversions:**
- `(f32, f32, f32, f32)` → `Rect` — maps `(min_x, min_y, max_x, max_y)`, compatible with `Camera::bounds()`
