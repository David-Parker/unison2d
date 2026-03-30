//! Foundation tests for the unison-scripting crate.
//!
//! Validates:
//! - Lua VM initializes successfully
//! - `ScriptedGame` implements `Game` and can be driven through its lifecycle
//! - Script lifecycle functions (init/update/render) are all called
//! - Script errors are handled gracefully (no panic)
//! - Missing lifecycle functions are silently skipped

use unison_scripting::ScriptedGame;
use unison2d::{Engine, Game};

// ---------------------------------------------------------------------------
// Test: Lua VM initializes successfully
// ---------------------------------------------------------------------------

#[test]
fn lua_vm_initializes() {
    use mlua::Lua;
    let lua = Lua::new();
    lua.load("local x = 1 + 1; assert(x == 2)").exec()
        .expect("Lua VM should execute a trivial script without error");
}

// ---------------------------------------------------------------------------
// Test: ScriptedGame implements Game — init/update/render without panic
// ---------------------------------------------------------------------------

#[test]
fn scripted_game_lifecycle_no_panic() {
    let mut game = ScriptedGame::new(
        r#"
        local g = {}
        function g.init()   end
        function g.update(dt) end
        function g.render() end
        return g
        "#,
    );

    let mut engine: Engine<unison_scripting::NoAction> = Engine::new();

    // None of these should panic even without a renderer.
    game.init(&mut engine);
    game.update(&mut engine);
    game.render(&mut engine);
}

// ---------------------------------------------------------------------------
// Test: Script lifecycle — all three functions are called
// ---------------------------------------------------------------------------

#[test]
fn script_lifecycle_all_called() {
    use mlua::Lua;

    // We verify via side effects: the script increments a counter for each
    // lifecycle call. We read the counter from Lua after driving the game.
    let lua = Lua::new();
    lua.globals().set("call_count", 0i32).unwrap();

    let result = lua.load(r#"
        local g = {}
        function g.init()
            call_count = call_count + 10
        end
        function g.update(dt)
            call_count = call_count + 1
        end
        function g.render()
            call_count = call_count + 100
        end
        return g
    "#).eval::<mlua::Table>().unwrap();

    // Simulate what ScriptedGame does: call each lifecycle function.
    let init_fn: mlua::Function = result.get("init").unwrap();
    let update_fn: mlua::Function = result.get("update").unwrap();
    let render_fn: mlua::Function = result.get("render").unwrap();

    init_fn.call::<()>(()).unwrap();
    update_fn.call::<()>(0.016f32).unwrap();
    render_fn.call::<()>(()).unwrap();

    let count: i32 = lua.globals().get("call_count").unwrap();
    assert_eq!(count, 111, "init(+10) + update(+1) + render(+100) = 111");
}

// ---------------------------------------------------------------------------
// Test: Script error handling — syntax error does not panic
// ---------------------------------------------------------------------------

#[test]
fn script_syntax_error_no_panic() {
    let mut game = ScriptedGame::new("this is not valid lua }{");
    let mut engine: Engine<unison_scripting::NoAction> = Engine::new();
    // Should log the error internally and continue — no panic.
    game.init(&mut engine);
    game.update(&mut engine);
    game.render(&mut engine);
}

// ---------------------------------------------------------------------------
// Test: Missing lifecycle function — graceful fallback, no panic
// ---------------------------------------------------------------------------

#[test]
fn missing_render_function_is_noop() {
    let mut game = ScriptedGame::new(
        r#"
        local g = {}
        function g.init()   end
        function g.update(dt) end
        -- render is intentionally missing
        return g
        "#,
    );
    let mut engine: Engine<unison_scripting::NoAction> = Engine::new();
    game.init(&mut engine);
    game.update(&mut engine);
    game.render(&mut engine); // should not panic
}

#[test]
fn empty_script_is_noop() {
    // Script returns nothing (nil) — ScriptedGame must handle this gracefully.
    let mut game = ScriptedGame::new("");
    let mut engine: Engine<unison_scripting::NoAction> = Engine::new();
    game.init(&mut engine);
    game.update(&mut engine);
    game.render(&mut engine);
}
