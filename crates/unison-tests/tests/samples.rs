//! Smoke tests for sample projects.
//!
//! Each test loads a sample's `scripts/` directory, injects the Lua files into
//! the engine's asset store (so `require()` works), then runs init + 5 update
//! ticks and asserts no errors.

use std::fs;
use std::path::Path;

use unison_scripting::ScriptedGame;
use unison2d::{Engine, Game};

/// Recursively collect all files under `dir` with their paths relative to `dir`.
fn collect_files(dir: &Path, rel: &Path, out: &mut Vec<(String, Vec<u8>)>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let rel_path = rel.join(entry.file_name());
        if path.is_dir() {
            collect_files(&path, &rel_path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("lua") {
            let key = format!("scripts/{}", rel_path.display());
            let data = fs::read(&path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            out.push((key, data));
        }
    }
}

/// Load all Lua files from a sample's `scripts/` directory into the engine,
/// run init + 5 update ticks, and assert no errors.
fn run_sample(scripts_dir: &Path) {
    let main_lua = scripts_dir.join("main.lua");
    assert!(
        main_lua.exists(),
        "main.lua not found at {}",
        main_lua.display()
    );

    let source = fs::read_to_string(&main_lua)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", main_lua.display()));

    let mut game = ScriptedGame::new(&source);
    let mut engine: Engine = Engine::new();

    // Inject all .lua files from the sample into the asset store so that
    // `require()` works (setup_require reads from engine.assets()).
    let mut files = Vec::new();
    collect_files(scripts_dir, Path::new(""), &mut files);
    for (key, data) in files {
        engine.assets_mut().insert(key, data);
    }

    game.init(&mut engine);
    assert!(
        !game.has_error(),
        "Lua error during init: {}",
        game.error_message().unwrap_or("(unknown)")
    );

    for _ in 0..5 {
        game.update(&mut engine);
        assert!(
            !game.has_error(),
            "Lua error during update: {}",
            game.error_message().unwrap_or("(unknown)")
        );
    }
}

// ---------------------------------------------------------------------------
// Lua minimal sample
// ---------------------------------------------------------------------------

#[test]
fn lua_minimal_sample_runs() {
    let scripts_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../samples/lua-minimal/scripts");
    run_sample(&scripts_dir);
}

// ---------------------------------------------------------------------------
// TypeScript minimal sample (transpiled output — skip if not present)
// ---------------------------------------------------------------------------

#[test]
fn ts_minimal_sample_runs() {
    let scripts_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../samples/ts-minimal/scripts");

    if !scripts_dir.join("main.lua").exists() {
        eprintln!(
            "SKIPPING ts_minimal_sample_runs: transpiled .lua not found at {}",
            scripts_dir.display()
        );
        return;
    }

    run_sample(&scripts_dir);
}

// ---------------------------------------------------------------------------
// donut-game (TSTL-transpiled — skip if not present)
// ---------------------------------------------------------------------------

#[test]
fn donut_game_scripts_run() {
    let scripts_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../project/assets/scripts");

    let main_lua = scripts_dir.join("main.lua");
    if !main_lua.exists() {
        eprintln!(
            "SKIPPING donut_game_scripts_run: transpiled .lua not found at {}",
            scripts_dir.display()
        );
        return;
    }

    run_sample(&scripts_dir);
}
