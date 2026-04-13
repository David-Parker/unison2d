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
    // Run tstl once if the project uses TypeScript. Individual platform builders no
    // longer need to re-run it.
    if matches!(cfg.project.lang, Lang::Ts) {
        platforms::run_tstl(project_root, invoker)?;
    }
    match args.platform.as_str() {
        "web" => {
            if !cfg.platforms.web { bail!("web is not enabled in unison.toml"); }
            platforms::web::build(project_root, invoker, platforms::web::WebBuildArgs {
                release: args.release, profile: args.profile,
            })?;
        }
        "ios" => {
            if !cfg.platforms.ios { bail!("ios is not enabled in unison.toml"); }
            platforms::ios::build(project_root, invoker, platforms::ios::IosBuildArgs {
                release: args.release, profile: args.profile,
                project_name: cfg.project.name.clone(),
            })?;
        }
        "android" => {
            if !cfg.platforms.android { bail!("android is not enabled in unison.toml"); }
            platforms::android::build(project_root, invoker, platforms::android::AndroidBuildArgs {
                release: args.release, profile: args.profile,
            })?;
        }
        "all" => {
            if cfg.platforms.web {
                platforms::web::build(project_root, invoker, platforms::web::WebBuildArgs {
                    release: args.release, profile: args.profile,
                })?;
            }
            if cfg.platforms.ios {
                platforms::ios::build(project_root, invoker, platforms::ios::IosBuildArgs {
                    release: args.release, profile: args.profile,
                    project_name: cfg.project.name.clone(),
                })?;
            }
            if cfg.platforms.android {
                platforms::android::build(project_root, invoker, platforms::android::AndroidBuildArgs {
                    release: args.release, profile: args.profile,
                })?;
            }
        }
        other => bail!("unknown platform: {}", other),
    }
    Ok(())
}
