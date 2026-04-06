//! Hot reload support for Lua scripts during development.
//!
//! # Two-level hot reload architecture
//!
//! Hot reload means re-executing Lua source at runtime without restarting the process.
//! Two levels are available:
//!
//! - **Level 2 (default) — VM-preserving rebind:** Re-execute the new script source
//!   inside the *existing* Lua VM. The `__game` table is replaced with the freshly
//!   returned one, so `update` and `render` immediately pick up new definitions.
//!   World objects, physics state, event subscriptions, and all other Lua globals
//!   created during `init()` are preserved.
//!
//! - **Level 1 (fallback) — Full VM restart:** If Level 2 fails (e.g. the script's
//!   top-level structure changed in a way that breaks re-evaluation), tear down the
//!   Lua VM entirely, create a fresh one, re-register all bindings, re-execute the
//!   script, and call `init()` again. World state is lost.
//!
//! # Platform strategy
//!
//! - **Native debug builds:** Use [`ScriptWatcher`] to poll the filesystem each frame.
//!   Call `watcher.check()` and pass the result to `ScriptedGame::reload()` when
//!   `Some(source)` is returned.
//!
//! - **Web (WASM):** The Trunk dev server already watches all files in the project
//!   directory — including `.lua` files — and triggers a full page reload whenever
//!   they change. Hot reload is therefore handled at the tooling level on web, and
//!   no in-process file watching is required. `ScriptWatcher` is not compiled for
//!   `target_arch = "wasm32"`.
//!
//! - **Release builds:** `ScriptedGame::reload()` is a no-op. `ScriptWatcher` is
//!   not compiled.
//!
//! # Wiring example (native platform layer)
//!
//! ```rust,ignore
//! use unison_scripting::hot_reload::ScriptWatcher;
//!
//! // In your platform main:
//! let mut watcher = ScriptWatcher::new("project/assets/scripts/main.lua");
//!
//! // Each frame, before game.update():
//! if let Some(new_src) = watcher.check() {
//!     game.reload(&new_src);
//! }
//! ```

// ----------------------------------------------------------------------------
// ScriptWatcher — filesystem polling (debug native only)
// ----------------------------------------------------------------------------

/// Polls a filesystem path for script changes (debug native builds only).
///
/// Call [`check`](ScriptWatcher::check) each frame; it returns `Some(new_source)`
/// when the file has changed on disk since the last successful read.
///
/// `ScriptWatcher` is intentionally *not* wired into `ScriptedGame` automatically —
/// the game or platform layer creates and owns the watcher and calls
/// [`ScriptedGame::reload`] when a change is detected. This keeps the watcher
/// out of production builds and lets each platform choose its own strategy.
#[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
pub struct ScriptWatcher {
    path: std::path::PathBuf,
    last_modified: Option<std::time::SystemTime>,
}

#[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
impl ScriptWatcher {
    /// Create a new watcher for the given path.
    ///
    /// The watcher starts with no recorded modification time, so the *first*
    /// call to [`check`](ScriptWatcher::check) will always return the file
    /// contents (treating it as "changed from nothing").
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            path: path.into(),
            last_modified: None,
        }
    }

    /// Returns `Some(source)` if the file has changed since the last check,
    /// `None` if it is unchanged or cannot be read.
    ///
    /// On success the internal modification timestamp is updated so subsequent
    /// calls return `None` until another change occurs.
    pub fn check(&mut self) -> Option<String> {
        // Read the file's metadata to get the last modification time.
        let metadata = std::fs::metadata(&self.path).ok()?;
        let mtime = metadata.modified().ok()?;

        // Compare against the recorded timestamp.
        if let Some(last) = self.last_modified {
            if mtime <= last {
                return None;
            }
        }

        // File is new or changed — read its contents.
        let contents = std::fs::read_to_string(&self.path).ok()?;

        // Update the recorded timestamp only after a successful read.
        self.last_modified = Some(mtime);

        Some(contents)
    }
}
