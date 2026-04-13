use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

use crate::config::Config;

/// Engine crates that `unison new` adds as dependencies. Listed with the
/// `[dependencies]` vs `[build-dependencies]` section they belong to so that
/// `link` / `unlink` can find and rewrite them in place.
const ENGINE_DEPS: &[(&str, &str)] = &[
    ("dependencies", "unison-scripting"),
    ("build-dependencies", "unison-assets"),
];

pub fn link(project_root: &Path, engine_path: &str) -> Result<()> {
    let engine_abs = fs::canonicalize(engine_path)
        .with_context(|| format!("resolving {}", engine_path))?;
    validate_engine_workspace(&engine_abs)?;
    rewrite_deps_to_path(project_root, &engine_abs)?;
    remove_legacy_git_patch_block(project_root)?;
    add_vendor_patch(project_root, &engine_abs)?;
    rewrite_xcode_spm_to_local(project_root, &engine_abs)?;
    rewrite_android_engine_path(project_root, Some(&engine_abs))?;
    let engine_path_owned = engine_path.to_string();
    Config::edit_in_place(&project_root.join("unison.toml"), |doc| {
        doc["engine"]["link_path"] = toml_edit::value(engine_path_owned.clone());
        Ok(())
    })?;
    println!("Linked {} to engine at {}", project_root.display(), engine_abs.display());
    Ok(())
}

pub fn unlink(project_root: &Path) -> Result<()> {
    let cfg = Config::load(&project_root.join("unison.toml"))?;
    rewrite_deps_to_git(project_root, &cfg)?;
    remove_legacy_git_patch_block(project_root)?;
    remove_vendor_patch(project_root)?;
    rewrite_xcode_spm_to_remote(project_root, &cfg)?;
    rewrite_android_engine_path(project_root, None)?;
    Config::edit_in_place(&project_root.join("unison.toml"), |doc| {
        if let Some(engine) = doc.get_mut("engine").and_then(|i| i.as_table_mut()) {
            engine.remove("link_path");
        }
        Ok(())
    })?;
    println!("Unlinked.");
    Ok(())
}

fn validate_engine_workspace(engine_abs: &Path) -> Result<()> {
    for (_section, crate_name) in ENGINE_DEPS {
        if !engine_abs.join("crates").join(crate_name).join("Cargo.toml").exists() {
            bail!(
                "{} does not look like a unison2d workspace (missing crates/{}/Cargo.toml)",
                engine_abs.display(), crate_name
            );
        }
    }
    Ok(())
}

/// Rewrite each engine dep from `{ git = ..., tag = ..., features = [...] }`
/// to `{ path = "<engine>/crates/<name>", features = [...] }`, preserving the
/// existing `features` array. This is more reliable than `[patch]` — cargo
/// won't try to fetch the git source at all when the dep points at a path.
fn rewrite_deps_to_path(project_root: &Path, engine_abs: &Path) -> Result<()> {
    let engine_abs = engine_abs.to_path_buf();
    Config::edit_in_place(&project_root.join("Cargo.toml"), |doc| {
        for (section, crate_name) in ENGINE_DEPS {
            let Some(tbl) = doc.get_mut(*section).and_then(|i| i.as_table_mut()) else { continue };
            let Some(existing) = tbl.get(*crate_name) else { continue };
            let features = existing
                .as_inline_table()
                .and_then(|t| t.get("features"))
                .cloned();

            let mut inline = toml_edit::InlineTable::new();
            let crate_path = engine_abs.join("crates").join(crate_name);
            inline.insert("path", toml_edit::Value::from(crate_path.display().to_string()));
            if let Some(f) = features {
                inline.insert("features", f);
            }
            tbl.insert(
                crate_name,
                toml_edit::Item::Value(toml_edit::Value::InlineTable(inline)),
            );
        }
        Ok(())
    })
}

/// Inverse of `rewrite_deps_to_path` — restore each engine dep to its git
/// spec using `unison.toml`'s `[engine]` section as the source of truth.
fn rewrite_deps_to_git(project_root: &Path, cfg: &Config) -> Result<()> {
    let git = cfg.engine.git.clone();
    let tag = cfg.engine.tag.clone();
    let branch = cfg.engine.branch.clone();
    let rev = cfg.engine.rev.clone();
    Config::edit_in_place(&project_root.join("Cargo.toml"), |doc| {
        for (section, crate_name) in ENGINE_DEPS {
            let Some(tbl) = doc.get_mut(*section).and_then(|i| i.as_table_mut()) else { continue };
            let Some(existing) = tbl.get(*crate_name) else { continue };
            let features = existing
                .as_inline_table()
                .and_then(|t| t.get("features"))
                .cloned();

            let mut inline = toml_edit::InlineTable::new();
            inline.insert("git", toml_edit::Value::from(&git));
            if let Some(t) = &tag { inline.insert("tag", toml_edit::Value::from(t)); }
            if let Some(b) = &branch { inline.insert("branch", toml_edit::Value::from(b)); }
            if let Some(r) = &rev { inline.insert("rev", toml_edit::Value::from(r)); }
            if let Some(f) = features {
                inline.insert("features", f);
            }
            tbl.insert(
                crate_name,
                toml_edit::Item::Value(toml_edit::Value::InlineTable(inline)),
            );
        }
        Ok(())
    })
}

