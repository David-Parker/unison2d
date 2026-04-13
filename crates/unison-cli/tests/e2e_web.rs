use std::process::Command;
use tempfile::tempdir;

fn have(bin: &str) -> bool {
    Command::new(bin)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
#[ignore] // opt-in: cargo test --test e2e_web -- --ignored
fn e2e_scaffold_and_build_web() {
    if !have("trunk") || !have("cargo") {
        eprintln!("skipping: trunk or cargo missing");
        return;
    }
    let dir = tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_unison"))
        .args(["new", "smoke", "--no-ios", "--no-android", "--no-git"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "new failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let root = dir.path().join("smoke");

    // Cargo check (will fetch the engine git dep — needs internet).
    let out = Command::new("cargo")
        .arg("check")
        .arg("--no-default-features")
        .arg("--features")
        .arg("web")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "cargo check failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Trunk build.
    let out = Command::new("trunk")
        .arg("build")
        .current_dir(root.join("platform/web"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "trunk build failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(root.join("platform/web/dist").exists());
}
