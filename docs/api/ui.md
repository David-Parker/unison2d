# unison-ui

React-like declarative UI system. Game code describes a UI tree each frame; the system diffs, lays out, handles input, and renders.

## Quick Start

```rust
use unison2d::ui::facade::{Ui, OverlayTarget};
use unison2d::ui::node::{UiNode, UiTree};
use unison2d::ui::style::Anchor;
use unison2d::ui;

// In your level/game struct:
struct MyLevel {
    world: World,
    ui: Ui<Action>,
}

// In update():
let ui_input = self.ui.begin_frame(ctx.input, screen_size, ctx.dt);
self.ui.describe(ui! {
    column(anchor = Anchor::TopLeft, padding = 8.0, gap = 4.0) [
        label("Score: {}", self.score),
        label("HP: {}/{}", self.hp, self.max_hp),
    ]
    if self.paused {
        panel(anchor = Anchor::Center) [
            label("PAUSED"),
            button("Resume", on_click = Action::Resume),
            button("Quit", on_click = Action::Quit),
        ]
    }
}, &mut renderer);

// Events flow through EventSink → EventBus automatically.
// Use ctx.events.drain::<Action>() or register handlers.
if !ui_input.consumed_click { /* game input */ }

// In render():
self.ui.render(&mut self.world, &mut renderer);
self.world.auto_render(&mut renderer);
```

## `Ui<E>` Facade

| Method | Description |
|--------|-------------|
| `Ui::new(font_bytes, renderer, sink) -> Result<Self, String>` | Create UI with TTF/OTF font data and an `EventSink`. Scale factor derived from renderer's drawable/screen size. Click events are emitted into the sink. |
| `begin_frame(input, screen_size, dt) -> UiInputResult` | Process input against previous frame's layout, advance animations. Click events are emitted into the `EventSink`. Returns `UiInputResult`. |
| `describe(tree, renderer)` | Diff against previous tree, update widget state, compute layout. Also re-checks scale factor. |
| `render(world, renderer)` | Emit overlay render commands via `OverlayTarget`. Call before `world.auto_render()`. |
| `text_renderer() -> &mut TextRenderer` | Access the `TextRenderer` (e.g., for custom text measurement). |
| `layout() -> &Layout` | Access current `Layout` (for debugging or custom hit testing). |
| `state() -> &UiState` | Access widget `UiState` (for debugging or custom rendering). |

### Event Routing

`Ui<E>` requires an `EventSink` at construction. Click events are always emitted into the sink.

```rust
// Option A: Use the factory (creates UI + wires sink to EventBus automatically)
let ui = engine.create_ui::<MenuAction>(font_bytes)?;
// or from a Ctx:
let ui = ctx.create_ui::<MenuAction>(font_bytes)?;

// Option B: Create with a standalone sink (for pull-based consumption)
let sink = EventSink::new();
let ui = Ui::new(font_bytes, renderer, sink.clone())?;
// Later: drain events directly from the sink

// Option C: Wire to the EventBus manually
let ui = Ui::new(font_bytes, renderer, bus.create_sink())?;
```

When wired to the `EventBus`, click events can be handled with `ctx.events.on::<MenuAction>(...)` or pulled with `ctx.events.drain::<MenuAction>()`.

## `UiInputResult`

```rust
pub struct UiInputResult {
    pub consumed_click: bool,
    pub consumed_hover: bool,
}
```

| Field | Type | Description |
|-------|------|-------------|
| `consumed_click` | `bool` | UI consumed a mouse click this frame |
| `consumed_hover` | `bool` | Mouse is hovering over any interactive UI element |

## `UiTree<E>`

A root-level UI description: one or more anchored node trees.

```rust
pub struct UiTree<E> {
    pub roots: Vec<UiNode<E>>,
}
```

| Method | Description |
|--------|-------------|
| `UiTree::new(roots: Vec<UiNode<E>>)` | Create a tree from root nodes |
| `UiTree::empty()` | Create an empty tree (no roots) |

## `UiNode<E>`

A node in the declarative UI tree. All fields are public.

```rust
pub struct UiNode<E> {
    pub kind: WidgetKind<E>,
    pub children: Vec<UiNode<E>>,
    pub anchor: Option<Anchor>,
    pub padding: EdgeInsets,
    pub gap: f32,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub text_style: Option<TextStyle>,
    pub panel_style: Option<PanelStyle>,
    pub nine_slice: Option<NineSlice>,
    pub key: Option<u64>,
    pub visible: bool,
}
```

## `WidgetKind<E>`

