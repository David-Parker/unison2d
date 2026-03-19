//! Unison Profiler - Lightweight performance profiling for games
//!
//! Provides automatic function-level profiling with minimal code changes.
//!
//! # Usage
//!
//! ## Automatic profiling with the `#[profile]` attribute
//!
//! ```ignore
//! use unison_profiler::profile;
//!
//! #[profile]
//! fn expensive_function() {
//!     // This function is automatically profiled
//! }
//!
//! impl MyStruct {
//!     #[profile]
//!     fn my_method(&mut self) {
//!         // Methods can be profiled too
//!     }
//! }
//! ```
//!
//! ## Manual profiling for fine-grained control
//!
//! ```ignore
//! use unison_profiler::{profile_begin, profile_end, Profiler};
//!
//! fn update(&mut self) {
//!     profile_begin!("physics");
//!     self.physics.step(dt);
//!     profile_end!();
//! }
//!
//! // Log stats periodically
//! if frame_count % 60 == 0 {
//!     println!("{}", Profiler::format_stats());
//!     Profiler::reset();
//! }
//! ```
//!
//! ## Setup (once at startup)
//!
//! ```ignore
//! use unison_profiler::{set_time_fn, Profiler};
//!
//! // Set time function (platform-specific)
//! set_time_fn(|| performance.now()); // WASM
//! set_time_fn(|| Instant::now().elapsed().as_secs_f64() * 1000.0); // Native
//!
//! // Enable profiling
//! Profiler::set_enabled(true);
//! ```

use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static PROFILER: RefCell<ProfilerState> = RefCell::new(ProfilerState::new());
    static TIME_FN: RefCell<Option<fn() -> f64>> = const { RefCell::new(None) };
}

/// Set the time function used for profiling
/// The function should return current time in milliseconds
pub fn set_time_fn(f: fn() -> f64) {
    TIME_FN.with(|tf| {
        *tf.borrow_mut() = Some(f);
    });
}

/// Get current time using the registered time function
#[inline]
pub fn now() -> f64 {
    TIME_FN.with(|tf| {
        tf.borrow().map(|f| f()).unwrap_or(0.0)
    })
}

/// Internal profiler state
struct ProfilerState {
    /// Current frame's scope timings (full_path -> accumulated ms)
    current_frame: HashMap<String, f64>,
    /// Historical data for averaging (full_path -> (total_ms, count, depth))
    history: HashMap<String, (f64, u32, usize)>,
    /// Stack of active scopes for nested timing (name, start_time)
    scope_stack: Vec<(String, f64)>,
    /// Frame count
    frame_count: u64,
    /// Whether profiling is enabled
    enabled: bool,
    /// Total frame time accumulator (for utilization calculation)
    total_frame_time: f64,
    /// Frame start time
    frame_start_time: f64,
    /// Target frame time in ms (e.g., 16.67 for 60 FPS)
    target_frame_time: f64,
}

impl ProfilerState {
    fn new() -> Self {
        Self {
            current_frame: HashMap::new(),
            history: HashMap::new(),
            scope_stack: Vec::new(),
            frame_count: 0,
            enabled: cfg!(feature = "enabled"),
            total_frame_time: 0.0,
            frame_start_time: 0.0,
            target_frame_time: 1000.0 / 60.0, // Default 60 FPS = 16.67ms
        }
    }
}

/// Main profiler interface
pub struct Profiler;

impl Profiler {
    /// Initialize the profiler (call once at startup)
    pub fn init() {
        PROFILER.with(|p| {
            let mut state = p.borrow_mut();
            state.current_frame.clear();
            state.history.clear();
            state.scope_stack.clear();
            state.frame_count = 0;
        });
    }

    /// Enable or disable profiling
    pub fn set_enabled(enabled: bool) {
        PROFILER.with(|p| {
            p.borrow_mut().enabled = enabled;
        });
    }

    /// Check if profiling is enabled
    pub fn is_enabled() -> bool {
        PROFILER.with(|p| p.borrow().enabled)
    }

    /// Set target frame rate (for percentage calculations)
    pub fn set_target_fps(fps: f64) {
        PROFILER.with(|p| {
            p.borrow_mut().target_frame_time = 1000.0 / fps;
        });
    }

    /// Get target frame time in ms
    pub fn target_frame_time() -> f64 {
        PROFILER.with(|p| p.borrow().target_frame_time)
    }

