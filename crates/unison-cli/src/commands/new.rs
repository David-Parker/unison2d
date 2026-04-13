use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{Build, Config, Engine, Lang, Platforms, Project};
use crate::render::render;
use crate::templates;

pub struct NewArgs {
    pub name: String,
    pub lang: String,
    pub no_web: bool,
    pub no_ios: bool,
    pub no_android: bool,
    pub no_git: bool,
    pub bundle_id: Option<String>,
    pub engine_tag: Option<String>,
    pub template: Option<String>,
}

pub fn run(args: NewArgs, engine_tag_default: &str, engine_git_url: &str) -> Result<()> {
    if args.template.is_some() {
        bail!("--template is not yet implemented");
    }
    validate_name(&args.name)?;
    let lang = parse_lang(&args.lang)?;
    let dest = PathBuf::from(&args.name);
    if dest.exists() {
        bail!("target directory {} already exists", dest.display());
    }

    let platforms = Platforms {
        web: !args.no_web,
        ios: !args.no_ios,
        android: !args.no_android,
    };
    if !platforms.web && !platforms.ios && !platforms.android {
        bail!("cannot disable all platforms (use at least one)");
    }

    let crate_name = args.name.replace('-', "_");
    let bundle_id = args.bundle_id.unwrap_or_else(|| format!("com.example.{}", crate_name));
    let engine_tag = args.engine_tag.unwrap_or_else(|| engine_tag_default.to_string());

    let vars: HashMap<&str, &str> = [
        ("PROJECT_NAME", args.name.as_str()),
        ("CRATE_NAME", crate_name.as_str()),
        ("BUNDLE_ID", bundle_id.as_str()),
        ("ANDROID_APP_ID", bundle_id.as_str()),
        ("KOTLIN_PACKAGE", bundle_id.as_str()),
        ("IOS_MODULE", crate_name.as_str()),
        ("ENGINE_TAG", engine_tag.as_str()),
        ("ENGINE_GIT_URL", engine_git_url),
    ].into_iter().collect();

    fs::create_dir_all(&dest)
        .with_context(|| format!("creating {}", dest.display()))?;

    render_dir(&templates::COMMON, &dest, &vars)?;
    match lang {
        Lang::Lua => render_dir(&templates::SCRIPTING_LUA, &dest, &vars)?,
        Lang::Ts => {
            render_dir_to(&templates::SCRIPTING_TS, &dest, &vars)?;
            // Append TS-specific .gitignore rules to the common .gitignore.
            if let Some(addon) = templates::SCRIPTING_TS.get_file(".gitignore-ts-addon") {
                let current = fs::read_to_string(dest.join(".gitignore")).unwrap_or_default();
                let extra = addon.contents_utf8().unwrap_or("");
                fs::write(
                    dest.join(".gitignore"),
                    format!("{}\n{}\n", current.trim_end(), extra.trim()),
                )?;
                // Remove the addon file from the scaffolded output.
                let _ = fs::remove_file(dest.join(".gitignore-ts-addon"));
            }
        }
    }
    if platforms.web {
        render_dir_to(&templates::PLATFORM_WEB, &dest.join("platform/web"), &vars)?;
    }
    if platforms.ios {
        render_dir_to(&templates::PLATFORM_IOS, &dest.join("platform/ios"), &vars)?;
    }
    if platforms.android {
        render_dir_to(&templates::PLATFORM_ANDROID, &dest.join("platform/android"), &vars)?;
        // Make build-rust.sh executable.
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            let p = dest.join("platform/android/build-rust.sh");
            if let Ok(md) = fs::metadata(&p) {
                let mut perm = md.permissions();
                perm.set_mode(0o755);
                let _ = fs::set_permissions(&p, perm);
            }
        }
    }

    // Write unison.toml
    let cfg = Config {
        project: Project { name: args.name.clone(), lang },
        engine: Engine {
            git: engine_git_url.to_string(),
            tag: Some(engine_tag.clone()),
            branch: None,
            rev: None,
            link_path: None,
        },
        platforms,
        build: Build::default(),
    };
    cfg.save(&dest.join("unison.toml"))?;

    if !args.no_git {
        let _ = std::process::Command::new("git")
            .arg("init")
            .arg("--initial-branch=main")
            .current_dir(&dest)
            .status();
    }

    println!("Created {}. Next: `cd {} && unison doctor && unison dev web`", dest.display(), dest.display());
    Ok(())
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("project name must not be empty");
    }
    let first = name.chars().next().unwrap();
    if !first.is_ascii_alphabetic() {
        bail!("project name must start with an ASCII letter");
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        bail!("project name may only contain letters, digits, '-', '_'");
    }
    Ok(())
}

fn parse_lang(s: &str) -> Result<Lang> {
    match s {
        "lua" => Ok(Lang::Lua),
        "ts" => Ok(Lang::Ts),
        other => bail!("unknown --lang {} (expected 'lua' or 'ts')", other),
    }
}

fn render_dir(dir: &include_dir::Dir<'_>, dest: &Path, vars: &HashMap<&str, &str>) -> Result<()> {
    render_dir_to(dir, dest, vars)
}

fn render_path_component(s: &str, vars: &HashMap<&str, &str>) -> String {
    // Filenames support substitution via literal substrings in path segments.
    // KOTLIN_PACKAGE_PATH expands to e.g. "com/example/my_game" (dots → slashes),
    // which means one original component becomes multiple path components joined by '/'.
    let project_name = vars.get("PROJECT_NAME").copied().unwrap_or("");
    let kotlin_pkg = vars.get("KOTLIN_PACKAGE").copied().unwrap_or("");
    let kotlin_path = kotlin_pkg.replace('.', "/");
    s.replace("PROJECT_NAME", project_name)
     .replace("KOTLIN_PACKAGE_PATH", &kotlin_path)
}

pub fn render_dir_to(dir: &include_dir::Dir<'_>, dest: &Path, vars: &HashMap<&str, &str>) -> Result<()> {
    for entry in dir.entries() {
        match entry {
            include_dir::DirEntry::File(f) => {
                let rel = f.path();
                // Apply PROJECT_NAME substitution to each path component
                let rendered_rel: std::path::PathBuf = rel
                    .components()
                    .map(|c| render_path_component(c.as_os_str().to_str().unwrap_or(""), vars))
                    .collect();
                let target = dest.join(&rendered_rel);
                if let Some(p) = target.parent() {
                    fs::create_dir_all(p)?;
                }
                let content = f.contents_utf8()
                    .ok_or_else(|| anyhow::anyhow!("non-UTF8 template file: {}", rel.display()))?;
                let rendered = render(content, vars)
                    .with_context(|| format!("rendering {}", rel.display()))?;
                fs::write(&target, rendered)
                    .with_context(|| format!("writing {}", target.display()))?;
            }
            include_dir::DirEntry::Dir(d) => {
                let sub_dest = dest.join(d.path());
                fs::create_dir_all(&sub_dest)?;
                render_dir_to(d, dest, vars)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_name() {
        assert!(validate_name("").is_err());
    }
    #[test]
    fn rejects_leading_digit() {
        assert!(validate_name("1game").is_err());
    }
    #[test]
    fn rejects_spaces() {
        assert!(validate_name("my game").is_err());
    }
    #[test]
    fn accepts_dashed_name() {
        validate_name("my-game").unwrap();
    }
    #[test]
    fn accepts_underscored_name() {
        validate_name("my_game").unwrap();
    }
}