```rust
pub enum WidgetKind<E> {
    Column,
    Row,
    Panel,
    Label { text: String },
    Button { text: String, on_click: Option<E> },
    Icon { texture: TextureId },
    ProgressBar { value: f32 },
    Spacer { size: f32 },
}
```

| Method | Description |
|--------|-------------|
| `tag() -> &'static str` | Returns a string tag for this widget kind (used by diffing): `"column"`, `"row"`, `"panel"`, `"label"`, `"button"`, `"icon"`, `"progress_bar"`, `"spacer"` |

## Widget Constructors

| Widget | Constructor | Description |
|--------|------------|-------------|
| Column | `UiNode::column()` | Vertical layout container |
| Row | `UiNode::row()` | Horizontal layout container |
| Panel | `UiNode::panel()` | Visual container with background (default `PanelStyle` applied) |
| Label | `UiNode::label(text)` | Text display |
| Button | `UiNode::button(text)` | Clickable button with text |
| Icon | `UiNode::icon(texture)` | Texture sprite |
| ProgressBar | `UiNode::progress_bar(value)` | Fill bar (value clamped to 0.0..1.0) |
| Spacer | `UiNode::spacer(size)` | Fixed-size gap |

## Builder Methods

All widgets support:

| Method | Description |
|--------|-------------|
| `.with_children(Vec<UiNode<E>>)` | Set child nodes |
| `.with_anchor(Anchor)` | Screen anchor (root nodes only) |
| `.with_padding(impl Into<EdgeInsets>)` | Internal padding (accepts `f32` for uniform or `EdgeInsets`) |
| `.with_gap(f32)` | Child spacing (Column/Row) |
| `.with_width(f32)` | Explicit width override |
| `.with_height(f32)` | Explicit height override |
| `.with_text_style(TextStyle)` | Text font size/color |
| `.with_panel_style(PanelStyle)` | Panel background/border |
| `.with_nine_slice(NineSlice)` | 9-slice texture background |
| `.with_visible(bool)` | Visibility toggle |
| `.with_key(u64)` | Explicit diff identity key |
| `.with_on_click(E)` | Button click event (Button only, no-op on other widget kinds) |

## `ui!` Macro

```rust
ui! {
    column(anchor = Anchor::TopLeft, padding = 8.0, gap = 4.0) [
        label("Static text"),
        label("Dynamic: {}", value),
        row(gap = 8.0) [
            icon(heart_texture, width = 16.0),
            label("x3"),
        ],
        button("Click", on_click = Action::Click),
        progress_bar(0.75),
        spacer(10.0),
    ]
    if show_menu {
        panel(anchor = Anchor::Center, style = menu_style) [
            button("Resume", on_click = Action::Resume),
        ]
    }
}
```

### Macro Property Names

Properties available in the macro's `( ... )` section:

| Property | Maps to |
|----------|---------|
| `anchor = expr` | `node.anchor` |
| `padding = expr` | `node.padding` (via `EdgeInsets::from`) |
| `gap = expr` | `node.gap` |
| `width = expr` | `node.width` |
| `height = expr` | `node.height` |
| `style = expr` | `node.panel_style` |
| `text_style = expr` | `node.text_style` |
| `nine_slice = expr` | `node.nine_slice` |
| `key = expr` | `node.key` |
| `visible = expr` | `node.visible` |
| `on_click = expr` | Button-only: `WidgetKind::Button { on_click }` |

Conditionals: `if condition { ... }` and `if let pattern = expr { ... }` are supported at any level.

## Anchors

9 anchor points for root-level positioning:

```rust
#[derive(Default)]
pub enum Anchor {
    #[default]
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}
```

| Anchor | Position |
|--------|----------|
| `TopLeft` | Top-left corner (default) |
| `TopCenter` | Top center |
| `TopRight` | Top-right corner |
| `CenterLeft` | Left center |
| `Center` | Screen center |
| `CenterRight` | Right center |
| `BottomLeft` | Bottom-left corner |
| `BottomCenter` | Bottom center |
| `BottomRight` | Bottom-right corner |

## Styles

### `EdgeInsets`

```rust
pub struct EdgeInsets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}
```

| Method | Description |
|--------|-------------|
| `EdgeInsets::all(v)` | Uniform insets on all sides |
| `EdgeInsets::xy(x, y)` | Symmetric insets (horizontal, vertical) |
| `horizontal()` | Total horizontal inset (`left + right`) |
| `vertical()` | Total vertical inset (`top + bottom`) |

Implements `From<f32>` (converts to `EdgeInsets::all(v)`).

