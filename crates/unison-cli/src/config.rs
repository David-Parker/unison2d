use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub project: Project,
    pub engine: Engine,
    pub platforms: Platforms,
    #[serde(default)]
    pub build: Build,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub name: String,
    pub lang: Lang,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Lua,
    Ts,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Engine {
    pub git: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Platforms {
    #[serde(default)]
    pub web: bool,
    #[serde(default)]
    pub ios: bool,
    #[serde(default)]
    pub android: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Build {
    pub release: bool,
    pub profile: bool,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        let cfg: Config = toml::from_str(&text)
            .with_context(|| format!("parsing {}", path.display()))?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<()> {
        if self.project.name.trim().is_empty() {
            bail!("project.name must not be empty");
        }
        let refs = [&self.engine.tag, &self.engine.branch, &self.engine.rev];
        let set_count = refs.iter().filter(|r| r.is_some()).count();
        if set_count != 1 {
            bail!("engine must set exactly one of `tag`, `branch`, or `rev` (found {})", set_count);
        }
        if !self.platforms.web && !self.platforms.ios && !self.platforms.android {
            bail!("at least one platform must be enabled in [platforms]");
        }
        Ok(())
    }

    /// Write config to `path` as a canonical TOML file (no comment preservation).
    /// To update an existing file while preserving comments, use [`Self::edit_in_place`].
    pub fn save(&self, path: &Path) -> Result<()> {
        // For first-time writes, emit via serde → toml.
        let out = toml::to_string_pretty(self)
            .with_context(|| "serializing unison.toml")?;
        fs::write(path, out)
            .with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    /// Edit-in-place: reads path as toml_edit Document, applies `f`, writes back.
    /// Preserves comments and unknown fields.
    pub fn edit_in_place<F>(path: &Path, f: F) -> Result<()>
    where
        F: FnOnce(&mut toml_edit::DocumentMut) -> Result<()>,
    {
        let text = fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        let mut doc: toml_edit::DocumentMut = text.parse()
            .with_context(|| format!("parsing {} as TOML", path.display()))?;
        f(&mut doc)?;
        fs::write(path, doc.to_string())
            .with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn write(content: &str) -> NamedTempFile {
        let f = NamedTempFile::new().unwrap();
        fs::write(f.path(), content).unwrap();
        f
    }

    #[test]
    fn loads_minimal_valid_config() {
        let f = write(r#"
[project]
name = "my-game"
lang = "lua"

[engine]
git = "https://github.com/x/unison2d"
tag = "v0.1.0"

[platforms]
web = true
"#);
        let cfg = Config::load(f.path()).unwrap();
        assert_eq!(cfg.project.name, "my-game");
        assert_eq!(cfg.project.lang, Lang::Lua);
        assert!(cfg.platforms.web);
        assert!(!cfg.platforms.ios);
    }

    #[test]
    fn rejects_no_platforms() {
        let f = write(r#"
[project]
name = "x"
lang = "lua"
[engine]
git = "x"
tag = "v0"
[platforms]
"#);
        assert!(Config::load(f.path()).is_err());
    }

    #[test]
    fn rejects_two_refs_set() {
        let f = write(r#"
[project]
name = "x"
lang = "lua"
[engine]
git = "x"
tag = "v0"
branch = "main"
[platforms]
web = true
"#);
        assert!(Config::load(f.path()).is_err());
    }

    #[test]
    fn rejects_no_ref_set() {
        let f = write(r#"
[project]
name = "x"
lang = "lua"
[engine]
git = "x"
[platforms]
web = true
"#);
        assert!(Config::load(f.path()).is_err());
    }

    #[test]
    fn edit_in_place_preserves_comments() {
        let f = write(r#"# header comment
[project]
name = "x"  # inline
lang = "lua"

[engine]
git = "x"
tag = "v0"

[platforms]
web = true
"#);
        Config::edit_in_place(f.path(), |doc| {
            doc["platforms"]["ios"] = toml_edit::value(true);
            Ok(())
        }).unwrap();
        let after = fs::read_to_string(f.path()).unwrap();
        assert!(after.contains("# header comment"));
        assert!(after.contains("# inline"));
        assert!(after.contains("ios = true"));
    }

    #[test]
    fn rejects_empty_name() {
        let f = write(r#"
[project]
name = ""
lang = "lua"
[engine]
git = "x"
tag = "v0"
[platforms]
web = true
"#);
        assert!(Config::load(f.path()).is_err());
    }

    #[test]
    fn save_then_load_roundtrips_fields() {
        let cfg = Config {
            project: Project { name: "x".into(), lang: Lang::Lua },
            engine: Engine {
                git: "u".into(),
                tag: Some("v0.1.0".into()),
                branch: None,
                rev: None,
                link_path: None,
            },
            platforms: Platforms { web: true, ios: false, android: false },
            build: Build::default(),
        };
        let f = NamedTempFile::new().unwrap();
        cfg.save(f.path()).unwrap();
        let loaded = Config::load(f.path()).unwrap();
        assert_eq!(cfg, loaded);
    }
}
