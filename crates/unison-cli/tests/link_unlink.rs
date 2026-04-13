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
    fs::create_dir_all(engine.join("vendor/lua-src")).unwrap();
    fs::write(engine.join("crates/unison-scripting/Cargo.toml"), "").unwrap();
    fs::write(engine.join("crates/unison-assets/Cargo.toml"), "").unwrap();
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

    // No leftover git-URL patch block — but [patch.crates-io] lua-src IS
    // expected (engine ships a forked lua-src with wasm32 support).
    assert!(!cargo.contains("[patch.\"https://github.com/x/unison2d\"]"),
            "unexpected git patch after link:\n{}", cargo);
    assert!(cargo.contains("[patch.crates-io]"),
            "expected [patch.crates-io] block:\n{}", cargo);
    let expected_vendor = engine_canon.join("vendor/lua-src");
    assert!(cargo.contains(&format!("lua-src = {{ path = \"{}\"", expected_vendor.display())),
            "expected lua-src patch pointing at engine vendor dir:\n{}", cargo);

    let unison_toml = fs::read_to_string(proj.join("unison.toml")).unwrap();
    assert!(unison_toml.contains("link_path"));
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

    // No lingering path entries or patch block.
    assert!(!cargo.contains("path ="), "unexpected path spec after unlink:\n{}", cargo);
    assert!(!cargo.contains("[patch"), "unexpected [patch] block after unlink:\n{}", cargo);

    let unison_toml = fs::read_to_string(proj.join("unison.toml")).unwrap();
    assert!(!unison_toml.contains("link_path"));
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

#[test]
fn link_strips_legacy_patch_block() {
    // Projects created before the rewrite may have a [patch.<url>] block from
    // the old `unison link`. Running the new `link` should clean it up.
    let (_d, proj, engine) = setup_project_and_engine();
    // Inject a legacy patch block as if left behind by an older link run.
    let mut cargo = fs::read_to_string(proj.join("Cargo.toml")).unwrap();
    cargo.push_str(&format!(
        "\n[patch.\"https://github.com/x/unison2d\"]\n\
         unison-scripting = {{ path = \"/old/path/unison-scripting\" }}\n"
    ));
    fs::write(proj.join("Cargo.toml"), cargo).unwrap();

    unison_cli::commands::link::link(&proj, engine.to_str().unwrap()).unwrap();
    let cargo = fs::read_to_string(proj.join("Cargo.toml")).unwrap();
    assert!(!cargo.contains("[patch.\"https://github.com/x/unison2d\"]"),
            "legacy git-URL patch block not cleaned up:\n{}", cargo);
    assert!(!cargo.contains("/old/path"),
            "old patch entry still present:\n{}", cargo);
}
