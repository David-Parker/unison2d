use std::process::Command;
use tempfile::tempdir;

#[test]
fn new_ts_lang_creates_ts_layout() {
    let dir = tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_unison"))
        .args(["new", "ts-test", "--lang", "ts", "--no-ios", "--no-android", "--no-git"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let root = dir.path().join("ts-test");
    assert!(root.join("package.json").exists());
    assert!(root.join("project/scripts-src/tsconfig.json").exists());
    assert!(root.join("project/scripts-src/main.ts").exists());
    let gi = std::fs::read_to_string(root.join(".gitignore")).unwrap();
    assert!(gi.contains("/node_modules"));
    assert!(gi.contains("/project/assets/scripts/"));
    assert!(!root.join(".gitignore-ts-addon").exists());
}

#[test]
fn new_web_index_html_points_trunk_at_project_root_cargo_toml() {
    // Regression test: the scaffolded platform/web/index.html must tell trunk
    // where Cargo.toml lives. Without `href="../../Cargo.toml"` on the
    // `<link data-trunk rel="rust">` tag, trunk looks for Cargo.toml inside
    // platform/web/ and fails immediately with
    // `manifest path '.../platform/web/Cargo.toml' does not exist`.
    let dir = tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_unison"))
        .args(["new", "web-href-test", "--no-ios", "--no-android", "--no-git"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let index = std::fs::read_to_string(
        dir.path().join("web-href-test/platform/web/index.html"),
    ).unwrap();
    assert!(
        index.contains("href=\"../../Cargo.toml\""),
        "index.html is missing trunk href pointing at project-root Cargo.toml:\n{}",
        index
    );
    // unison-web::run() hardcodes document.get_element_by_id("canvas") and
    // panics at runtime if the id doesn't match. The template must use
    // id="canvas" (not "game-canvas" or anything else).
    assert!(
        index.contains("id=\"canvas\""),
        "index.html is missing <canvas id=\"canvas\"> (unison-web looks up by this exact id):\n{}",
        index
    );
}

#[test]
fn new_creates_expected_files() {
    let dir = tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_unison"))
        .arg("new")
        .arg("test-game")
        .arg("--no-ios")
        .arg("--no-android")
        .arg("--no-git")
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let root = dir.path().join("test-game");
    assert!(root.join("Cargo.toml").exists());
    assert!(root.join("build.rs").exists());
    assert!(root.join("project/lib.rs").exists());
    assert!(root.join("project/assets/scripts/main.lua").exists());
    assert!(root.join("platform/web/index.html").exists());
    assert!(root.join("platform/web/Trunk.toml").exists());
    assert!(root.join("unison.toml").exists());
    assert!(root.join(".gitignore").exists());

    // Assert substitution happened
    let cargo = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(cargo.contains("name = \"test_game\""));
    assert!(!cargo.contains("{{"));

    // The scaffolded main.lua must match the engine's expected shape:
    // a table with init/update/render methods, returned from the script.
    // Without `return game`, unison-scripting's `eval()` call gets nil
    // and the engine renders the error overlay (red bar) at runtime.
    let main_lua = std::fs::read_to_string(root.join("project/assets/scripts/main.lua")).unwrap();
    assert!(
        main_lua.contains("function game.init") && main_lua.contains("function game.render"),
        "main.lua missing lifecycle methods on `game` table:\n{}",
        main_lua
    );
    assert!(
        main_lua.trim_end().ends_with("return game"),
        "main.lua must end with `return game` (unison-scripting expects script to return a table):\n{}",
        main_lua
    );
}

#[test]
fn new_fails_if_dir_exists() {
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("pre-existing")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_unison"))
        .arg("new")
        .arg("pre-existing")
        .arg("--no-ios")
        .arg("--no-android")
        .arg("--no-git")
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
}

#[test]
fn new_creates_ios_files() {
    let dir = tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_unison"))
        .args(["new", "ios-test", "--no-android", "--no-git"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let root = dir.path().join("ios-test");
    assert!(root.join("platform/ios/AppDelegate.swift").exists());
    assert!(root.join("platform/ios/Info.plist").exists());
    assert!(root.join("platform/ios/ios-test-ios.xcodeproj/project.pbxproj").exists());

    let info = std::fs::read_to_string(root.join("platform/ios/Info.plist")).unwrap();
    assert!(info.contains("com.example.ios_test"));
    assert!(!info.contains("{{"));
}

#[test]
fn new_git_init_uses_main_branch() {
    let dir = tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_unison"))
        .args(["new", "main-branch-test", "--no-ios", "--no-android"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let root = dir.path().join("main-branch-test");
    let head = std::fs::read_to_string(root.join(".git/HEAD")).unwrap();
    assert!(
        head.trim() == "ref: refs/heads/main",
        "expected HEAD to point at main, got: {:?}",
        head
    );
}

#[test]
fn new_creates_android_files() {
    let dir = tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_unison"))
        .args(["new", "android-test", "--no-ios", "--no-git"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let root = dir.path().join("android-test");
    assert!(root.join("platform/android/settings.gradle.kts").exists());
    assert!(root.join("platform/android/app/src/main/AndroidManifest.xml").exists());
    assert!(root.join("platform/android/app/src/main/java/com/example/android_test/MainActivity.kt").exists());
    assert!(root.join("platform/android/build-rust.sh").exists());

    let manifest = std::fs::read_to_string(root.join("platform/android/app/src/main/AndroidManifest.xml")).unwrap();
    assert!(manifest.contains("android-test"));
    assert!(!manifest.contains("{{"));
}
