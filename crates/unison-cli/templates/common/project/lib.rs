//! {{PROJECT_NAME}} — Unison 2D game entry point.
//!
//! All gameplay lives in Lua under project/assets/scripts/. This file only
//! wires the platform runners to the Lua entry point via `scripted_game_entry!`.

#[allow(dead_code)]
mod assets {
    include!(concat!(env!("OUT_DIR"), "/assets.rs"));
}

unison_scripting::scripted_game_entry!("scripts/main.lua", assets::ASSETS);
