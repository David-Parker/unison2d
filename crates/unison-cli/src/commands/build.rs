use anyhow::{bail, Result};
use std::path::Path;

use crate::config::{Config, Lang};
use crate::platforms;
use crate::toolchain::{Invoker, SystemInvoker};

pub struct BuildArgs {
    pub platform: String,
    pub release: bool,
    pub profile: bool,
}

pub fn run(project_root: &Path, args: BuildArgs) -> Result<()> {
    let cfg = Config::load(&project_root.join("unison.toml"))?;
    let invoker = SystemInvoker;
    run_with(&cfg, project_root, &invoker, args)
}

pub fn run_with(cfg: &Config, project_root: &Path, invoker: &dyn Invoker, args: BuildArgs) -> Result<()> {
    let run_tstl = matches!(cfg.project.lang, Lang::Ts);
    match args.platform.as_str() {
        "web" => {
            if !cfg.platforms.web { bail!("web is not enabled in unison.toml"); }
            platforms::web::build(project_root, invoker, platforms::web::WebBuildArgs {
                release: args.release, profile: args.profile,
            }, run_tstl)?;
        }
        "ios" | "android" => bail!("{} build — Task 13/14", args.platform),
        "all" => bail!("all — Task 15"),
        other => bail!("unknown platform: {}", other),
    }
    Ok(())
}