/// Earlier versions of `unison link` added a `[patch."<git-url>"]` block
/// instead of rewriting deps. Cargo fetches the git source to validate patches,
/// so that approach broke when the engine tag wasn't published yet. Strip any
/// such leftover block so upgraded projects aren't left with dead entries.
fn remove_legacy_git_patch_block(project_root: &Path) -> Result<()> {
    Config::edit_in_place(&project_root.join("Cargo.toml"), |doc| {
        let Some(patch) = doc.get_mut("patch").and_then(|i| i.as_table_mut()) else { return Ok(()) };
        // Remove every [patch."<url>"] entry (anything that isn't the
        // crates-io subtable). The vendor lua-src patch lives under
        // [patch.crates-io] and is managed by add_vendor_patch.
        let keys: Vec<String> = patch
            .iter()
            .filter(|(k, _)| *k != "crates-io")
            .map(|(k, _)| k.to_string())
            .collect();
        for k in keys {
            patch.remove(&k);
        }
        if patch.is_empty() {
            doc.as_table_mut().remove("patch");
        }
        Ok(())
    })
}

/// Add `[patch.crates-io] lua-src = { path = "<engine>/vendor/lua-src" }`.
/// The engine ships a forked lua-src with wasm32 support; building for web
/// without this patch fails in `mlua-sys` with "don't know how to build Lua
/// for wasm32-unknown-unknown".
fn add_vendor_patch(project_root: &Path, engine_abs: &Path) -> Result<()> {
    let vendor = engine_abs.join("vendor").join("lua-src");
    let vendor_str = vendor.display().to_string();
    Config::edit_in_place(&project_root.join("Cargo.toml"), |doc| {
        let patch = doc.entry("patch").or_insert_with(|| {
            let mut t = toml_edit::Table::new();
            t.set_implicit(true);
            toml_edit::Item::Table(t)
        });
        let patch_tbl = patch.as_table_mut()
            .ok_or_else(|| anyhow::anyhow!("[patch] is not a table"))?;
        patch_tbl.set_implicit(true);
        let crates_io = patch_tbl.entry("crates-io").or_insert_with(|| {
            toml_edit::Item::Table(toml_edit::Table::new())
        });
        let crates_io_tbl = crates_io.as_table_mut()
            .ok_or_else(|| anyhow::anyhow!("[patch.crates-io] is not a table"))?;
        let mut inline = toml_edit::InlineTable::new();
        inline.insert("path", toml_edit::Value::from(vendor_str.clone()));
        crates_io_tbl.insert("lua-src", toml_edit::Item::Value(toml_edit::Value::InlineTable(inline)));
        Ok(())
    })
}

/// Swap every `XCRemoteSwiftPackageReference` that points at the engine git
/// URL for an `XCLocalSwiftPackageReference` pointing at `engine_abs`. Xcode
/// treats local references as on-disk packages and skips the network resolve.
///
/// This mirrors what `rewrite_deps_to_path` does for Cargo: symmetric linking
/// across both Rust and Swift dependency graphs.
fn rewrite_xcode_spm_to_local(project_root: &Path, engine_abs: &Path) -> Result<()> {
    // Point at the engine workspace root — `Package.swift` there re-exports
    // the `UnisoniOS` product. Using the root (rather than the inner
    // `crates/unison-ios/UnisoniOS` package) lets each consumer resolve its
    // own independent SwiftPM package, avoiding Xcode's "already opened from
    // another project" error when two Unison projects are open at once.
    let local_path = engine_abs.display().to_string();
    for_each_pbxproj(project_root, |pbxproj| {
        let text = fs::read_to_string(pbxproj)
            .with_context(|| format!("reading {}", pbxproj.display()))?;
        let rewritten = pbxproj_remote_to_local(&text, &local_path);
        if rewritten != text {
            fs::write(pbxproj, rewritten)
                .with_context(|| format!("writing {}", pbxproj.display()))?;
        }
        Ok(())
    })
}

