//! Unified cross-platform logger for the Unison 2D engine.
//!
//! Routes `log::{info,warn,error,debug}!` macros to platform-appropriate
//! sinks (web console / logcat / stderr) with per-target filtering.
//!
//! Call [`init`] exactly once per process at the platform entry point.
//! Re-exports the `log` macros so crates depending on `unison-log` don't
//! need a separate `log` dep.

pub use log::{debug, error, info, trace, warn};
pub use log::{Level, LevelFilter};

pub mod filter;

/// Initialize the global logger. Idempotent — safe to call multiple times.
pub fn init() {
    // Filled in by later tasks.
}

/// Replace the active filter at runtime.
pub fn set_filter(spec: &str) {
    let _ = spec;
}
