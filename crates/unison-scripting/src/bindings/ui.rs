//! UI bindings — declarative UI from Lua tables.
//!
//! ```lua
//! local ui = engine.create_ui("fonts/DejaVuSans-Bold.ttf")
//!
//! -- In render:
//! ui:frame({
//!     { type = "column", anchor = "center", gap = 10, children = {
//!         { type = "label", text = "Donut Game", font_size = 48 },
//!         { type = "button", text = "Play", on_click = "start_game",
//!           width = 200, height = 60, font_size = 32 },
//!         { type = "button", text = "Quit", on_click = "quit",
//!           width = 200, height = 60, font_size = 32 },
//!     }},
//! })
//! ```
//!
//! Button `on_click` values are emitted as string events through the Lua
//! event system. Listen for them with `events.on("start_game", callback)`.

use std::cell::RefCell;

use mlua::prelude::*;

use unison2d::core::{Color, EventSink, Vec2};
use unison2d::render::TextureId;
use unison2d::ui::facade::Ui;
use unison2d::ui::node::{UiNode, UiTree};
use unison2d::ui::style::{Anchor, PanelStyle, TextStyle};
use unison2d::{Engine, World};

use super::super::NoAction;

// ===================================================================
// Thread-local UI state
// ===================================================================

/// Pending UI tree for the current frame (set by ui:frame(), consumed by ScriptedGame::render()).
/// Stored as serialized node descriptors that ScriptedGame converts to UiTree.
pub struct UiFrameRequest {
    /// The font asset path used to create this UI.
    pub font_path: String,
    /// Serialized tree nodes.
    pub nodes: Vec<UiNodeDesc>,
}

/// A UI node descriptor parsed from Lua tables.
#[derive(Clone, Debug)]
pub struct UiNodeDesc {
    pub kind: String,
    pub text: Option<String>,
    pub on_click: Option<String>,
    pub anchor: Option<String>,
    pub gap: f32,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub font_size: Option<f32>,
    pub font_color: Option<u32>,
    pub bg_color: Option<u32>,
    pub padding: f32,
    pub visible: bool,
    pub texture: Option<u32>,
    pub value: Option<f32>,
    pub children: Vec<UiNodeDesc>,
}

/// Persistent Lua UI state — owns the `Ui<String>` instance and its event sink.
/// Lazily created on the first `ui:frame()` call that has a valid renderer.
struct LuaUiState {
    ui: Ui<String>,
    sink: EventSink,
    font_path: String,
}

thread_local! {
    static UI_FRAME: RefCell<Option<UiFrameRequest>> = const { RefCell::new(None) };
    static LUA_UI: RefCell<Option<LuaUiState>> = const { RefCell::new(None) };
}

pub fn take_ui_frame() -> Option<UiFrameRequest> {
    UI_FRAME.with(|cell| cell.borrow_mut().take())
}

// ===================================================================
// LuaUi userdata
// ===================================================================

struct LuaUi {
    font_path: String,
}

impl LuaUserData for LuaUi {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // ui:frame(tree_table)
        methods.add_method("frame", |_, this, tree: LuaTable| {
            let nodes = parse_node_list(&tree)?;
            UI_FRAME.with(|cell| {
                *cell.borrow_mut() = Some(UiFrameRequest {
                    font_path: this.font_path.clone(),
                    nodes,
                });
            });
            Ok(())
        });
    }
}

// ===================================================================
// Registration
// ===================================================================

/// Register `engine.create_ui()` on the existing engine table.
pub fn register(lua: &Lua) -> LuaResult<()> {
    let engine: LuaTable = lua.globals().get("engine")?;

    engine.set("create_ui", lua.create_function(|_, font_path: String| {
        Ok(LuaUi { font_path })
    })?)?;

    Ok(())
}

// ===================================================================
// Lua table → UiNodeDesc parsing
// ===================================================================

