use std::process::Command;
use tempfile::tempdir;
use walkdir::WalkDir;

fn scaffold(args: &[&str]) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_unison"));
    cmd.arg("new");
    for a in args { cmd.arg(a); }
    cmd.arg("--no-git").current_dir(dir.path());
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let root = dir.path().join(args.first().unwrap());
    (dir, root)
}

fn file_tree(root: &std::path::Path) -> String {
    let mut lines: Vec<String> = WalkDir::new(root)
        .sort_by_file_name()
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().strip_prefix(root).unwrap().display().to_string())
        .collect();
    lines.sort();
    lines.join("\n")
}

fn captured(root: &std::path::Path, rel: &str) -> String {
    std::fs::read_to_string(root.join(rel)).unwrap_or_else(|_| String::new())
}

#[test]
fn snapshot_lua_web_only() {
    let (_d, root) = scaffold(&["g", "--no-ios", "--no-android"]);
    insta::assert_snapshot!("lua-web-only-tree", file_tree(&root));
    insta::assert_snapshot!("lua-web-only-cargo", captured(&root, "Cargo.toml"));
    insta::assert_snapshot!("lua-web-only-unison-toml", captured(&root, "unison.toml"));
}

#[test]
fn snapshot_lua_all_platforms() {
    let (_d, root) = scaffold(&["g"]);
    insta::assert_snapshot!("lua-all-tree", file_tree(&root));
    insta::assert_snapshot!("lua-all-cargo", captured(&root, "Cargo.toml"));
}

#[test]
fn snapshot_ts_web_only() {
    let (_d, root) = scaffold(&["g", "--lang", "ts", "--no-ios", "--no-android"]);
    insta::assert_snapshot!("ts-web-tree", file_tree(&root));
}

#[test]
fn snapshot_ts_all_platforms() {
    let (_d, root) = scaffold(&["g", "--lang", "ts"]);
    insta::assert_snapshot!("ts-all-tree", file_tree(&root));
}
