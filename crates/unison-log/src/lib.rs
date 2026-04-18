//! Unified cross-platform logger for the Unison 2D engine.
//!
//! Routes `log::{info,warn,error,debug}!` macros to platform-appropriate
//! sinks (web console / logcat / stderr) with per-target filtering.
//!
//! Call [`init`] exactly once per process at the platform entry point.
//! Re-exports `log` macros so consumers don't need a separate `log` dep.

pub use log::{debug, error, info, trace, warn};
pub use log::{Level, LevelFilter};

pub mod filter;

#[cfg(target_arch = "wasm32")]
mod sink_web;
#[cfg(target_os = "android")]
mod sink_android;
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
mod sink_stderr;

use filter::Filter;
use std::sync::{OnceLock, RwLock};

#[cfg(target_arch = "wasm32")]
type ActiveSink = sink_web::WebSink;
#[cfg(target_os = "android")]
type ActiveSink = sink_android::AndroidSink;
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
type ActiveSink = sink_stderr::StderrSink;

static SINK: OnceLock<&'static ActiveSink> = OnceLock::new();

fn default_filter_spec() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "info"
    }
}

/// Initialize the global logger. Idempotent — safe to call multiple times.
pub fn init() {
    init_with_spec(default_filter_spec());
}

/// Initialize with an explicit filter spec (used by build-script-generated callers).
pub fn init_with_spec(spec: &str) {
    if SINK.get().is_some() {
        return;
    }
    let filter = Filter::parse(spec);
    let max = filter.max_level();

    #[cfg(target_arch = "wasm32")]
    let sink: &'static ActiveSink = Box::leak(Box::new(sink_web::WebSink {
        filter: RwLock::new(filter),
    }));
    #[cfg(target_os = "android")]
    let sink: &'static ActiveSink = Box::leak(Box::new(sink_android::AndroidSink::new(filter)));
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    let sink: &'static ActiveSink = Box::leak(Box::new(sink_stderr::StderrSink {
        filter: RwLock::new(filter),
    }));

    let _ = log::set_logger(sink);
    log::set_max_level(max);
    let _ = SINK.set(sink);
}

/// Replace the active filter at runtime. Silently no-ops if `init` hasn't been called.
pub fn set_filter(spec: &str) {
    if let Some(sink) = SINK.get() {
        if let Ok(mut guard) = sink.filter.write() {
            *guard = Filter::parse(spec);
            log::set_max_level(guard.max_level());
        }
    }
}
