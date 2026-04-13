use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

use crate::config::Config;

const ENGINE_CRATES: &[&str] = &["unison-scripting", "unison-assets"];

pub fn link(project_root: &Path, engine_path: &str) -> Result<()> {
    let engine_abs = fs::canonicalize(engine_path)
        .with_context(|| format!("resolving {}", engine_path))?;
    validate_engine_workspace(&engine_abs)?;
    let cfg = Config::load(&project_root.join("unison.toml"))?;
    write_patch(project_root, &cfg.engine.git, &engine_abs)?;
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
    remove_patch(project_root, &cfg.engine.git)?;
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
    for crate_name in ENGINE_CRATES {
        if !engine_abs.join("crates").join(crate_name).join("Cargo.toml").exists() {
            bail!(
                "{} does not look like a unison2d workspace (missing crates/{}/Cargo.toml)",
                engine_abs.display(), crate_name
            );
        }
    }
    Ok(())
}

fn write_patch(project_root: &Path, engine_git: &str, engine_abs: &Path) -> Result<()> {
    let engine_git = engine_git.to_string();
    let engine_abs = engine_abs.to_path_buf();
    Config::edit_in_place(&project_root.join("Cargo.toml"), |doc| {
        // Build the inner table: one entry per engine crate, each { path = "..." }
        let mut tbl = toml_edit::Table::new();
        tbl.set_implicit(false);
        for c in ENGINE_CRATES {
            let mut inline = toml_edit::InlineTable::new();
            let crate_path = engine_abs.join("crates").join(c);
            inline.insert("path", toml_edit::Value::from(crate_path.display().to_string()));
            tbl.insert(c, toml_edit::Item::Value(toml_edit::Value::InlineTable(inline)));
        }
        // Ensure [patch] exists as an implicit parent table so the child renders as
        // [patch."<url>"] rather than dotted [patch] block.
        let patch = doc.entry("patch").or_insert_with(|| {
            let mut t = toml_edit::Table::new();
            t.set_implicit(true);
            toml_edit::Item::Table(t)
        });
        let patch_tbl = patch.as_table_mut()
            .ok_or_else(|| anyhow::anyhow!("[patch] is not a table"))?;
        patch_tbl.set_implicit(true);
        patch_tbl.insert(&engine_git, toml_edit::Item::Table(tbl));
        Ok(())
    })
}

fn remove_patch(project_root: &Path, engine_git: &str) -> Result<()> {
    let engine_git = engine_git.to_string();
    Config::edit_in_place(&project_root.join("Cargo.toml"), |doc| {
        if let Some(patch) = doc.get_mut("patch").and_then(|i| i.as_table_mut()) {
            patch.remove(&engine_git);
            // If patch is now empty, remove it entirely to keep Cargo.toml tidy.
            if patch.is_empty() {
                doc.as_table_mut().remove("patch");
            }
        }
        Ok(())
    })
}