### `TextStyle`

```rust
pub struct TextStyle {
    pub font_size: f32,   // default: 16.0
    pub color: Color,     // default: Color::WHITE
    pub bold: bool,       // default: false
}
```

```rust
TextStyle::new().font_size(24.0).color(Color::WHITE).bold()
```

### `PanelStyle`

```rust
pub struct PanelStyle {
    pub background: Color,    // default: Color::new(0.1, 0.1, 0.1, 0.8)
    pub border_color: Color,  // default: Color::TRANSPARENT
    pub border_width: f32,    // default: 0.0
}
```

```rust
PanelStyle::new()
    .background(Color::new(0.1, 0.1, 0.1, 0.8))
    .border(Color::WHITE, 2.0)
```

### `NineSlice`

```rust
pub struct NineSlice {
    pub texture: TextureId,
    pub border: EdgeInsets,       // non-stretching regions (in texels)
    pub texture_width: f32,       // full texture width in pixels
    pub texture_height: f32,      // full texture height in pixels
}
```

```rust
NineSlice {
    texture: my_texture,
    border: EdgeInsets::all(8.0),
    texture_width: 32.0,
    texture_height: 32.0,
}
```

## Diff Engine

The diff engine compares two `UiTree`s to detect structural and content changes.

### `NodeKey`

Identifies a node across frames by its position in the tree.

```rust
pub struct NodeKey {
    pub path: Vec<u16>,
}
```

| Method | Description |
|--------|-------------|
| `NodeKey::root(index: u16)` | Key for a root-level node |
| `child(index: u16) -> NodeKey` | Derive a child key by appending an index |

### `DiffOp`

```rust
pub enum DiffOp {
    Added(NodeKey),      // node exists in new tree but not old
    Removed(NodeKey),    // node exists in old tree but not new
    Updated(NodeKey),    // same widget kind, but content changed
    Unchanged(NodeKey),  // identical across frames
}
```

### `diff_trees()`

```rust
pub fn diff_trees<E>(prev: &UiTree<E>, next: &UiTree<E>) -> Vec<DiffOp>
```

Compares root-by-root, recursing into children. Different widget kinds at the same position produce `Removed` + `Added`. Same kind with different content (text, value, texture) produces `Updated`. Containers are compared structurally (children diffed separately).

## Widget State

### `WidgetState`

Per-widget state that persists across frames.

```rust
pub struct WidgetState {
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
    pub animation_time: f32,      // seconds since creation
    pub exit_time: Option<f32>,   // remaining exit animation time (None = not exiting)
    pub hover_time: f32,          // 0.0..1.0 hover interpolation
}
```

| Method | Description |
|--------|-------------|
| `WidgetState::new()` | Fresh state (all zeroed/false) |
| `is_exiting() -> bool` | Whether this widget is in its exit animation |

### `UiState`

Manages state for all active widgets.

| Method | Description |
|--------|-------------|
| `UiState::new()` | Create empty state |
| `get(key) -> Option<&WidgetState>` | Get state for a widget |
| `get_mut(key) -> Option<&mut WidgetState>` | Get mutable state for a widget |
| `apply_diff(ops)` | Apply diff operations: create state on `Added`, start exit animation on `Removed` |
| `update(dt)` | Advance animation timers, hover interpolation, purge completed exit animations |
| `len() -> usize` | Number of active widget states (including exiting) |
| `is_empty() -> bool` | Whether there are no active states |
| `all_keys() -> Vec<NodeKey>` | Get all currently tracked node keys |
| `clear_input_state()` | Reset hover state for all widgets (called at start of each frame) |

### Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `EXIT_DURATION` | `0.12` | Exit (fade-out) animation duration in seconds |

## Animations

Built-in animations with no configuration needed:

| Animation | Duration | Behavior |
|-----------|----------|----------|
| Enter (fade-in) | 0.15s | Alpha 0→1 on widget creation |
| Exit (fade-out) | 0.12s | Alpha 1→0 on widget removal |
| Hover | ~0.125s | Color interpolation on mouse hover (speed = 8.0/s) |

### `AnimationParams`

Computed animation values for a single widget, used by the renderer.

```rust
pub struct AnimationParams {
    pub alpha: f32,     // 0.0 = invisible, 1.0 = fully visible
    pub hover_t: f32,   // 0.0 = not hovered, 1.0 = fully hovered
}
```

### `compute_animation()`

```rust
pub fn compute_animation(state: Option<&WidgetState>) -> AnimationParams
```