fn parse_node_list(table: &LuaTable) -> LuaResult<Vec<UiNodeDesc>> {
    let mut nodes = Vec::new();
    for entry in table.sequence_values::<LuaTable>() {
        nodes.push(parse_node(&entry?)?);
    }
    Ok(nodes)
}

fn parse_node(t: &LuaTable) -> LuaResult<UiNodeDesc> {
    let kind: String = t.get("type").unwrap_or_else(|_| "column".to_string());
    let text: Option<String> = t.get("text").ok();
    let on_click: Option<String> = t.get("on_click").ok();
    let anchor: Option<String> = t.get("anchor").ok();
    let gap: f32 = t.get("gap").unwrap_or(0.0);
    let width: Option<f32> = t.get("width").ok();
    let height: Option<f32> = t.get("height").ok();
    let font_size: Option<f32> = t.get("font_size").ok();
    let font_color: Option<u32> = t.get("font_color").ok();
    let bg_color: Option<u32> = t.get("bg_color").ok();
    let padding: f32 = t.get("padding").unwrap_or(0.0);
    // mlua converts nil → Ok(false) for bool (Lua truthiness), so we must
    // check for an explicit boolean value rather than using unwrap_or.
    let visible: bool = match t.get::<LuaValue>("visible") {
        Ok(LuaValue::Boolean(b)) => b,
        _ => true,
    };
    let texture: Option<u32> = t.get("texture").ok();
    let value: Option<f32> = t.get("value").ok();

    let children = match t.get::<LuaTable>("children") {
        Ok(c) => parse_node_list(&c)?,
        Err(_) => Vec::new(),
    };

    Ok(UiNodeDesc {
        kind,
        text,
        on_click,
        anchor,
        gap,
        width,
        height,
        font_size,
        font_color,
        bg_color,
        padding,
        visible,
        texture,
        value,
        children,
    })
}

// ===================================================================
// UiNodeDesc → UiNode<String> conversion
// ===================================================================

fn convert_tree(nodes: &[UiNodeDesc]) -> UiTree<String> {
    UiTree::new(nodes.iter().map(convert_node).collect())
}

fn convert_node(desc: &UiNodeDesc) -> UiNode<String> {
    let text = desc.text.clone().unwrap_or_default();
    let mut node: UiNode<String> = match desc.kind.as_str() {
        "column" => UiNode::column(),
        "row" => UiNode::row(),
        "panel" => UiNode::panel(),
        "label" => UiNode::label(text),
        "button" => {
            let btn = UiNode::button(text);
            match &desc.on_click {
                Some(evt) => btn.with_on_click(evt.clone()),
                None => btn,
            }
        }
        "icon" => UiNode::icon(TextureId::from_raw(desc.texture.unwrap_or(0))),
        "progress_bar" => UiNode::progress_bar(desc.value.unwrap_or(0.0)),
        "spacer" => UiNode::spacer(desc.value.unwrap_or(0.0)),
        _ => UiNode::column(),
    };

    if let Some(a) = &desc.anchor {
        node = node.with_anchor(parse_anchor(a));
    }
    if let Some(w) = desc.width {
        node = node.with_width(w);
    }
    if let Some(h) = desc.height {
        node = node.with_height(h);
    }
    if desc.gap != 0.0 {
        node = node.with_gap(desc.gap);
    }
    if desc.padding != 0.0 {
        node = node.with_padding(desc.padding);
    }
    if desc.font_size.is_some() || desc.font_color.is_some() {
        let mut style = TextStyle::default();
        if let Some(fs) = desc.font_size {
            style = style.font_size(fs);
        }
        if let Some(fc) = desc.font_color {
            style = style.color(Color::from_hex(fc));
        }
        node = node.with_text_style(style);
    }
    if let Some(bg) = desc.bg_color {
        let pstyle = PanelStyle::default().background(Color::from_hex(bg));
        node = node.with_panel_style(pstyle);
    }
    if !desc.visible {
        node = node.with_visible(false);
    }

    let children: Vec<UiNode<String>> = desc.children.iter().map(convert_node).collect();
    if !children.is_empty() {
        node = node.with_children(children);
    }
    node
}

