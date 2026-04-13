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
    remove_legacy_patch_block(project_root)?;
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
    remove_legacy_patch_block(project_root)?;
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
fn remove_legacy_patch_block(project_root: &Path) -> Result<()> {
    Config::edit_in_place(&project_root.join("Cargo.toml"), |doc| {
        if let Some(patch) = doc.get_mut("patch").and_then(|i| i.as_table_mut()) {
            patch.clear();
        }
        if let Some(tbl) = doc.as_table().get("patch").and_then(|i| i.as_table()) {
            if tbl.is_empty() {
                doc.as_table_mut().remove("patch");
            }
        }
        Ok(())
    })
}
