# unison-ui crate — declarative UI

## Overview

React-like declarative UI. Build a handle once with `unison.UI.new()`, then
call `:frame(tree)` every render frame. The system diffs the tree, lays out
nodes, handles pointer input, and emits overlay draw commands.

## Creating a UI handle

```lua
local ui = unison.UI.new("fonts/DejaVuSans-Bold.ttf")
```

Pass the asset path for a TTF/OTF font bundled via `unison_assets`. Reuse the
handle across frames — do not recreate it every frame.

## Rendering a frame

Call `:frame(tree)` from the scene's (or game's) `render` callback:

```lua
ui:frame({
    { type = "panel", anchor = "center", padding = 16, children = {
        { type = "label", text = "Score: " .. score, font_size = 24 },
        { type = "button", text = "Quit", on_click = "menu_quit" },
    }},
})
```

## Node types

All nodes share an optional `visible` field (defaults to `true`).

| Type | Key props | Description |
|------|-----------|-------------|
| `column` | `anchor`, `gap`, `padding`, `children` | Vertical layout container |
| `row` | `anchor`, `gap`, `padding`, `children` | Horizontal layout container |
| `panel` | `anchor`, `padding`, `bg_color`, `children` | Panel with optional background |
| `label` | `text`, `font_size`, `font_color` | Text display |
| `button` | `text`, `on_click`, `width`, `height`, `font_size`, `font_color`, `bg_color` | Clickable button |
| `icon` | `texture` | Texture sprite (TextureId from `unison.assets.load_texture`) |
| `progress_bar` | `value`, `width`, `height` | Horizontal fill bar; `value` in `[0, 1]` |
| `spacer` | `value` | Fixed-size gap in pixels |

**Anchor values** (for root nodes):
`"top_left"`, `"top"`, `"top_right"`,
`"left"`, `"center"`, `"right"`,
`"bottom_left"`, `"bottom"`, `"bottom_right"`

## Events

Button clicks are emitted as named string events on `unison.events`:

```lua
-- In the UI tree:
{ type = "button", text = "Play", on_click = "menu_play_clicked" }

-- Subscribe anywhere (e.g. in on_enter):
unison.events.on("menu_play_clicked", function()
    unison.scenes.set(game_scene)
end)
```

## Internal: Rust UI system

The Lua `unison.UI.new()` factory wraps the Rust `unison-ui` crate. Under the
hood, `unison-ui` provides a typed `UiNode<E>` / `UiTree<E>` model, a diff
engine, a two-pass layout solver, per-widget animation state (enter/exit/hover),
and a glyph-atlas text renderer backed by `ab_glyph`. For engine-contributor
details, see the source at `crates/unison-ui/src/`.
