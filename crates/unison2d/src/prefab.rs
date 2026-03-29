//! Prefab — a lightweight trait for consistent object spawning.
//!
//! Implement [`Prefab`] for reusable spawn templates:
//!
//! ```ignore
//! struct DonutPrefab {
//!     mesh: Mesh,
//!     material: Material,
//!     color: Color,
//! }
//!
//! impl Prefab for DonutPrefab {
//!     fn spawn(&self, world: &mut World, position: Vec2) -> ObjectId {
//!         world.objects.spawn_soft_body(SoftBodyDesc {
//!             mesh: self.mesh.clone(),
//!             material: self.material,
//!             position,
//!             color: self.color,
//!         })
//!     }
//! }
//! ```

use unison_core::Vec2;

use crate::object::ObjectId;
use crate::World;

/// A reusable object spawning template.
///
/// Each prefab knows how to spawn a fully-configured object at a given position.
/// Prefabs are stateless descriptions — they don't own the spawned object.
pub trait Prefab {
    /// Spawn this prefab's object into the world at the given position.
    fn spawn(&self, world: &mut World, position: Vec2) -> ObjectId;
}
