# unison-profiler

Lightweight function-level profiling with hierarchical scoping. Zero dependencies. Feature-gated via `enabled` (on by default).

## Quick Start

```rust
use unison_profiler::{Profiler, set_time_fn, profile_scope};

// 1. Set platform time function (once at startup)
set_time_fn(|| performance.now()); // WASM example

// 2. Profile scopes
fn update() {
    profile_scope!("update");

    {
        profile_scope!("physics");
        // ... creates hierarchical path "update/physics"
    }
}

// 3. Frame loop
Profiler::begin_frame();
update();
Profiler::end_frame();

// 4. Report
println!("{}", Profiler::format_stats());
```

## Macros

```rust
// RAII guard — timing ends when scope exits (recommended)
profile_scope!("name");

// Manual begin/end
profile_begin!("name");
// ... work ...
profile_end!();

// With explicit timestamps
profile_begin!("name", start_time);
profile_end!(end_time);
```

## Profiler (static methods)

### Init & Config

```rust
Profiler::init();                    // call once at startup
Profiler::set_enabled(true);        // enable/disable at runtime
Profiler::is_enabled()              // -> bool
Profiler::set_target_fps(60.0);     // for budget calculations
Profiler::target_frame_time()      // -> f64 (ms), e.g. 16.67 for 60 FPS
```

### Frame Management

```rust
Profiler::begin_frame();
Profiler::end_frame();
```

### Scope Timing

```rust
Profiler::begin_scope("name", now());
Profiler::end_scope(now());
```

Scopes nest automatically — `begin_scope("a")` then `begin_scope("b")` creates path `"a/b"`.

### Statistics

```rust
Profiler::frame_count()       // -> u64
Profiler::total_frame_time()  // -> f64 (ms)
Profiler::avg_frame_time()    // -> f64 (ms, CPU time averaged over frames)
Profiler::avg_wall_time()     // -> f64 (ms, real display interval between frames)
Profiler::reset();

// Structured data
Profiler::get_stats()
    // -> Vec<(name, avg_ms, total_ms, call_count, depth)>

// Human-readable report
Profiler::format_stats() // -> String
```

### format_stats() Output Example

```
=== Profiler Stats (3600 frames) ===
Display: 60 Hz | Max: 62 FPS (16.13ms) | Target: 60 FPS (16.67ms)
Budget: 15.20ms/frame used (91.2%) | Headroom: 1.47ms (8.8%)
---------------------------------------------------------------------------
Scope                                   self%    total%   Calls
---------------------------------------------------------------------------
update                                   35.2%     78.5%    3600
  physics                                20.1%     42.1%    3600
    collision                            10.2%     20.5%    3600
```

## ProfileGuard

RAII guard created by `profile_scope!`. Can also be used directly:

```rust
let _guard = ProfileGuard::new("my_scope"); // begins scope
// ... scope ends when _guard is dropped
```

## Utility

```rust
set_time_fn(f: fn() -> f64)  // register platform clock (returns ms)
now() -> f64                  // read current time via registered clock
```
