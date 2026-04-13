use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::commands::new::{chmod_android_scripts, render_dir_to};
use crate::config::Config;
use crate::templates;

pub fn add(project_root: &Path, platform: &str) -> Result<()> {
    let cfg = Config::load(&project_root.join("unison.toml"))?;
    let dir = project_root.join(format!("platform/{}", platform));
    if dir.exists() { bail!("{} already exists", dir.display()); }

    let crate_name = cfg.project.name.replace('-', "_");
    let bundle_id = format!("com.example.{}", crate_name);
    let engine_tag = cfg.engine.tag.clone().unwrap_or_default();
    let engine_version = engine_tag.strip_prefix('v').unwrap_or(&engine_tag).to_string();
    let vars: HashMap<&str, &str> = [
        ("PROJECT_NAME", cfg.project.name.as_str()),
        ("CRATE_NAME", crate_name.as_str()),
        ("BUNDLE_ID", bundle_id.as_str()),
        ("ANDROID_APP_ID", bundle_id.as_str()),
        ("KOTLIN_PACKAGE", bundle_id.as_str()),
        ("IOS_MODULE", crate_name.as_str()),
        ("ENGINE_TAG", engine_tag.as_str()),
        ("ENGINE_VERSION", engine_version.as_str()),
        ("ENGINE_GIT_URL", cfg.engine.git.as_str()),
    ].into_iter().collect();

    match platform {
        "web" => render_dir_to(&templates::PLATFORM_WEB, &project_root.join("platform/web"), &vars)?,
        "ios" => render_dir_to(&templates::PLATFORM_IOS, &project_root.join("platform/ios"), &vars)?,
        "android" => {
            let android_dir = project_root.join("platform/android");
            render_dir_to(&templates::PLATFORM_ANDROID, &android_dir, &vars)?;
            chmod_android_scripts(&android_dir);
        }
        other => bail!("unknown platform: {}", other),
    }

    let platform_owned = platform.to_string();
    Config::edit_in_place(&project_root.join("unison.toml"), |doc| {
        doc["platforms"][&platform_owned] = toml_edit::value(true);
        Ok(())
    })?;
    Ok(())
}

pub fn remove(project_root: &Path, platform: &str) -> Result<()> {
    let cfg = Config::load(&project_root.join("unison.toml"))?;
    let enabled: Vec<&str> = [
        (cfg.platforms.web, "web"),
        (cfg.platforms.ios, "ios"),
        (cfg.platforms.android, "android"),
    ]
    .into_iter()
    .filter_map(|(en, n)| if en { Some(n) } else { None })
    .collect();
    if enabled.as_slice() == [platform] {
        bail!("cannot remove {} — it's the only enabled platform", platform);
    }
    let dir = project_root.join(format!("platform/{}", platform));
    if dir.exists() {
        fs::remove_dir_all(&dir)
            .with_context(|| format!("removing {}", dir.display()))?;
    }
    let platform_owned = platform.to_string();
    Config::edit_in_place(&project_root.join("unison.toml"), |doc| {
        doc["platforms"][&platform_owned] = toml_edit::value(false);
        Ok(())
    })?;
    Ok(())
}
