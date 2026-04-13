use std::fs;
use tempfile::tempdir;

fn setup_project_and_engine() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let proj = dir.path().join("proj");
    fs::create_dir_all(&proj).unwrap();
    fs::write(proj.join("Cargo.toml"), r#"[package]
name = "proj"
version = "0.1.0"

[dependencies]
unison-scripting = { git = "https://github.com/x/unison2d", tag = "v0", features = ["simd"] }

[build-dependencies]
unison-assets = { git = "https://github.com/x/unison2d", tag = "v0", features = ["build"] }

[patch.crates-io]
lua-src = { git = "https://github.com/x/unison2d", tag = "v0" }
"#).unwrap();
    fs::write(proj.join("unison.toml"), r#"[project]
name = "proj"
lang = "lua"

[engine]
git = "https://github.com/x/unison2d"
tag = "v0"

[platforms]
web = true
ios = false
android = false
"#).unwrap();
    let engine = dir.path().join("engine");
    fs::create_dir_all(engine.join("crates/unison-scripting")).unwrap();
    fs::create_dir_all(engine.join("crates/unison-assets")).unwrap();
    fs::create_dir_all(engine.join("crates/unison-lua")).unwrap();
    fs::write(engine.join("crates/unison-scripting/Cargo.toml"), "").unwrap();
    fs::write(engine.join("crates/unison-assets/Cargo.toml"), "").unwrap();
    fs::write(engine.join("crates/unison-lua/Cargo.toml"), "").unwrap();
    (dir, proj, engine)
}

#[test]
fn link_swaps_git_deps_to_path_deps() {
    let (_d, proj, engine) = setup_project_and_engine();
    let engine_str = engine.to_str().unwrap();
    unison_cli::commands::link::link(&proj, engine_str).unwrap();
    let cargo = fs::read_to_string(proj.join("Cargo.toml")).unwrap();

    // Git specs should be gone, replaced with path specs.
    assert!(!cargo.contains("git = \"https://github.com/x/unison2d\""),
            "expected git spec to be removed after link:\n{}", cargo);
    assert!(!cargo.contains("tag = \"v0\""),
            "expected tag to be removed after link:\n{}", cargo);

    // Path entries present. Use canonicalize to match link's canonicalize call
    // (on macOS /var → /private/var).
    let engine_canon = fs::canonicalize(&engine).unwrap();
    let expected_scripting = engine_canon.join("crates/unison-scripting");
    let expected_assets = engine_canon.join("crates/unison-assets");
    assert!(cargo.contains(&format!("path = \"{}\"", expected_scripting.display())),
            "expected unison-scripting path:\n{}", cargo);
    assert!(cargo.contains(&format!("path = \"{}\"", expected_assets.display())),
            "expected unison-assets path:\n{}", cargo);

    // Features preserved.
    assert!(cargo.contains("features = [\"simd\"]"));
    assert!(cargo.contains("features = [\"build\"]"));

    let unison_toml = fs::read_to_string(proj.join("unison.toml")).unwrap();
    assert!(unison_toml.contains("link_path"));
}

#[test]
fn link_writes_lua_src_patch() {
    let (_d, proj, engine) = setup_project_and_engine();
    unison_cli::commands::link::link(&proj, engine.to_str().unwrap()).unwrap();
    let cargo = fs::read_to_string(proj.join("Cargo.toml")).unwrap();

    let engine_canon = fs::canonicalize(&engine).unwrap();
    let expected_lua = engine_canon.join("crates/unison-lua");

    assert!(cargo.contains("[patch.crates-io]"),
            "expected [patch.crates-io] block:\n{}", cargo);
    assert!(cargo.contains(&format!("lua-src = {{ path = \"{}\" }}", expected_lua.display())),
            "expected lua-src path patch:\n{}", cargo);
    // No `package = ...` key should be present on the lua-src patch.
    assert!(!cargo.contains("package = \"lua-src\""),
            "unexpected package key on lua-src patch:\n{}", cargo);
    assert!(!cargo.contains("package = \"unison-lua\""),
            "unexpected package key on lua-src patch:\n{}", cargo);
}

#[test]
fn unlink_restores_git_deps() {
    let (_d, proj, engine) = setup_project_and_engine();
    unison_cli::commands::link::link(&proj, engine.to_str().unwrap()).unwrap();
    unison_cli::commands::link::unlink(&proj).unwrap();
    let cargo = fs::read_to_string(proj.join("Cargo.toml")).unwrap();

    // Git specs restored.
    assert!(cargo.contains("git = \"https://github.com/x/unison2d\""),
            "expected git spec restored:\n{}", cargo);
    assert!(cargo.contains("tag = \"v0\""),
            "expected tag restored:\n{}", cargo);

    // Features preserved through link → unlink round trip.
    assert!(cargo.contains("features = [\"simd\"]"));
    assert!(cargo.contains("features = [\"build\"]"));

    // No lingering path entries for the engine deps/patch.
    assert!(!cargo.contains("path ="), "unexpected path spec after unlink:\n{}", cargo);

    let unison_toml = fs::read_to_string(proj.join("unison.toml")).unwrap();
    assert!(!unison_toml.contains("link_path"));
}

#[test]
fn unlink_writes_lua_src_patch_with_git() {
    let (_d, proj, engine) = setup_project_and_engine();
    unison_cli::commands::link::link(&proj, engine.to_str().unwrap()).unwrap();
    unison_cli::commands::link::unlink(&proj).unwrap();
    let cargo = fs::read_to_string(proj.join("Cargo.toml")).unwrap();

    // lua-src patch is in git form — same URL + tag as the engine deps.
    assert!(cargo.contains("[patch.crates-io]"),
            "expected [patch.crates-io] block after unlink:\n{}", cargo);
    // Look for lua-src as a git-form inline table.
    let lua_line = cargo
        .lines()
        .find(|l| l.trim_start().starts_with("lua-src"))
        .unwrap_or_else(|| panic!("no lua-src line after unlink:\n{}", cargo));
    assert!(lua_line.contains("git = \"https://github.com/x/unison2d\""),
            "expected lua-src git spec:\n{}", lua_line);
    assert!(lua_line.contains("tag = \"v0\""),
            "expected lua-src tag spec:\n{}", lua_line);
    assert!(!lua_line.contains("path ="),
            "unexpected path spec on lua-src after unlink:\n{}", lua_line);
    assert!(!lua_line.contains("package ="),
            "unexpected package key on lua-src:\n{}", lua_line);
}

#[test]
fn link_unlink_round_trip_preserves_engine_deps() {
    let (_d, proj, engine) = setup_project_and_engine();
    for _ in 0..2 {
        unison_cli::commands::link::link(&proj, engine.to_str().unwrap()).unwrap();
        unison_cli::commands::link::unlink(&proj).unwrap();
    }
    let cargo = fs::read_to_string(proj.join("Cargo.toml")).unwrap();

    // Engine deps present with features intact.
    assert!(cargo.contains("unison-scripting"));
    assert!(cargo.contains("features = [\"simd\"]"),
            "unison-scripting features lost after round trip:\n{}", cargo);
    assert!(cargo.contains("unison-assets"));
    assert!(cargo.contains("features = [\"build\"]"),
            "unison-assets features lost after round trip:\n{}", cargo);

    // lua-src patch survives in git form.
    assert!(cargo.contains("[patch.crates-io]"),
            "expected [patch.crates-io] after round trip:\n{}", cargo);
    let lua_line = cargo
        .lines()
        .find(|l| l.trim_start().starts_with("lua-src"))
        .unwrap_or_else(|| panic!("no lua-src line after round trip:\n{}", cargo));
    assert!(lua_line.contains("git ="),
            "expected lua-src git spec after round trip:\n{}", lua_line);
    assert!(lua_line.contains("tag = \"v0\""),
            "expected lua-src tag after round trip:\n{}", lua_line);
}

#[test]
fn link_rejects_non_engine_path() {
    let (dir, proj, _engine) = setup_project_and_engine();
    let fake = dir.path().join("not-engine");
    fs::create_dir(&fake).unwrap();
    let err = unison_cli::commands::link::link(&proj, fake.to_str().unwrap()).unwrap_err();
    assert!(err.to_string().contains("not look like"),
            "unexpected error message: {}", err);
}
