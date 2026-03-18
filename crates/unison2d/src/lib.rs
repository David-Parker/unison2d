/// Unison 2D Game Engine
///
/// A modular 2D game engine built from independent subsystems:
/// - `unison-physics`: XPBD soft body physics simulation
/// - `unison-render`: Platform-agnostic rendering abstractions
/// - `unison-lighting`: 2D dynamic lighting with soft shadows
/// - `unison-profiler`: Lightweight function-level profiling

pub use unison_math as math;
pub use unison_physics as physics;
pub use unison_render as render;
pub use unison_lighting as lighting;
pub use unison_profiler as profiler;
