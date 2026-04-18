//! End-to-end tests that scaffold a fresh TypeScript project and drive real
//! platform builds (web / iOS / Android). Heavy — each is gated behind
//! `#[ignore]` and a toolchain probe so they're opt-in and skip cleanly on
//! machines without the toolchain.
//!
//! Run individually:
//!   cargo test --test e2e_ts e2e_ts_build_web     -- --ignored --nocapture
//!   cargo test --test e2e_ts e2e_ts_build_ios     -- --ignored --nocapture
//!   cargo test --test e2e_ts e2e_ts_build_android -- --ignored --nocapture
//!
//! Run everything (slow):
//!   cargo test --test e2e_ts -- --ignored --nocapture --test-threads=1

use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::{tempdir, TempDir};

fn have(bin: &str) -> bool {
    // xcodebuild only accepts single-dash `-version`, so probe with that
    // form for Apple-family tools and --version for everyone else.
    let version_flag = if bin == "xcodebuild" { "-version" } else { "--version" };
    Command::new(bin)
        .arg(version_flag)
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

/// Scaffold a fresh TS project in a tempdir and link it at the local engine.
/// Returns the tempdir (must be held for the lifetime of the test) and the
/// project root path.
fn scaffold_ts_project() -> (TempDir, PathBuf) {
    let dir = tempdir().unwrap();
    let unison_bin = env!("CARGO_BIN_EXE_unison");

    let out = Command::new(unison_bin)
        .args(["new", "smoke", "--lang", "ts", "--no-git"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "unison new --lang ts failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let root = dir.path().join("smoke");

    // Sanity: engine types should have been copied under scripts-src/types/
    assert!(
        root.join("project/scripts-src/types/unison2d/unison.d.ts").exists(),
        "engine types missing from scaffolded TS project",
    );

    let engine = engine_root();
    let out = Command::new(unison_bin)
        .args(["link", engine.to_str().unwrap()])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "unison link failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );

    (dir, root)
}

/// Run a CLI command and fail the test with full output on non-zero exit.
fn run_unison(root: &Path, args: &[&str]) {
    let unison_bin = env!("CARGO_BIN_EXE_unison");
    let out = Command::new(unison_bin)
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "unison {} failed:\nstdout: {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

/// Lightweight type-check: scaffolds a TS project, installs deps, and runs
/// `tsc --noEmit`. Catches stale template ↔ type-declaration mismatches
/// (e.g. template calling world.spawn_soft_body() when the method moved to
/// world.objects.spawn_soft_body()) without needing trunk or wasm.
#[test]
#[ignore]
fn e2e_ts_typecheck() {
    if !have("npm") {
        eprintln!("skipping: npm missing");
        return;
    }
    let (_tmp, root) = scaffold_ts_project();

    let install = Command::new("npm")
        .arg("install")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        install.status.success(),
        "npm install failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&install.stdout),
        String::from_utf8_lossy(&install.stderr),
    );

    let tsc = Command::new("npx")
        .args(["tsc", "--noEmit"])
        .current_dir(root.join("project/scripts-src"))
        .output()
        .unwrap();
    assert!(
        tsc.status.success(),
        "tsc --noEmit failed — template does not type-check against engine declarations:\n{}",
        String::from_utf8_lossy(&tsc.stdout),
    );
}

/// Would have caught: missing engine types in TS template, broken tsconfig
/// paths (`../../assets` typo, missing language-extensions include, wrong
/// `module` for `export =`), missing `node_modules/` auto-install in
/// `run_tstl`.
#[test]
#[ignore]
fn e2e_ts_build_web() {
    if !have("trunk") || !have("cargo") || !have("npm") {
        eprintln!("skipping: trunk, cargo, or npm missing");
        return;
    }
    let (_tmp, root) = scaffold_ts_project();
    run_unison(&root, &["build", "web"]);
    assert!(
        root.join("platform/web/dist").exists(),
        "expected platform/web/dist/ after build",
    );
    assert!(
        root.join("project/assets/scripts/main.lua").exists(),
        "expected tstl output at project/assets/scripts/main.lua",
    );
}

#[test]
#[ignore]
fn e2e_ts_build_ios() {
    if !have("xcodebuild") || !have("cargo") || !have("npm") {
        eprintln!("skipping: xcodebuild, cargo, or npm missing");
        return;
    }
    let (_tmp, root) = scaffold_ts_project();
    run_unison(&root, &["build", "ios"]);
}

#[test]
#[ignore]
fn e2e_ts_build_android() {
    // Gradle isn't usually on PATH; the generated `./gradlew` drives the
    // build and needs java on PATH. Skip if any link in the chain is
    // missing. (`cargo test` doesn't source ~/.zshrc, so JAVA_HOME may be
    // unset even when the user has it exported interactively.)
    let has_ndk = std::env::var("ANDROID_NDK_HOME").is_ok()
        || PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join("Library/Android/sdk/ndk")
            .exists();
    if !has_ndk || !have("cargo") || !have("npm") || !have("java") {
        eprintln!("skipping: NDK, cargo, npm, or java missing");
        return;
    }
    let (_tmp, root) = scaffold_ts_project();
    run_unison(&root, &["build", "android"]);
}
