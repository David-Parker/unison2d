//! Tests for the Lua-facing action map (unison.input.bind_action / bind_axis / queries).

use mlua::prelude::*;
use unison2d::input::{InputState, KeyCode};
use unison_scripting::bindings;

/// Create a Lua VM with all engine bindings registered.
fn make_lua() -> Lua {
    let lua = Lua::new();
    bindings::register_all(&lua).expect("register_all failed");
    lua
}

/// Simulate a fresh `InputState` with Space pressed and refresh the snapshot.
fn refresh_with_space_pressed() -> InputState {
    let mut input = InputState::new();
    input.key_pressed(KeyCode::Space);
    bindings::input::refresh(&input);
    input
}

// ===================================================================
// Test 1: bind_action + pressed key → is_action_pressed returns true
// ===================================================================

#[test]
fn action_pressed_matches_held_key() {
    let lua = make_lua();

    refresh_with_space_pressed();

    lua.load(r#"
        unison.input.bind_action("jump", { keys = {"Space"} })
        assert(unison.input.is_action_pressed("jump") == true,
            "expected is_action_pressed to be true while Space is held")
        assert(unison.input.is_action_pressed("unknown") == false,
            "expected is_action_pressed to be false for unbound action")
    "#).exec().expect("Lua error in test 1");

    // Reset bindings so tests don't bleed into each other.
    bindings::action_map::reset();
}

// ===================================================================
// Test 2: just_pressed on the first frame; false after begin_frame
// ===================================================================

#[test]
fn action_just_pressed_clears_after_begin_frame() {
    let lua = make_lua();

    // Frame 1: Space is freshly pressed.
    refresh_with_space_pressed();

    lua.load(r#"
        unison.input.bind_action("jump", { keys = {"Space"} })
        assert(unison.input.is_action_just_pressed("jump") == true,
            "expected just_pressed on first frame")
    "#).exec().expect("Lua error in test 2a");

    // Frame 2: Space is still held but begin_frame was called — just_pressed clears.
    let mut input = InputState::new();
    input.key_pressed(KeyCode::Space); // held from before
    input.begin_frame();               // clears just_pressed
    input.key_pressed(KeyCode::Space); // still held (begin_frame already inserted; this is a no-op for held)
    // Actually: after begin_frame keys_just_pressed is cleared, but key is still in keys_held.
    // We need to simulate the "held but not freshly pressed" state:
    // — build a new state, mark Space as held without re-firing key_pressed.
    let mut input2 = InputState::new();
    // Copy held state from previous frame: use the state we built.
    // The simplest way: press Space (held), call begin_frame (clears just_pressed),
    // then DON'T call key_pressed again → held=true, just_pressed=false.
    input2.key_pressed(KeyCode::Space);
    input2.begin_frame();
    bindings::input::refresh(&input2);

    lua.load(r#"
        assert(unison.input.is_action_pressed("jump") == true,
            "expected still pressed in frame 2")
        assert(unison.input.is_action_just_pressed("jump") == false,
            "expected just_pressed to be false after begin_frame")
    "#).exec().expect("Lua error in test 2b");

    bindings::action_map::reset();
}

// ===================================================================
// Test 3: bind_axis — digital axis from positive/negative actions
// ===================================================================

#[test]
fn axis_digital_from_positive_negative_actions() {
    let lua = make_lua();

    // Bind actions for up/down, then bind an axis using them.
    lua.load(r#"
        unison.input.bind_action("up", { keys = {"ArrowUp"} })
        unison.input.bind_action("down", { keys = {"ArrowDown"} })
        unison.input.bind_axis("vertical", { positive = "up", negative = "down" })
    "#).exec().expect("Lua error in test 3 setup");

    // Press "up" → axis should be 1.0
    {
        let mut input = InputState::new();
        input.key_pressed(KeyCode::ArrowUp);
        bindings::input::refresh(&input);

        lua.load(r#"
            assert(unison.input.axis("vertical") == 1.0,
                "expected axis = 1.0 when up is pressed")
        "#).exec().expect("Lua error in test 3a");
    }

    // Press "down" → axis should be -1.0
    {
        let mut input = InputState::new();
        input.key_pressed(KeyCode::ArrowDown);
        bindings::input::refresh(&input);

        lua.load(r#"
            assert(unison.input.axis("vertical") == -1.0,
                "expected axis = -1.0 when down is pressed")
        "#).exec().expect("Lua error in test 3b");
    }

    // Release both → axis should be 0.0
    {
        let input = InputState::new(); // nothing pressed
        bindings::input::refresh(&input);

        lua.load(r#"
            assert(unison.input.axis("vertical") == 0.0,
                "expected axis = 0.0 when nothing is pressed")
        "#).exec().expect("Lua error in test 3c");
    }

    bindings::action_map::reset();
}

// ===================================================================
// Test 4: mouse button action binding
// ===================================================================

#[test]
fn action_pressed_from_mouse_button() {
    use unison2d::input::MouseButton;

    let lua = make_lua();

    lua.load(r#"
        unison.input.bind_action("fire", { mouse_buttons = {0} })
    "#).exec().expect("Lua error in test 4 setup");

    // Left mouse button pressed.
    {
        let mut input = InputState::new();
        input.mouse_button_pressed(MouseButton::Left);
        bindings::input::refresh(&input);

        lua.load(r#"
            assert(unison.input.is_action_pressed("fire") == true,
                "expected fire action pressed when left mouse button held")
            assert(unison.input.is_action_just_pressed("fire") == true,
                "expected fire action just_pressed on first frame")
        "#).exec().expect("Lua error in test 4a");
    }

    // After begin_frame: still held, just_pressed clears.
    {
        let mut input = InputState::new();
        input.mouse_button_pressed(MouseButton::Left);
        input.begin_frame();
        bindings::input::refresh(&input);

        lua.load(r#"
            assert(unison.input.is_action_pressed("fire") == true,
                "expected fire still pressed")
            assert(unison.input.is_action_just_pressed("fire") == false,
                "expected fire just_pressed cleared after begin_frame")
        "#).exec().expect("Lua error in test 4b");
    }

    bindings::action_map::reset();
}

// ===================================================================
// Test 5: is_action_just_released
// ===================================================================

#[test]
fn action_just_released() {
    let lua = make_lua();

    lua.load(r#"
        unison.input.bind_action("jump", { keys = {"Space"} })
    "#).exec().expect("Lua error in test 5 setup");

    // Press Space.
    let mut input = InputState::new();
    input.key_pressed(KeyCode::Space);
    bindings::input::refresh(&input);

    // Release Space (begin_frame first to move to held state, then release).
    input.begin_frame();
    input.key_released(KeyCode::Space);
    bindings::input::refresh(&input);

    lua.load(r#"
        assert(unison.input.is_action_pressed("jump") == false,
            "expected not pressed after release")
        assert(unison.input.is_action_just_released("jump") == true,
            "expected just_released on frame of release")
    "#).exec().expect("Lua error in test 5");

    bindings::action_map::reset();
}