/// Inverse of `rewrite_xcode_spm_to_local` — regenerate the remote reference
/// block from `unison.toml`'s `[engine]` git/tag.
fn rewrite_xcode_spm_to_remote(project_root: &Path, cfg: &Config) -> Result<()> {
    let git = cfg.engine.git.clone();
    let tag = cfg.engine.tag.clone().unwrap_or_default();
    let version = tag.strip_prefix('v').unwrap_or(&tag).to_string();
    for_each_pbxproj(project_root, |pbxproj| {
        let text = fs::read_to_string(pbxproj)
            .with_context(|| format!("reading {}", pbxproj.display()))?;
        let rewritten = pbxproj_local_to_remote(&text, &git, &version);
        if rewritten != text {
            fs::write(pbxproj, rewritten)
                .with_context(|| format!("writing {}", pbxproj.display()))?;
        }
        Ok(())
    })
}

fn for_each_pbxproj(
    project_root: &Path,
    mut f: impl FnMut(&Path) -> Result<()>,
) -> Result<()> {
    let ios_dir = project_root.join("platform").join("ios");
    if !ios_dir.exists() { return Ok(()); }
    for entry in fs::read_dir(&ios_dir)
        .with_context(|| format!("reading {}", ios_dir.display()))?
    {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("xcodeproj") { continue; }
        let pbxproj = path.join("project.pbxproj");
        if pbxproj.exists() { f(&pbxproj)?; }
    }
    Ok(())
}

