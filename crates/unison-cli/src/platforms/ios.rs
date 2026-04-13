use anyhow::Result;
use std::path::Path;

use crate::toolchain::{Invocation, Invoker};

pub struct IosBuildArgs {
    pub release: bool,
    pub profile: bool,
    pub project_name: String,
}

pub fn build(project_root: &Path, invoker: &dyn Invoker, args: IosBuildArgs) -> Result<()> {
    let xcodeproj = format!("platform/ios/{}-ios.xcodeproj", args.project_name);
    let scheme = format!("{}-ios", args.project_name);
    let configuration = if args.release { "Release" } else { "Debug" };

    let mut inv = Invocation::new("xcodebuild", project_root)
        .arg("-project").arg(&xcodeproj)
        .arg("-scheme").arg(&scheme)
        .arg("-configuration").arg(configuration)
        .arg("build");
    if args.profile {
        inv = inv.env("UNISON_PROFILING", "1");
    }
    let out = invoker.run(&inv.streaming())?;
    if out.status != 0 {
        anyhow::bail!("xcodebuild failed (exit {}) — see output above", out.status);
    }
    Ok(())
}