    /// Begin a named scope
    #[inline]
    pub fn begin_scope(name: &str, start_time: f64) {
        #[cfg(feature = "enabled")]
        PROFILER.with(|p| {
            let mut state = p.borrow_mut();
            if state.enabled {
                state.scope_stack.push((name.to_string(), start_time));
            }
        });
        #[cfg(not(feature = "enabled"))]
        let _ = (name, start_time);
    }

    /// End the current scope
    #[inline]
    pub fn end_scope(end_time: f64) {
        #[cfg(feature = "enabled")]
        PROFILER.with(|p| {
            let mut state = p.borrow_mut();
            if state.enabled {
                if let Some((name, start_time)) = state.scope_stack.pop() {
                    let duration = end_time - start_time;
                    let depth = state.scope_stack.len();

                    // Build full hierarchical path
                    let full_path = if state.scope_stack.is_empty() {
                        name
                    } else {
                        let parent_path: String = state.scope_stack
                            .iter()
                            .map(|(n, _)| n.as_str())
                            .collect::<Vec<_>>()
                            .join("/");
                        format!("{}/{}", parent_path, name)
                    };

                    // Store with depth for indentation
                    let entry = state.current_frame.entry(full_path.clone()).or_insert(0.0);
                    *entry += duration;

                    // Also store depth in a separate tracking (we'll merge later)
                    state.history.entry(full_path).or_insert((0.0, 0, depth));
                }
            }
        });
        #[cfg(not(feature = "enabled"))]
        let _ = end_time;
    }

    /// Begin a new frame (call at start of frame)
    pub fn begin_frame() {
        PROFILER.with(|p| {
            let mut state = p.borrow_mut();
            state.frame_start_time = now();
        });
    }

    /// End the current frame and accumulate statistics
    pub fn end_frame() {
        PROFILER.with(|p| {
            let mut state = p.borrow_mut();

            // Always track frame time for FPS reporting
            let frame_time = now() - state.frame_start_time;
            state.total_frame_time += frame_time;
            state.frame_count += 1;

            // Only accumulate scope data when profiling is enabled
            if !state.enabled {
                return;
            }

            // Drain into temp vec to avoid borrow issues
            let current: Vec<_> = state.current_frame.drain().collect();

            // Accumulate current frame data into history
            for (path, duration) in current {
                let depth = path.matches('/').count();
                let entry = state.history.entry(path).or_insert((0.0, 0, depth));
                entry.0 += duration;
                entry.1 += 1;
            }
        });
    }

    /// Get total frame time accumulated
    pub fn total_frame_time() -> f64 {
        PROFILER.with(|p| p.borrow().total_frame_time)
    }

    /// Get average frame time
    pub fn avg_frame_time() -> f64 {
        PROFILER.with(|p| {
            let state = p.borrow();
            if state.frame_count > 0 {
                state.total_frame_time / state.frame_count as f64
            } else {
                0.0
            }
        })
    }

    /// Get current frame count
    pub fn frame_count() -> u64 {
        PROFILER.with(|p| p.borrow().frame_count)
    }