fn parse_anchor(s: &str) -> Anchor {
    match s {
        "top_left" | "top-left" | "topleft" | "TopLeft" => Anchor::TopLeft,
        "top" | "top_center" | "top-center" | "TopCenter" => Anchor::TopCenter,
        "top_right" | "top-right" | "topright" | "TopRight" => Anchor::TopRight,
        "left" | "center_left" | "center-left" | "CenterLeft" => Anchor::CenterLeft,
        "center" | "Center" => Anchor::Center,
        "right" | "center_right" | "center-right" | "CenterRight" => Anchor::CenterRight,
        "bottom_left" | "bottom-left" | "bottomleft" | "BottomLeft" => Anchor::BottomLeft,
        "bottom" | "bottom_center" | "bottom-center" | "BottomCenter" => Anchor::BottomCenter,
        "bottom_right" | "bottom-right" | "bottomright" | "BottomRight" => Anchor::BottomRight,
        _ => Anchor::TopLeft,
    }
}

// ===================================================================
// Public entry — render any pending UI frame into the world's overlays.
// ===================================================================

/// Consume any pending `ui:frame()` request, lazily building the `Ui<String>`
/// the first time a frame is requested. Click events are drained from the
/// internal sink and returned as string event names for the Lua event system
/// to emit.
pub fn render_pending_ui(
    engine: &mut Engine<NoAction>,
    world: &mut World,
) -> Vec<String> {
    let frame_request = match take_ui_frame() {
        Some(r) => r,
        None => return Vec::new(),
    };

    // Decide whether we need to (re)build the Ui<String>.
    let needs_new = LUA_UI.with(|cell| match cell.borrow().as_ref() {
        Some(state) => state.font_path != frame_request.font_path,
        None => true,
    });

    if needs_new {
        let font_bytes = match engine.assets().get(&frame_request.font_path) {
            Some(b) => b.to_vec(),
            None => {
                eprintln!(
                    "[unison-scripting] UI font asset not found: '{}'",
                    frame_request.font_path
                );
                return Vec::new();
            }
        };

        let sink = EventSink::new();
        let sink_for_ui = sink.clone();
        let renderer = match engine.renderer.as_mut() {
            Some(r) => r.as_mut(),
            None => return Vec::new(),
        };

        let ui = match Ui::<String>::new(font_bytes, renderer, sink_for_ui) {
            Ok(u) => u,
            Err(e) => {
                eprintln!("[unison-scripting] Failed to create UI: {e}");
                return Vec::new();
            }
        };

        LUA_UI.with(|cell| {
            *cell.borrow_mut() = Some(LuaUiState {
                ui,
                sink,
                font_path: frame_request.font_path.clone(),
            });
        });
    }

    // Convert the parsed descriptors into a real UiTree<String>.
    let tree = convert_tree(&frame_request.nodes);

    // Screen size and dt.
    let (sw, sh) = super::engine::get_screen_size();
    let screen_size = Vec2::new(sw, sh);
    let dt = engine.dt();

    // Drive the Ui facade. We split-borrow the engine's input and renderer
    // fields directly so we can pass both to `ui.frame()` at once.
    LUA_UI.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let state = match borrow.as_mut() {
            Some(s) => s,
            None => return,
        };
        let renderer = match engine.renderer.as_mut() {
            Some(r) => r.as_mut(),
            None => return,
        };
        let input = &engine.input;
        let _ = state.ui.frame(tree, input, screen_size, dt, world, renderer);
    });

    // Drain emitted click events — return them so the caller can push them
    // into the Lua event system.
    LUA_UI.with(|cell| {
        let borrow = cell.borrow();
        let state = match borrow.as_ref() {
            Some(s) => s,
            None => return Vec::new(),
        };
        state
            .sink
            .drain()
            .into_iter()
            .filter_map(|e| e.downcast::<String>().ok().map(|b| *b))
            .collect()
    })
}
