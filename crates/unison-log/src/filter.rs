//! Filter parsing and longest-prefix target matching.
//!
//! Grammar: `level[,target=level]*`
//!
//! The first token is the default level applied to all targets that have
//! no explicit override. Subsequent comma-separated tokens are
//! `target=level` pairs. Unknown levels fall back to `Info`.

use log::{Level, LevelFilter};

#[derive(Debug, Clone)]
pub struct Filter {
    default: LevelFilter,
    /// Sorted by prefix length descending so longest-prefix wins.
    overrides: Vec<(String, LevelFilter)>,
}

impl Filter {
    pub fn parse(spec: &str) -> Self {
        let mut default = LevelFilter::Info;
        let mut overrides: Vec<(String, LevelFilter)> = Vec::new();

        for (i, raw) in spec.split(',').enumerate() {
            let tok = raw.trim();
            if tok.is_empty() {
                continue;
            }

            if let Some((target, lvl)) = tok.split_once('=') {
                let target = target.trim();
                let lvl = lvl.trim();
                if target.is_empty() {
                    continue;
                }
                if lvl.contains('=') {
                    continue;
                }
                let parsed = parse_level_filter(lvl).unwrap_or(LevelFilter::Info);
                overrides.push((target.to_string(), parsed));
            } else if i == 0 {
                default = parse_level_filter(tok).unwrap_or(LevelFilter::Info);
            }
        }

        overrides.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

        Self { default, overrides }
    }

    /// Returns `true` if a log at `level` targeting `target` should pass.
    pub fn enabled(&self, target: &str, level: Level) -> bool {
        let effective = self
            .overrides
            .iter()
            .find(|(prefix, _)| {
                target == prefix.as_str() || target.starts_with(&format!("{prefix}::"))
            })
            .map(|(_, lf)| *lf)
            .unwrap_or(self.default);
        level.to_level_filter() <= effective
    }

    pub fn max_level(&self) -> LevelFilter {
        self.overrides
            .iter()
            .map(|(_, lf)| *lf)
            .chain(std::iter::once(self.default))
            .max()
            .unwrap_or(LevelFilter::Info)
    }
}

impl Default for Filter {
    fn default() -> Self {
        Self {
            default: LevelFilter::Info,
            overrides: Vec::new(),
        }
    }
}

fn parse_level_filter(s: &str) -> Option<LevelFilter> {
    match s.to_ascii_lowercase().as_str() {
        "off" => Some(LevelFilter::Off),
        "error" => Some(LevelFilter::Error),
        "warn" => Some(LevelFilter::Warn),
        "info" => Some(LevelFilter::Info),
        "debug" => Some(LevelFilter::Debug),
        "trace" => Some(LevelFilter::Trace),
        _ => None,
    }
}
