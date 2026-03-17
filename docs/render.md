# unison-render

Platform-agnostic rendering abstractions. Zero dependencies. Platform crates implement the `Renderer` trait.

## Renderer (trait)

The core abstraction that platform implementations must satisfy.

```rust
pub trait Renderer {
    type Error;

    fn init(&mut self) -> Result<(), Self::Error>;
    fn begin_frame(&mut self, camera: &Camera);
    fn clear(&mut self, color: Color);
    fn draw(&mut self, command: RenderCommand);
    fn end_frame(&mut self);
    fn create_texture(&mut self, desc: &TextureDescriptor) -> Result<TextureId, Self::Error>;
    fn destroy_texture(&mut self, id: TextureId);
    fn screen_size(&self) -> (f32, f32);
}
```

## RenderCommand

```rust
enum RenderCommand {
    Sprite(DrawSprite),
    Mesh(DrawMesh),
    Line { start: [f32; 2], end: [f32; 2], color: Color, width: f32 },
    Rect { position: [f32; 2], size: [f32; 2], color: Color },
    Terrain { points: Vec<(f32, f32)>, fill_color: Color, line_color: Color },
}
```

### DrawSprite

```rust
DrawSprite {
    texture: TextureId,
    position: [f32; 2],
    size: [f32; 2],
    rotation: f32,
    uv: [f32; 4],       // (min_u, min_v, max_u, max_v)
    color: Color,        // tint
}
```

### DrawMesh

```rust
DrawMesh {
    positions: Vec<f32>,  // flat [x0, y0, x1, y1, ...]
    uvs: Vec<f32>,        // flat [u0, v0, u1, v1, ...]
    indices: Vec<u32>,    // triangle indices
    texture: TextureId,   // or TextureId::NONE for solid color
    color: Color,
}
```

## Camera

2D orthographic camera.

```rust
let mut cam = Camera::new(20.0, 15.0); // viewport in world units
cam.set_position(x, y);
cam.translate(dx, dy);
cam.move_toward(target_x, target_y, smoothing);
cam.zoom = 2.0;       // 2x zoom
cam.rotation = 0.1;   // radians

cam.bounds()            // -> (min_x, min_y, max_x, max_y)
cam.is_visible(x, y)   // -> bool
cam.screen_to_world(sx, sy, screen_w, screen_h) // -> (f32, f32)
cam.world_to_screen(wx, wy, screen_w, screen_h) // -> (f32, f32)
```

Default: position (0, 0), viewport (20, 15), zoom 1.0.

## Color

```rust
Color::rgb(1.0, 0.5, 0.0)          // RGB, alpha = 1.0
Color::new(1.0, 0.5, 0.0, 0.8)    // RGBA
Color::from_rgba8(255, 128, 0, 200) // from u8
Color::from_hex(0xFF8000)           // from hex

// Presets
Color::WHITE, Color::BLACK, Color::RED, Color::GREEN, Color::BLUE, Color::TRANSPARENT

color.to_array()  // -> [f32; 4]
color.to_rgba8()  // -> [u8; 4]
```

## Texture

```rust
// Opaque handle
TextureId::NONE  // null/invalid
id.is_valid()    // -> bool

// Descriptor for creating textures
let desc = TextureDescriptor::new(256, 256, TextureFormat::Rgba8, pixel_data)
    .with_filter(TextureFilter::Linear)
    .with_wrap(TextureWrap::ClampToEdge);

desc.is_power_of_two() // -> bool
```

### TextureFormat

`R8`, `Rg8`, `Rgb8`, `Rgba8` (default). Use `format.bytes_per_pixel()`.

### TextureFilter

`Nearest` (pixelated), `Linear` (smooth, default), `LinearMipmap`.

### TextureWrap

`Repeat` (default), `ClampToEdge`, `MirroredRepeat`.

## Sprite

```rust
Sprite::from_texture(texture_id)
    .with_uv(min_u, min_v, max_u, max_v)
    .with_color(Color::RED)
    .with_pivot(0.5, 0.0)  // bottom-center
```

Pivot: (0,0) = bottom-left, (0.5, 0.5) = center, (1, 1) = top-right.

## SpriteSheet

```rust
let sheet = SpriteSheet::new(texture_id, tex_width, tex_height, frame_width, frame_height);
sheet.frame_uv(3)   // -> [f32; 4] UV for frame 3
sheet.sprite(3)      // -> Sprite for frame 3
sheet.columns        // auto-calculated
sheet.rows           // auto-calculated
sheet.frame_count    // auto-calculated
```
