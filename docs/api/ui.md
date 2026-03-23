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

for event in self.ui.drain_events() {
    match event { Action::Resume => ..., Action::Quit => ... }
}
if !ui_input.consumed_click { /* game input */ }

// In render():
self.ui.render(&mut self.world, &mut renderer);
self.world.auto_render(&mut renderer);
```

## `Ui<E>` Facade

| Method | Description |
|--------|-------------|
| `Ui::new(font_bytes, renderer)` | Create UI with TTF/OTF font data |
| `begin_frame(input, screen_size, dt)` | Process input, advance animations. Returns `UiInputResult` |
| `describe(tree, renderer)` | Diff, update state, compute layout |
| `drain_events()` | Consume triggered events (`Vec<E>`) |
| `render(world, renderer)` | Emit overlay render commands |
| `text_renderer()` | Access the `TextRenderer` |
| `layout()` | Access current `Layout` |
| `state()` | Access widget `UiState` |

## `UiInputResult`

| Field | Type | Description |
|-------|------|-------------|
| `consumed_click` | `bool` | UI consumed a mouse click this frame |
| `consumed_hover` | `bool` | Mouse is over an interactive UI element |

## Widget Types

| Widget | Constructor | Description |
|--------|------------|-------------|
| Column | `UiNode::column()` | Vertical layout container |
| Row | `UiNode::row()` | Horizontal layout container |
| Panel | `UiNode::panel()` | Visual container with background |
| Label | `UiNode::label(text)` | Text display |
| Button | `UiNode::button(text)` | Clickable button with `on_click` event |
| Icon | `UiNode::icon(texture)` | Texture sprite |
| ProgressBar | `UiNode::progress_bar(value)` | Fill bar (0.0..1.0) |
| Spacer | `UiNode::spacer(size)` | Fixed-size gap |

## Builder Methods

All widgets support:

| Method | Description |
|--------|-------------|
| `.with_anchor(Anchor)` | Screen anchor (root nodes only) |
| `.with_padding(f32)` | Internal padding |
| `.with_gap(f32)` | Child spacing (Column/Row) |
| `.with_width(f32)` | Explicit width override |
| `.with_height(f32)` | Explicit height override |
| `.with_text_style(TextStyle)` | Text font size/color |
| `.with_panel_style(PanelStyle)` | Panel background/border |
| `.with_nine_slice(NineSlice)` | 9-slice texture background |
| `.with_visible(bool)` | Visibility toggle |
| `.with_key(u64)` | Explicit diff identity key |
| `.with_on_click(E)` | Button click event (Button only) |

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

## Anchors

9 anchor points for root-level positioning:

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

### TextStyle

```rust
TextStyle::new().font_size(24.0).color(Color::WHITE).bold()
```

### PanelStyle

```rust
PanelStyle::new()
    .background(Color::new(0.1, 0.1, 0.1, 0.8))
    .border(Color::WHITE, 2.0)
```

### NineSlice

```rust
NineSlice {
    texture: my_texture,
    border: EdgeInsets::all(8.0),
    texture_width: 32.0,
    texture_height: 32.0,
}
```

## Animations

Built-in animations with no configuration needed:

| Animation | Duration | Behavior |
|-----------|----------|----------|
| Enter (fade-in) | 0.15s | Alpha 0→1 on widget creation |
| Exit (fade-out) | 0.12s | Alpha 1→0 on widget removal |
| Hover | ~0.125s | Color interpolation on mouse hover |

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

## OverlayTarget

The `OverlayTarget` trait allows `Ui::render()` to work with `World` or any mock:

```rust
impl OverlayTarget for World {
    fn draw_overlay(&mut self, command: RenderCommand, z_order: i32) { ... }
}
```