fn pbxproj_remote_to_local(text: &str, local_path: &str) -> String {
    use regex::Regex;
    let block = Regex::new(
        r#"(?ms)^(\t\t[0-9A-F]{24}) /\* XCRemoteSwiftPackageReference "[^"]*" \*/ = \{\s*isa = XCRemoteSwiftPackageReference;\s*repositoryURL = "[^"]*";\s*requirement = \{[^}]*\};\s*\};"#,
    ).unwrap();
    // The replacement uses raw-string substitution so `local_path` can contain
    // any character (including `$`) without being misinterpreted by the regex
    // replacement engine.
    let replacement = format!(
        "$1 /* XCLocalSwiftPackageReference \"{local_path}\" */ = {{\n\t\t\tisa = XCLocalSwiftPackageReference;\n\t\t\trelativePath = \"{local_path}\";\n\t\t}};"
    );
    let out = block.replace_all(text, replacement.as_str()).into_owned();

    // Fix up stale inline `/* XCRemoteSwiftPackageReference "url" */` comments
    // that appear on product-dependency and packageReferences entries.
    let comment = Regex::new(r#"/\* XCRemoteSwiftPackageReference "[^"]*" \*/"#).unwrap();
    let new_comment = format!(r#"/* XCLocalSwiftPackageReference "{local_path}" */"#);
    comment.replace_all(&out, regex::NoExpand(&new_comment)).into_owned()
}

fn pbxproj_local_to_remote(text: &str, git_url: &str, version: &str) -> String {
    use regex::Regex;
    let block = Regex::new(
        r#"(?ms)^(\t\t[0-9A-F]{24}) /\* XCLocalSwiftPackageReference "[^"]*" \*/ = \{\s*isa = XCLocalSwiftPackageReference;\s*relativePath = "[^"]*";\s*\};"#,
    ).unwrap();
    let replacement = format!(
        "$1 /* XCRemoteSwiftPackageReference \"{git_url}\" */ = {{\n\t\t\tisa = XCRemoteSwiftPackageReference;\n\t\t\trepositoryURL = \"{git_url}\";\n\t\t\trequirement = {{\n\t\t\t\tkind = exactVersion;\n\t\t\t\tversion = {version};\n\t\t\t}};\n\t\t}};"
    );
    let out = block.replace_all(text, replacement.as_str()).into_owned();

    let comment = Regex::new(r#"/\* XCLocalSwiftPackageReference "[^"]*" \*/"#).unwrap();
    let new_comment = format!(r#"/* XCRemoteSwiftPackageReference "{git_url}" */"#);
    comment.replace_all(&out, regex::NoExpand(&new_comment)).into_owned()
}

/// Rewrite the `project(":unison-android").projectDir = file("...")` line in
/// `platform/android/settings.gradle.kts` to point at the linked engine (when
/// `engine_abs` is `Some`) or back to the template default (when `None`).
///
/// Why this exists: Android's gradle config pulls the engine's
/// `UnisonAndroid` Kotlin module as a subproject via a filesystem path. That
/// path only resolves when the consumer project is next to the engine checkout
/// — linked projects have the engine elsewhere, so we rewrite.
fn rewrite_android_engine_path(project_root: &Path, engine_abs: Option<&Path>) -> Result<()> {
    let settings = project_root.join("platform/android/settings.gradle.kts");
    if !settings.exists() { return Ok(()); }
    let text = fs::read_to_string(&settings)
        .with_context(|| format!("reading {}", settings.display()))?;
    let new_path = match engine_abs {
        Some(abs) => format!("{}/crates/unison-android/UnisonAndroid", abs.display()),
        None => "../../unison2d/crates/unison-android/UnisonAndroid".to_string(),
    };
    let re = regex::Regex::new(
        r#"(?m)^(\s*project\(":unison-android"\)\.projectDir\s*=\s*\n?\s*file\()"[^"]*"(\)\s*)$"#,
    ).unwrap();
    // The pattern above matches a single-line form; handle the common
    // two-line form separately with a simpler scan.
    let rewritten = if re.is_match(&text) {
        re.replace_all(&text, |caps: &regex::Captures| {
            format!("{}\"{}\"{}", &caps[1], new_path, &caps[2])
        }).into_owned()
    } else {
        // Fall back to a tolerant replacement that doesn't care about line breaks.
        let re_multi = regex::Regex::new(
            r#"(?ms)(project\(":unison-android"\)\.projectDir\s*=\s*\n?\s*file\()"[^"]*"(\))"#,
        ).unwrap();
        re_multi.replace_all(&text, |caps: &regex::Captures| {
            format!("{}\"{}\"{}", &caps[1], new_path, &caps[2])
        }).into_owned()
    };
    if rewritten != text {
        fs::write(&settings, rewritten)
            .with_context(|| format!("writing {}", settings.display()))?;
    }
    Ok(())
}

fn remove_vendor_patch(project_root: &Path) -> Result<()> {
    Config::edit_in_place(&project_root.join("Cargo.toml"), |doc| {
        let Some(patch) = doc.get_mut("patch").and_then(|i| i.as_table_mut()) else { return Ok(()) };
        if let Some(crates_io) = patch.get_mut("crates-io").and_then(|i| i.as_table_mut()) {
            crates_io.remove("lua-src");
            if crates_io.is_empty() {
                patch.remove("crates-io");
            }
        }
        if patch.is_empty() {
            doc.as_table_mut().remove("patch");
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const REMOTE_BLOCK: &str = "/* Begin XCRemoteSwiftPackageReference section */\n\t\t631F83672F7442A9003D446A /* XCRemoteSwiftPackageReference \"https://github.com/David-Parker/unison2d\" */ = {\n\t\t\tisa = XCRemoteSwiftPackageReference;\n\t\t\trepositoryURL = \"https://github.com/David-Parker/unison2d\";\n\t\t\trequirement = {\n\t\t\t\tkind = exactVersion;\n\t\t\t\tversion = 0.1.0;\n\t\t\t};\n\t\t};\n/* End XCRemoteSwiftPackageReference section */\n\t\t\tpackage = 631F83672F7442A9003D446A /* XCRemoteSwiftPackageReference \"https://github.com/David-Parker/unison2d\" */;\n";

    #[test]
    fn remote_to_local_rewrites_block_and_comments() {
        let out = pbxproj_remote_to_local(REMOTE_BLOCK, "/abs/engine");
        assert!(out.contains("isa = XCLocalSwiftPackageReference;"));
        assert!(out.contains("relativePath = \"/abs/engine\";"));
        assert!(!out.contains("isa = XCRemoteSwiftPackageReference;"));
        assert!(!out.contains("repositoryURL"));
        assert!(!out.contains("requirement"));
        // Inline product-dependency comment also rewritten.
        assert!(out.contains("/* XCLocalSwiftPackageReference \"/abs/engine\" */"));
        // Stale inline comments on other references should also be replaced.
        assert!(!out.contains(r#"/* XCRemoteSwiftPackageReference "https"#));
    }

    #[test]
    fn local_to_remote_round_trips() {
        let local = pbxproj_remote_to_local(REMOTE_BLOCK, "/abs/engine");
        let remote = pbxproj_local_to_remote(&local, "https://github.com/David-Parker/unison2d", "0.1.0");
        assert!(remote.contains("isa = XCRemoteSwiftPackageReference;"));
        assert!(remote.contains("repositoryURL = \"https://github.com/David-Parker/unison2d\";"));
        assert!(remote.contains("kind = exactVersion;"));
        assert!(remote.contains("version = 0.1.0;"));
        assert!(!remote.contains("XCLocalSwiftPackageReference"));
        assert!(!remote.contains("relativePath"));
    }

    #[test]
    fn no_op_when_no_spm_block_present() {
        let input = "/* some other pbxproj content */\n";
        assert_eq!(pbxproj_remote_to_local(input, "/abs"), input);
        assert_eq!(pbxproj_local_to_remote(input, "url", "1.0.0"), input);
    }
}
