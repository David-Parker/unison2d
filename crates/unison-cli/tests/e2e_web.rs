use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

fn have(bin: &str) -> bool {
    Command::new(bin)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Absolute path to the unison2d workspace root (two levels up from this
/// crate's manifest). Used to `unison link` the scaffolded project at the
/// local engine instead of the published git tag.
fn engine_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()  // crates/
        .parent().unwrap()  // unison2d/
        .to_path_buf()
}

/// End-to-end: scaffold a fresh project, link it at the local engine, and run
/// the full `unison build web` pipeline. This would have caught the template
/// regression where `platform/web/index.html` was missing
/// `href="../../Cargo.toml"` on the trunk rust link.
///
/// Heavy (runs trunk + cargo + wasm-bindgen) so it's `#[ignore]`d by default.
/// Run with: `cargo test --test e2e_web -- --ignored`.
#[test]
#[ignore]
fn e2e_scaffold_link_build_web() {
    if !have("trunk") || !have("cargo") {
        eprintln!("skipping: trunk or cargo missing");
        return;
    }
    let dir = tempdir().unwrap();
    let unison_bin = env!("CARGO_BIN_EXE_unison");

    // 1. Scaffold.
    let out = Command::new(unison_bin)
        .args(["new", "smoke", "--no-ios", "--no-android", "--no-git"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "unison new failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let root = dir.path().join("smoke");

    // 2. Link at the local engine so we don't need network or a published tag.
    let engine = engine_root();
    let out = Command::new(unison_bin)
        .args(["link", engine.to_str().unwrap()])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "unison link failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // 3. Full web build via the CLI — exercises the same code path end users hit.
    let out = Command::new(unison_bin)
        .args(["build", "web"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "unison build web failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(root.join("platform/web/dist").exists(), "expected dist/ after build");
}
