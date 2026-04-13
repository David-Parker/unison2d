//! Validate that the scripts shipped by `unison new` actually load and run
//! through the real `ScriptedGame` pipeline.
//!
//! These tests sit in `unison-tests` (rather than `unison-cli`) so they can
//! reuse the Lua VM + engine binding machinery without pulling `mlua` into
//! `unison-cli`'s dev-deps. They're the stronger counterpart to the textual
//! checks in `unison-cli/tests/new_smoke.rs`: the smoke tests verify the
//! template file's shape, these verify it executes cleanly against the real
//! engine.

use unison2d::{Engine, Game};
use unison_scripting::{NoAction, ScriptedGame};

const LUA_TEMPLATE: &str = include_str!(
    "../../unison-cli/templates/scripting-lua/project/assets/scripts/main.lua"
);

/// Placeholder substitution matches what `unison-cli`'s `render` step performs
/// at scaffold time. Keep this in sync with
/// `unison-cli/src/render.rs::render`.
fn render(template: &str, project_name: &str) -> String {
    template.replace("{{PROJECT_NAME}}", project_name)
}

#[test]
fn cli_lua_template_returns_table_with_lifecycle_methods() {
    // Minimal check: the template's top-level chunk must return a table, and
    // the table must carry the lifecycle functions the engine dispatches to.
    // This is the smallest possible reproduction of the "red bar on fresh
    // project" bug — it doesn't need the engine at all, just a plain Lua VM.
    use mlua::{Function, Lua, Table};
    let source = render(LUA_TEMPLATE, "test_game");
    let lua = Lua::new();
    let game: Table = lua
        .load(&source)
        .eval()
        .expect("template main.lua must return a table from top-level chunk");
    let _: Function = game
        .get("init")
        .expect("template must define `init` on the returned table");
    let _: Function = game
        .get("update")
        .expect("template must define `update` on the returned table");
    let _: Function = game
        .get("render")
        .expect("template must define `render` on the returned table");
}

#[test]
fn cli_lua_template_runs_full_lifecycle_without_error_overlay() {
    // End-to-end: drive the template through the actual ScriptedGame pipeline.
    // If the template ever drifts out of sync with engine expectations (wrong
    // function names, missing return, bad binding call, etc.), the engine
    // captures the error into its overlay — which is exactly what would light
    // up the red bar in the browser.
    let source = render(LUA_TEMPLATE, "test_game");
    let mut game = ScriptedGame::new(source);
    let mut engine: Engine<NoAction> = Engine::new();

    game.init(&mut engine);
    assert!(
        !game.has_error(),
        "template main.lua errored during init: {}",
        game.error_message().unwrap_or("(none)")
    );

    game.update(&mut engine);
    assert!(
        !game.has_error(),
        "template main.lua errored during update: {}",
        game.error_message().unwrap_or("(none)")
    );

    game.render(&mut engine);
    assert!(
        !game.has_error(),
        "template main.lua errored during render: {}",
        game.error_message().unwrap_or("(none)")
    );
}
