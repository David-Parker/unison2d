use anyhow::Result;
use std::path::Path;

use crate::toolchain::{Invocation, Invoker};

pub struct AndroidBuildArgs {
    pub release: bool,
    pub profile: bool,
}

pub fn build(project_root: &Path, invoker: &dyn Invoker, args: AndroidBuildArgs) -> Result<()> {
    let gradle_dir = project_root.join("platform/android");
    let task = if args.release { ":app:assembleRelease" } else { ":app:assembleDebug" };
    let mut inv = Invocation::new("./gradlew", &gradle_dir).arg(task);
    if args.profile {
        inv = inv.arg("-Pprofile=true");
    }
    let out = invoker.run(&inv.streaming())?;
    if out.status != 0 {
        anyhow::bail!("gradle failed (exit {}) — see output above", out.status);
    }
    Ok(())
}
