use anyhow::{bail, Result};
use std::path::Path;

use crate::config::{Config, Lang};
use crate::toolchain::{Invocation, Invoker, SystemInvoker};

pub fn run(project_root: &Path, platform: &str) -> Result<()> {
    let cfg = Config::load(&project_root.join("unison.toml"))?;
    let invoker = SystemInvoker;
    run_with(&cfg, project_root, &invoker, platform)
}

pub fn run_with(cfg: &Config, project_root: &Path, invoker: &dyn Invoker, platform: &str) -> Result<()> {
    match platform {
        "web" => {
            if !cfg.platforms.web { bail!("web is not enabled in unison.toml"); }
            if matches!(cfg.project.lang, Lang::Ts) {
                // Spawn tstl --watch. In the real SystemInvoker this blocks; for v1 we run
                // it synchronously and rely on trunk serve's own reload for the main loop.
                // A follow-up can introduce Invoker::spawn() for true parallelism.
                let inv = Invocation::new("npx", project_root)
                    .arg("tstl").arg("-p").arg("project/scripts-src/tsconfig.json").arg("--watch");
                invoker.run(&inv)?;
            }
            let inv = Invocation::new("trunk", project_root.join("platform/web")).arg("serve");
            let out = invoker.run(&inv)?;
            if out.status != 0 { bail!("trunk serve failed:\n{}", out.stderr); }
        }
        "ios" => {
            println!("Open platform/ios/{}-ios.xcodeproj in Xcode.", cfg.project.name);
        }
        "android" => {
            println!("Open platform/android/ in Android Studio.");
        }
        other => bail!("unknown platform: {}", other),
    }
    Ok(())
}
