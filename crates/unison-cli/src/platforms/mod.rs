pub mod web;
pub mod ios;
pub mod android;

use anyhow::Result;
use std::path::Path;
use crate::toolchain::{Invocation, Invoker};

/// Runs `npx tstl -p project/scripts-src/tsconfig.json` once. Used by `build`/`dev` before
/// a platform build so we don't re-run it per-platform when building all.
///
/// Auto-runs `npm install` first if `node_modules/` is missing — otherwise `npx` would
/// silently fail to find `tstl` and emit the unhelpful "could not determine executable
/// to run" error. Freshly-scaffolded TS projects always need this on first build.
pub fn run_tstl(project_root: &Path, invoker: &dyn Invoker) -> Result<()> {
    ensure_npm_deps(project_root, invoker)?;
    let inv = Invocation::new("npx", project_root)
        .arg("tstl")
        .arg("-p")
        .arg("project/scripts-src/tsconfig.json")
        .streaming();
    let out = invoker.run(&inv)?;
    if out.status != 0 {
        anyhow::bail!("tstl failed (exit {}) — see output above", out.status);
    }
    Ok(())
}

/// Run `npm install` if `node_modules/` is missing. No-op when deps are already present.
pub fn ensure_npm_deps(project_root: &Path, invoker: &dyn Invoker) -> Result<()> {
    if project_root.join("node_modules").exists() {
        return Ok(());
    }
    if !project_root.join("package.json").exists() {
        return Ok(());
    }
    eprintln!("node_modules/ missing — running `npm install`…");
    let inv = Invocation::new("npm", project_root).arg("install").streaming();
    let out = invoker.run(&inv)?;
    if out.status != 0 {
        anyhow::bail!("npm install failed (exit {}) — see output above", out.status);
    }
    Ok(())
}
