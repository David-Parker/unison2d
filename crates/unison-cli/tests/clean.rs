use std::fs;
use tempfile::tempdir;

#[test]
fn clean_removes_expected_dirs() {
    let dir = tempdir().unwrap();
    let proj = dir.path().join("p");
    fs::create_dir_all(proj.join("target")).unwrap();
    fs::create_dir_all(proj.join("platform/web/dist")).unwrap();
    fs::create_dir_all(proj.join("platform/android/app/build")).unwrap();
    fs::write(proj.join("README.md"), "x").unwrap();
    fs::write(proj.join("unison.toml"), r#"
[project]
name = "p"
lang = "lua"
[engine]
git = "x"
tag = "v0"
[platforms]
web = true
"#).unwrap();
    unison_cli::commands::clean::run(&proj).unwrap();
    assert!(!proj.join("target").exists());
    assert!(!proj.join("platform/web/dist").exists());
    assert!(!proj.join("platform/android/app/build").exists());
    // README untouched.
    assert!(proj.join("README.md").exists());
}

#[test]
fn clean_ts_removes_scripts() {
    let dir = tempdir().unwrap();
    let proj = dir.path().join("p");
    fs::create_dir_all(proj.join("target")).unwrap();
    fs::create_dir_all(proj.join("project/assets/scripts")).unwrap();
    fs::write(proj.join("unison.toml"), r#"
[project]
name = "p"
lang = "ts"
[engine]
git = "x"
tag = "v0"
[platforms]
web = true
"#).unwrap();
    unison_cli::commands::clean::run(&proj).unwrap();
    assert!(!proj.join("target").exists());
    assert!(!proj.join("project/assets/scripts").exists());
}

#[test]
fn clean_idempotent_when_dirs_missing() {
    let dir = tempdir().unwrap();
    let proj = dir.path().join("p");
    fs::create_dir_all(&proj).unwrap();
    fs::write(proj.join("unison.toml"), r#"
[project]
name = "p"
lang = "lua"
[engine]
git = "x"
tag = "v0"
[platforms]
web = true
"#).unwrap();
    // None of the artifact dirs exist — should succeed without error.
    unison_cli::commands::clean::run(&proj).unwrap();
}
