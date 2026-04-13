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
            // Held across the trunk serve lifetime; dropped (killed) when we return.
            let _tstl_watcher = if matches!(cfg.project.lang, Lang::Ts) {
                let inv = Invocation::new("npx", project_root)
                    .arg("tstl").arg("-p").arg("project/scripts-src/tsconfig.json").arg("--watch")
                    .streaming();
                Some(invoker.spawn(&inv)?)
            } else {
                None
            };
            let inv = Invocation::new("trunk", project_root.join("platform/web")).arg("serve").streaming();
            let out = invoker.run(&inv)?;
            if out.status != 0 { bail!("trunk serve failed (exit {}) — see output above", out.status); }
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
