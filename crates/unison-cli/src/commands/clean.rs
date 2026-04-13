use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::config::{Config, Lang};

pub fn run(project_root: &Path) -> Result<()> {
    let cfg = Config::load(&project_root.join("unison.toml"))?;
    let mut to_remove: Vec<&Path> = Vec::new();
    let t1 = project_root.join("target");
    let t2 = project_root.join("platform/web/dist");
    let t3 = project_root.join("platform/android/app/build");
    let t4 = project_root.join("project/assets/scripts");
    to_remove.push(&t1);
    to_remove.push(&t2);
    to_remove.push(&t3);
    if matches!(cfg.project.lang, Lang::Ts) { to_remove.push(&t4); }

    for p in to_remove {
        if p.exists() {
            fs::remove_dir_all(p)?;
        }
    }
    Ok(())
}