Computes visual parameters from widget state timers. Final alpha = enter_alpha * exit_alpha. Returns defaults (alpha 1.0, hover_t 0.0) when state is `None`.

### Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `ENTER_DURATION` | `0.15` | Enter (fade-in) animation duration in seconds |

## Layout

Two-pass layout algorithm: measure (bottom-up) then position (top-down).

### `LayoutRect`

A laid-out widget with computed position and size in pixel coordinates. Origin is top-left, Y increases downward.

```rust
pub struct LayoutRect {
    pub node_index: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
```

| Method | Description |
|--------|-------------|
| `contains(px, py) -> bool` | Check if a point (pixel coords, origin top-left) is inside this rect |

### `Layout`

```rust
pub struct Layout {
    pub rects: Vec<LayoutRect>,
}
```

Flat list of `LayoutRect`s in pre-order tree traversal order (same order as a depth-first walk of the `UiTree`).

### `TextMeasurer` Trait

```rust
pub trait TextMeasurer {
    fn measure(&mut self, text: &str, font_size: f32) -> Vec2;
}
```

Trait for measuring text pixel size. `TextRenderer` implements this internally. Tests can use a mock.

### `compute_layout()`

```rust
pub fn compute_layout<E>(
    tree: &UiTree<E>,
    screen_size: Vec2,
    measurer: &mut dyn TextMeasurer,
) -> Layout
```

Computes layout for an entire UI tree. Each root is positioned by its `Anchor`. Children are laid out in Column (vertical) or Row (horizontal) direction. Invisible nodes are skipped but still advance the index counter.

### Default Widget Sizes

| Widget | Default Size |
|--------|-------------|
| Icon | 24 x 24 px |
| ProgressBar | 120 x 16 px |
| Button | text size + 12px horizontal / 6px vertical padding, min height 32px |

## Input Handling

### `process_input()`

```rust
pub fn process_input<E: Clone>(
    tree: &UiTree<E>,
    layout: &Layout,
    ui_state: &mut UiState,
    input: &InputState,
    screen_size: Vec2,
) -> (UiInputResult, Vec<E>)
```

Hit-tests the mouse (or first touch) against laid-out interactive widgets. Updates hover/press state and returns triggered click events plus input consumption info. Touch input is treated as a virtual cursor (first touch = left click). Only `Button` widgets are considered interactive.

## Render Pipeline

### `OverlayCommand`

```rust
pub struct OverlayCommand {
    pub command: RenderCommand,
    pub z_order: i32,
}
```

### `render_ui()`

```rust
pub fn render_ui<E>(
    tree: &UiTree<E>,
    layout: &Layout,
    ui_state: &UiState,
    screen_size: Vec2,
    text_renderer: &mut TextRenderer,
    renderer: &mut dyn Renderer<Error = String>,
) -> Vec<OverlayCommand>
```

Walks the UI tree alongside layout rects, emitting `OverlayCommand`s with automatically increasing z-order. Applies enter/exit/hover animation alpha to all colors. Column, Row, and Spacer produce no visual output. Coordinates are converted from pixel space to overlay space (0..1, bottom-left origin, Y-up).

### `render_nine_slice()`

```rust
pub fn render_nine_slice(
    nine: &NineSlice,
    x: f32, y: f32,
    width: f32, height: f32,
    color: Color,
) -> Vec<DrawSprite>
```

Generates up to 9 `DrawSprite` commands for a 9-slice panel. Corners are fixed size, edges stretch in one direction, center stretches both. Borders are clamped to half the target size to avoid negative inner dimensions. Sprites are in pixel space (caller converts to overlay).

## Text Rendering

### `TextRenderer`

Handles text measurement and rendering via a glyph atlas backed by `ab_glyph`.

| Method | Description |
|--------|-------------|
| `TextRenderer::new(font_bytes, scale_factor, renderer) -> Result<Self, String>` | Create from raw TTF/OTF bytes. `scale_factor` is the device pixel ratio. |
| `set_scale_factor(scale_factor, renderer) -> Result<(), String>` | Update device scale factor. Clears glyph cache so glyphs re-rasterize at new resolution. |
| `atlas_texture() -> TextureId` | The atlas GPU texture ID |
| `measure(text, font_size, renderer) -> Vec2` | Measure pixel dimensions of a string (ensures glyphs are cached) |
| `render_text(text, position, font_size, color, renderer) -> Vec<RenderCommand>` | Generate `Sprite` render commands for each visible glyph. `position` is top-left in pixels. |

### `FontData`

Loaded font data with metric queries (wraps `ab_glyph::FontArc`).

