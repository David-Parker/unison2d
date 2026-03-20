//! Object management — the registry of game objects tracked by Engine.

use unison_math::{Color, Vec2};
use unison_physics::{BodyConfig, BodyHandle, Collider, Material, Mesh, RigidBodyConfig};
use unison_render::TextureId;
use unison_lighting::LightHandle;

/// Unique identifier for a game object managed by the Engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ObjectId(pub(crate) u64);

impl ObjectId {
    /// A placeholder ID for use in struct initialization.
    /// Will be replaced when you call `engine.spawn_*()`.
    pub const PLACEHOLDER: Self = Self(u64::MAX);
}

impl Default for ObjectId {
    fn default() -> Self {
        Self::PLACEHOLDER
    }
}

/// Description for spawning a soft body object.
///
/// ```ignore
/// engine.spawn_soft_body(SoftBodyDesc {
///     mesh: create_ring_mesh(1.0, 0.4, 16, 6),
///     material: Material::RUBBER,
///     position: Vec2::new(0.0, 5.0),
///     color: Color::WHITE,
/// });
/// ```
pub struct SoftBodyDesc {
    /// Mesh geometry for the soft body
    pub mesh: Mesh,
    /// Physics material (JELLO, RUBBER, WOOD, METAL, or custom)
    pub material: Material,
    /// Initial position in world coordinates
    pub position: Vec2,
    /// Render color
    pub color: Color,
}

/// Description for spawning a rigid body object.
///
/// ```ignore
/// engine.spawn_rigid_body(RigidBodyDesc {
///     collider: Collider::aabb(5.0, 0.5),
///     position: Vec2::new(0.0, -3.0),
///     color: Color::from_hex(0x2d5016),
///     is_static: true,
/// });
/// ```
pub struct RigidBodyDesc {
    /// Collider shape (Circle or AABB)
    pub collider: Collider,
    /// Initial position in world coordinates
    pub position: Vec2,
    /// Render color
    pub color: Color,
    /// If true, this body is not affected by physics (platforms, walls)
    pub is_static: bool,
}

/// Description for spawning a sprite-only object (no physics).
///
/// Sprites are purely visual — a textured or colored quad with a transform.
/// They are not affected by gravity, collisions, or forces.
pub struct SpriteDesc {
    /// Texture to render (`TextureId::NONE` for a solid color rect).
    pub texture: TextureId,
    /// Initial position in world coordinates.
    pub position: Vec2,
    /// Size (width, height) in world units.
    pub size: Vec2,
    /// Rotation in radians.
    pub rotation: f32,
    /// Render color / tint.
    pub color: Color,
}

/// Description for spawning a light object.
///
/// Wraps a [`unison_lighting::Light`]. The light is added to the World's
/// `LightingSystem` and tracked as an object so it can be queried by `ObjectId`.
pub struct LightDesc {
    /// The light configuration.
    pub light: unison_lighting::Light,
}

/// Internal representation of a game object in the Engine's registry.
pub(crate) enum ObjectKind {
    SoftBody {
        handle: BodyHandle,
        color: Color,
        uvs: Vec<f32>,
    },
    RigidBody {
        handle: BodyHandle,
        color: Color,
    },
    Sprite {
        texture: TextureId,
        position: Vec2,
        size: Vec2,
        rotation: f32,
        color: Color,
    },
    Light {
        light_handle: LightHandle,
    },
}

pub(crate) struct ObjectEntry {
    pub(crate) kind: ObjectKind,
}

impl ObjectEntry {
    /// Get the physics BodyHandle for this object, if it has one.
    ///
    /// Returns `None` for Sprite and Light objects (no physics backing).
    pub(crate) fn physics_handle(&self) -> Option<BodyHandle> {
        match &self.kind {
            ObjectKind::SoftBody { handle, .. } => Some(*handle),
            ObjectKind::RigidBody { handle, .. } => Some(*handle),
            ObjectKind::Sprite { .. } | ObjectKind::Light { .. } => None,
        }
    }
}

impl SoftBodyDesc {
    /// Convert to physics BodyConfig for adding to PhysicsWorld
    pub(crate) fn to_body_config(&self) -> BodyConfig {
        BodyConfig::new()
            .with_material(self.material)
            .at_position(self.position.x, self.position.y)
    }
}

impl RigidBodyDesc {
    /// Convert to physics RigidBodyConfig for adding to PhysicsWorld
    pub(crate) fn to_rigid_body_config(&self) -> RigidBodyConfig {
        let mut config = RigidBodyConfig::new()
            .with_collider(self.collider.clone())
            .at_position(self.position.x, self.position.y);
        if self.is_static {
            config = config.as_kinematic();
        }
        config
    }
}
