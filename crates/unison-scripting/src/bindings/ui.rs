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

thread_local! {
    static UI_FRAME: RefCell<Option<UiFrameRequest>> = const { RefCell::new(None) };
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
    let visible: bool = t.get("visible").unwrap_or(true);
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