| Method | Description |
|--------|-------------|
| `FontData::from_bytes(bytes) -> Result<Self, String>` | Load from raw TTF/OTF bytes |
| `font() -> &FontArc` | Access underlying ab_glyph font |
| `scaled(font_size) -> PxScaleFont` | Get a scaled font for the given pixel size |
| `glyph_id(c: char) -> GlyphId` | Look up glyph ID for a character |
| `advance_width(glyph_id, font_size) -> f32` | Horizontal advance width |
| `line_height(font_size) -> f32` | Line height (ascent - descent) |
| `ascent(font_size) -> f32` | Distance from baseline to top |
| `descent(font_size) -> f32` | Distance from baseline to bottom (negative) |
| `kern(a, b, font_size) -> f32` | Kerning adjustment between two glyphs |

### `GlyphAtlas`

Shelf-packed glyph texture atlas. Glyphs are rasterized on demand at `font_size * scale_factor` for crisp HiDPI text. Auto-doubles in size when full.

| Method | Description |
|--------|-------------|
| `GlyphAtlas::new(width, height, scale_factor, renderer) -> Result<Self, String>` | Create with initial dimensions (default 512x512) |
| `set_scale_factor(scale_factor, renderer) -> Result<(), String>` | Update scale factor, clears cache |
| `texture() -> TextureId` | Atlas GPU texture ID |
| `size() -> (u32, u32)` | Current atlas dimensions |
| `glyph_count() -> usize` | Number of cached glyphs |
| `get_or_insert(font, glyph_id, font_size, renderer) -> Result<&GlyphEntry, String>` | Look up or rasterize and cache a glyph |

### `GlyphEntry`

A cached glyph entry in the atlas.

```rust
pub struct GlyphEntry {
    pub uv: [f32; 4],       // [min_u, min_v, max_u, max_v] in atlas
    pub offset_x: f32,      // offset from pen position to glyph top-left (pixels)
    pub offset_y: f32,
    pub width: f32,          // rasterized bitmap size in logical pixels
    pub height: f32,
}
```

## Architecture

```
Game code (ui! macro → UiTree)
    │
    ▼
Ui<E> facade
    ├── Diff engine     — diff_trees() → DiffOp list
    ├── Widget state    — UiState: hover, press, animation timers
    ├── Layout solver   — compute_layout() → LayoutRect list
    ├── Input handler   — process_input() → events + consumption
    ├── Animation       — compute_animation() → alpha, hover_t
    ├── Text renderer   — ab_glyph glyph atlas → DrawSprite commands
    ├── 9-slice         — render_nine_slice() → 9 DrawSprite commands
    └── Render emitter  — render_ui() → OverlayCommand list
```

## Coordinate Systems

- **Layout**: Pixel space, origin top-left, Y-down (matching CSS/DOM)
- **Overlay**: Normalized 0..1, origin bottom-left, Y-up (matching `World::draw_overlay()`)
- **Conversion**: The render pipeline handles pixel→overlay automatically

## `OverlayTarget`

The `OverlayTarget` trait allows `Ui::render()` to work with `World` or any mock:

```rust
pub trait OverlayTarget {
    fn draw_overlay(&mut self, command: RenderCommand, z_order: i32);
}
```

```rust
impl OverlayTarget for World {
    fn draw_overlay(&mut self, command: RenderCommand, z_order: i32) { ... }
}
```

## Module Structure

All modules are public (re-exported from crate root):

| Module | Description |
|--------|-------------|
| `facade` | `Ui<E>` facade, `OverlayTarget` trait |
| `node` | `UiNode<E>`, `UiTree<E>`, `WidgetKind<E>` |
| `style` | `Anchor`, `EdgeInsets`, `TextStyle`, `PanelStyle`, `NineSlice` |
| `input` | `UiInputResult`, `process_input()` |
| `state` | `WidgetState`, `UiState`, `EXIT_DURATION` |
| `diff` | `NodeKey`, `DiffOp`, `diff_trees()` |
| `layout` | `LayoutRect`, `Layout`, `TextMeasurer`, `compute_layout()` |
| `animation` | `AnimationParams`, `compute_animation()`, `ENTER_DURATION` |
| `render` | `OverlayCommand`, `render_ui()` |
| `nine_slice` | `render_nine_slice()` |
| `text` | `TextRenderer` |
| `text::font` | `FontData` |
| `text::atlas` | `GlyphAtlas`, `GlyphEntry` |
| `ui_macro` | `ui!` macro (macros exported at crate root) |
