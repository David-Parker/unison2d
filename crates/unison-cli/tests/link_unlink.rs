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
unison-scripting = { git = "https://github.com/x/unison2d", tag = "v0" }
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
    fs::write(engine.join("crates/unison-scripting/Cargo.toml"), "").unwrap();
    fs::write(engine.join("crates/unison-assets/Cargo.toml"), "").unwrap();
    (dir, proj, engine)
}

#[test]
fn link_writes_patch_block() {
    let (_d, proj, engine) = setup_project_and_engine();
    let engine_str = engine.to_str().unwrap();
    unison_cli::commands::link::link(&proj, engine_str).unwrap();
    let cargo = fs::read_to_string(proj.join("Cargo.toml")).unwrap();
    assert!(cargo.contains("[patch.\"https://github.com/x/unison2d\"]"),
            "expected patch header in:\n{}", cargo);
    assert!(cargo.contains("unison-scripting"));
    assert!(cargo.contains("unison-assets"));
    let unison_toml = fs::read_to_string(proj.join("unison.toml")).unwrap();
    assert!(unison_toml.contains("link_path"));
}

#[test]
fn unlink_removes_patch_block() {
    let (_d, proj, engine) = setup_project_and_engine();
    unison_cli::commands::link::link(&proj, engine.to_str().unwrap()).unwrap();
    unison_cli::commands::link::unlink(&proj).unwrap();
    let cargo = fs::read_to_string(proj.join("Cargo.toml")).unwrap();
    assert!(!cargo.contains("[patch."), "unexpected patch left in:\n{}", cargo);
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
