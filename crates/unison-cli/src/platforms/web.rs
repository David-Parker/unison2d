use anyhow::Result;
use std::path::Path;

use crate::toolchain::{Invocation, Invoker};

pub struct WebBuildArgs {
    pub release: bool,
    pub profile: bool,
}

pub fn build(project_root: &Path, invoker: &dyn Invoker, args: WebBuildArgs, run_tstl: bool) -> Result<()> {
    if run_tstl {
        let inv = Invocation::new("npx", project_root)
            .arg("tstl")
            .arg("-p")
            .arg("project/scripts-src/tsconfig.json");
        let out = invoker.run(&inv)?;
        if out.status != 0 {
            anyhow::bail!("tstl failed:\n{}", out.stderr);
        }
    }
    let mut inv = Invocation::new("trunk", project_root.join("platform/web")).arg("build");
    if args.release { inv = inv.arg("--release"); }
    if args.profile {
        inv = inv.arg("--features").arg("profiling");
    }
    let out = invoker.run(&inv)?;
    if out.status != 0 {
        anyhow::bail!("trunk build failed:\n{}", out.stderr);
    }
    Ok(())
}