    /// Get statistics for all scopes
    /// Returns Vec of (name, avg_ms, total_ms, call_count, depth)
    /// Sorted hierarchically with highest total% children first at each level
    pub fn get_stats() -> Vec<(String, f64, f64, u32, usize)> {
        PROFILER.with(|p| {
            let state = p.borrow();
            let stats: Vec<_> = state
                .history
                .iter()
                .map(|(path, (total, count, depth))| {
                    let avg = if *count > 0 { total / *count as f64 } else { 0.0 };
                    (path.clone(), avg, *total, *count, *depth)
                })
                .collect();

            // Build a map of path -> total for sorting
            let totals: HashMap<String, f64> = stats.iter()
                .map(|(path, _, total, _, _)| (path.clone(), *total))
                .collect();

            // Sort hierarchically: parents before children, siblings by total (descending)
            let mut sorted = stats;
            sorted.sort_by(|a, b| {
                let path_a = &a.0;
                let path_b = &b.0;

                // Check if one is ancestor of the other
                if path_b.starts_with(path_a) && path_b.len() > path_a.len() {
                    // a is ancestor of b, a comes first
                    return std::cmp::Ordering::Less;
                }
                if path_a.starts_with(path_b) && path_a.len() > path_b.len() {
                    // b is ancestor of a, b comes first
                    return std::cmp::Ordering::Greater;
                }

                // Find common ancestor and compare at divergence point
                let parts_a: Vec<&str> = path_a.split('/').collect();
                let parts_b: Vec<&str> = path_b.split('/').collect();

                // Find first differing index
                let mut common_len = 0;
                for i in 0..parts_a.len().min(parts_b.len()) {
                    if parts_a[i] == parts_b[i] {
                        common_len = i + 1;
                    } else {
                        break;
                    }
                }

                // Compare at the divergence point by total time (descending)
                if common_len < parts_a.len() && common_len < parts_b.len() {
                    // Both have children at this level - compare their totals
                    let prefix_a: String = parts_a[..=common_len].join("/");
                    let prefix_b: String = parts_b[..=common_len].join("/");
                    let total_a = totals.get(&prefix_a).unwrap_or(&0.0);
                    let total_b = totals.get(&prefix_b).unwrap_or(&0.0);
                    // Sort by total descending, then by name for stability
                    match total_b.partial_cmp(total_a) {
                        Some(std::cmp::Ordering::Equal) | None => prefix_a.cmp(&prefix_b),
                        Some(ord) => ord,
                    }
                } else {
                    // One is shorter - compare by total descending
                    let total_a = a.2;
                    let total_b = b.2;
                    match total_b.partial_cmp(&total_a) {
                        Some(std::cmp::Ordering::Equal) | None => path_a.cmp(path_b),
                        Some(ord) => ord,
                    }
                }
            });
            sorted
        })
    }

    /// Reset all statistics
    pub fn reset() {
        PROFILER.with(|p| {
            let mut state = p.borrow_mut();
            state.history.clear();
            state.frame_count = 0;
            state.total_frame_time = 0.0;
        });
    }

    /// Format stats as a string for logging
    pub fn format_stats() -> String {
        let stats = Self::get_stats();
        let frame_count = Self::frame_count();
        let avg_frame_time = Self::avg_frame_time();
        let target_frame_time = Self::target_frame_time();
        let target_fps = 1000.0 / target_frame_time;
        let actual_fps = if avg_frame_time > 0.0 { 1000.0 / avg_frame_time } else { 0.0 };

        if stats.is_empty() {
            return format!(
                "=== FPS ({} frames) === {:.0} FPS ({:.2}ms/frame)",
                frame_count, actual_fps, avg_frame_time
            );
        }

        // Total budget = frame_count * target_frame_time
        let total_budget = frame_count as f64 * target_frame_time;

        // Find root-level total time (sum of top-level scopes)
        let root_time: f64 = stats.iter()
            .filter(|(_, _, _, _, depth)| *depth == 0)
            .map(|(_, _, total, _, _)| total)
            .sum();
        let avg_active_time = if frame_count > 0 { root_time / frame_count as f64 } else { 0.0 };
        let budget_used = if total_budget > 0.0 { (root_time / total_budget) * 100.0 } else { 0.0 };

        // Build map of path -> total for calculating self time
        let totals: HashMap<String, f64> = stats.iter()
            .map(|(path, _, total, _, _)| (path.clone(), *total))
            .collect();

        // Calculate self time (total - sum of direct children)
        let mut self_times: HashMap<String, f64> = HashMap::new();
        for (path, _, total, _, _) in &stats {
            // Find direct children (paths that start with this path + "/" and have no additional "/")
            let children_time: f64 = stats.iter()
                .filter(|(child_path, _, _, _, _)| {
                    if let Some(rest) = child_path.strip_prefix(path) {
                        rest.starts_with('/') && !rest[1..].contains('/')
                    } else {
                        false
                    }
                })
                .map(|(_, _, child_total, _, _)| child_total)
                .sum();
            self_times.insert(path.clone(), total - children_time);
        }

        let mut output = format!(
            "=== Profiler Stats ({} frames) ===\n",
            frame_count
        );
        output.push_str(&format!(
            "Target: {:.0} FPS ({:.2}ms) | Actual: {:.0} FPS ({:.2}ms)\n",
            target_fps,
            target_frame_time,
            actual_fps,
            avg_frame_time
        ));
        output.push_str(&format!(
            "Budget: {:.2}ms/frame used ({:.1}%) | Headroom: {:.2}ms ({:.1}%)\n",
            avg_active_time,
            budget_used,
            target_frame_time - avg_active_time,
            100.0 - budget_used
        ));
        output.push_str("---------------------------------------------------------------------------\n");
        output.push_str("Scope                                   self%    total%   Calls\n");
        output.push_str("---------------------------------------------------------------------------\n");

        for (path, _avg_per_call, total, count, depth) in stats {
            // Get self time (excluding children)
            let self_time = self_times.get(&path).copied().unwrap_or(total);

            // Calculate percentages of TARGET frame budget (not actual frame time)
            let self_pct = if total_budget > 0.0 { (self_time / total_budget) * 100.0 } else { 0.0 };
            let total_pct = if total_budget > 0.0 { (total / total_budget) * 100.0 } else { 0.0 };

            // Skip scopes below 0.1% self time
            if self_pct < 0.1 && depth > 0 {
                continue;
            }

            // Root scopes show full path; children show only their leaf name, indented
            let display_name = if depth == 0 {
                path.clone()
            } else {
                let name = path.split('/').last().unwrap_or(&path);
                let indent = "  ".repeat(depth);
                format!("{}{}", indent, name)
            };

            // Truncate if too long
            let display_name = if display_name.len() > 36 {
                format!("{}...", &display_name[..33])
            } else {
                display_name
            };

            output.push_str(&format!(
                "{:<36} {:>6.1}%   {:>6.1}%   {:>5}\n",
                display_name,
                self_pct,
                total_pct,
                count,
            ));
        }

        output
    }
}

