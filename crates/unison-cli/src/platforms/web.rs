use anyhow::Result;
use std::path::Path;

use crate::toolchain::{Invocation, Invoker};

pub struct WebBuildArgs {
    pub release: bool,
    pub profile: bool,
}

pub fn build(project_root: &Path, invoker: &dyn Invoker, args: WebBuildArgs) -> Result<()> {
    let mut inv = Invocation::new("trunk", project_root.join("platform/web")).arg("build");
    if args.release { inv = inv.arg("--release"); }
    if args.profile {
        inv = inv.arg("--features").arg("profiling");
    }
    let out = invoker.run(&inv.streaming())?;
    if out.status != 0 {
        anyhow::bail!("trunk build failed (exit {}) — see output above", out.status);
    }
    Ok(())
}
