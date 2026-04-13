use anyhow::{bail, Result};
use std::path::Path;

use crate::config::{Config, Lang};
use crate::toolchain::{Invocation, Invoker, SystemInvoker};

pub fn run(project_root: &Path) -> Result<()> {
    let cfg = Config::load(&project_root.join("unison.toml"))?;
    run_with(&cfg, project_root, &SystemInvoker)
}

pub fn run_with(cfg: &Config, project_root: &Path, invoker: &dyn Invoker) -> Result<()> {
    let out = invoker.run(&Invocation::new("cargo", project_root).arg("test").streaming())?;
    if out.status != 0 { bail!("cargo test failed (exit {}) — see output above", out.status); }
    if matches!(cfg.project.lang, Lang::Ts) {
        let out = invoker.run(&Invocation::new("npm", project_root).arg("test").streaming())?;
        if out.status != 0 { bail!("npm test failed (exit {}) — see output above", out.status); }
    }
    Ok(())
}
