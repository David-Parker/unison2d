use std::process::Command;
use tempfile::tempdir;

fn scaffold_web_only(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let out = Command::new(env!("CARGO_BIN_EXE_unison"))
        .args(["new", name, "--no-ios", "--no-android", "--no-git"])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    dir.join(name)
}

#[test]
fn platform_add_ios_creates_dir_and_updates_config() {
    let dir = tempdir().unwrap();
    let proj = scaffold_web_only(dir.path(), "g");
    unison_cli::commands::platform::add(&proj, "ios").unwrap();
    assert!(proj.join("platform/ios/Info.plist").exists());
    let toml = std::fs::read_to_string(proj.join("unison.toml")).unwrap();
    assert!(toml.contains("ios = true"));
}

#[test]
fn platform_remove_deletes_dir_and_updates_config() {
    let dir = tempdir().unwrap();
    let proj = scaffold_web_only(dir.path(), "g");
    unison_cli::commands::platform::add(&proj, "ios").unwrap();
    unison_cli::commands::platform::remove(&proj, "ios").unwrap();
    assert!(!proj.join("platform/ios").exists());
    let toml = std::fs::read_to_string(proj.join("unison.toml")).unwrap();
    assert!(toml.contains("ios = false"));
}

#[test]
fn platform_remove_last_fails() {
    let dir = tempdir().unwrap();
    let proj = scaffold_web_only(dir.path(), "g");
    let err = unison_cli::commands::platform::remove(&proj, "web").unwrap_err();
    assert!(err.to_string().contains("only enabled"));
}