/// RAII scope guard for automatic timing
/// Created by profile_scope! macro
pub struct ProfileGuard {
    #[allow(dead_code)]
    name: &'static str,
}

impl ProfileGuard {
    #[inline]
    pub fn new(name: &'static str) -> Self {
        Profiler::begin_scope(name, now());
        Self { name }
    }
}

impl Drop for ProfileGuard {
    #[inline]
    fn drop(&mut self) {
        Profiler::end_scope(now());
    }
}

/// Profile a scope automatically using RAII
/// The scope is timed from creation until the guard goes out of scope
///
/// # Example
/// ```ignore
/// fn my_function() {
///     profile_scope!("my_function");
///     // ... work happens here ...
/// } // timing ends automatically when scope exits
/// ```
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        let _profile_guard = $crate::ProfileGuard::new($name);
    };
}

/// Begin a named profiling scope
/// Must be paired with profile_end!()
#[macro_export]
macro_rules! profile_begin {
    ($name:expr, $time:expr) => {
        $crate::Profiler::begin_scope($name, $time);
    };
    ($name:expr) => {
        $crate::Profiler::begin_scope($name, $crate::now());
    };
}

/// End the current profiling scope
#[macro_export]
macro_rules! profile_end {
    ($time:expr) => {
        $crate::Profiler::end_scope($time);
    };
    () => {
        $crate::Profiler::end_scope($crate::now());
    };
}

/// Profile a function by adding this attribute
///
/// This is a simple inline version. For a full proc-macro version,
/// use a separate proc-macro crate.
///
/// # Example
/// ```ignore
/// // For now, use profile_scope! at the start of functions:
/// fn expensive_function() {
///     profile_scope!("expensive_function");
///     // ... function body ...
/// }
/// ```

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_profiling() {
        Profiler::init();
        Profiler::set_enabled(true);

        // Simulate some scopes
        Profiler::begin_scope("test", 0.0);
        Profiler::end_scope(10.0);

        Profiler::begin_scope("test", 0.0);
        Profiler::end_scope(20.0);

        Profiler::end_frame();

        let stats = Profiler::get_stats();
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].0, "test");
        assert_eq!(stats[0].2, 30.0); // total
        assert_eq!(stats[0].3, 1);    // count (frames where scope appeared)
    }

    #[test]
    fn test_nested_scopes() {
        Profiler::init();
        Profiler::set_enabled(true);

        Profiler::begin_scope("outer", 0.0);
        Profiler::begin_scope("inner", 5.0);
        Profiler::end_scope(15.0); // inner: 10ms
        Profiler::end_scope(20.0); // outer: 20ms

        Profiler::end_frame();

        let stats = Profiler::get_stats();
        assert_eq!(stats.len(), 2);

        // Check hierarchical path
        let paths: Vec<_> = stats.iter().map(|s| s.0.as_str()).collect();
        assert!(paths.contains(&"outer"));
        assert!(paths.contains(&"outer/inner"));
    }
}
