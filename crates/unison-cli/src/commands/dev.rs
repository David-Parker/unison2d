use anyhow::{bail, Result};
use std::path::Path;

use crate::config::{Config, Lang};
use crate::toolchain::{Invocation, Invoker, SystemInvoker};

pub struct DevArgs {
    pub platform: String,
    pub release: bool,
    pub profile: bool,
}

pub fn run(project_root: &Path, args: DevArgs) -> Result<()> {
    let cfg = Config::load(&project_root.join("unison.toml"))?;
    let invoker = SystemInvoker;
    run_with(&cfg, project_root, &invoker, args)
}

pub fn run_with(cfg: &Config, project_root: &Path, invoker: &dyn Invoker, args: DevArgs) -> Result<()> {
    match args.platform.as_str() {
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
            let mut inv = Invocation::new("trunk", project_root.join("platform/web")).arg("serve");
            if args.release { inv = inv.arg("--release"); }
            if args.profile { inv = inv.arg("--features").arg("profiling"); }
            let out = invoker.run(&inv.streaming())?;
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
