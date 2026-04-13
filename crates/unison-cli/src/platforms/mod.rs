pub mod web;
pub mod ios;
pub mod android;

use anyhow::Result;
use std::path::Path;
use crate::toolchain::{Invocation, Invoker};

/// Runs `npx tstl -p project/scripts-src/tsconfig.json` once. Used by `build`/`dev` before
/// a platform build so we don't re-run it per-platform when building all.
pub fn run_tstl(project_root: &Path, invoker: &dyn Invoker) -> Result<()> {
    let inv = Invocation::new("npx", project_root)
        .arg("tstl")
        .arg("-p")
        .arg("project/scripts-src/tsconfig.json");
    let out = invoker.run(&inv)?;
    if out.status != 0 {
        anyhow::bail!("tstl failed:\n{}", out.stderr);
    }
    Ok(())
}
