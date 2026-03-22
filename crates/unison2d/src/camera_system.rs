//! CameraSystem — manages named cameras and their follow targets.
//!
//! A World has one CameraSystem holding all its cameras. Each camera can
//! optionally follow a game object with smoothing. A default "main" camera
//! is created automatically.

use std::collections::HashMap;

use unison_math::Vec2;
use unison_render::Camera;

use crate::object::ObjectId;
use crate::object_system::ObjectSystem;

/// The default camera name, created automatically.
const DEFAULT_CAMERA: &str = "main";

/// Follow-target entry: object to track, smoothing factor, and fixed offset.
struct FollowTarget {
    object: ObjectId,
    smoothing: f32,
    offset: Vec2,
}

/// Manages named cameras and their follow targets.
pub struct CameraSystem {
    cameras: HashMap<String, Camera>,
    follow_targets: HashMap<String, FollowTarget>,
}

impl CameraSystem {
    /// Create a new CameraSystem with a default "main" camera (20x15 world units).
    pub fn new() -> Self {
        let mut cameras = HashMap::new();
        cameras.insert(DEFAULT_CAMERA.to_string(), Camera::new(20.0, 15.0));

        Self {
            cameras,
            follow_targets: HashMap::new(),
        }
    }

    /// Add a named camera. Replaces any existing camera with the same name.
    pub fn add(&mut self, name: &str, camera: Camera) {
        self.cameras.insert(name.to_string(), camera);
    }

    /// Remove a named camera and its follow target.
    pub fn remove(&mut self, name: &str) {
        self.cameras.remove(name);
        self.follow_targets.remove(name);
    }

    /// Get a camera by name.
    pub fn get(&self, name: &str) -> Option<&Camera> {
        self.cameras.get(name)
    }

    /// Get a mutable camera by name.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Camera> {
        self.cameras.get_mut(name)
    }

    /// Iterate all cameras as (name, camera) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Camera)> {
        self.cameras.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Number of cameras.
    pub fn count(&self) -> usize {
        self.cameras.len()
    }

    /// Make a named camera follow an object with the given smoothing factor.
    /// Smoothing: 0.0 = no movement, 1.0 = instant snap. Typical: 0.05-0.2.
    pub fn follow(&mut self, camera_name: &str, target: ObjectId, smoothing: f32) {
        self.follow_targets.insert(camera_name.to_string(), FollowTarget {
            object: target,
            smoothing,
            offset: Vec2::ZERO,
        });
    }

    /// Like `follow`, but applies a fixed offset to the camera position.
    /// Useful for shifting the view (e.g. `Vec2::new(0.0, 3.0)` looks 3 units above the target).
    pub fn follow_with_offset(&mut self, camera_name: &str, target: ObjectId, smoothing: f32, offset: Vec2) {
        self.follow_targets.insert(camera_name.to_string(), FollowTarget {
            object: target,
            smoothing,
            offset,
        });
    }

    /// Set the follow offset for an already-following camera. No-op if not following.
    pub fn set_follow_offset(&mut self, camera_name: &str, offset: Vec2) {
        if let Some(ft) = self.follow_targets.get_mut(camera_name) {
            ft.offset = offset;
        }
    }

    /// Stop a named camera from following any object.
    pub fn unfollow(&mut self, camera_name: &str) {
        self.follow_targets.remove(camera_name);
    }

    /// Update all camera follow targets from current object positions.
    /// Called by `World::step()` after the physics step.
    pub fn update_follows(&mut self, objects: &ObjectSystem) {
        // Collect follow data to avoid borrow conflicts
        let updates: Vec<(String, Vec2, f32)> = self.follow_targets.iter()
            .filter_map(|(cam_name, ft)| {
                let pos = objects.get_position(ft.object);
                Some((cam_name.clone(), pos + ft.offset, ft.smoothing))
            })
            .collect();

        for (cam_name, pos, smoothing) in updates {
            if let Some(camera) = self.cameras.get_mut(&cam_name) {
                camera.move_toward(pos.x, pos.y, smoothing);
            }
        }
    }
}

impl Default for CameraSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_main_camera() {
        let cameras = CameraSystem::new();
        assert!(cameras.get("main").is_some());
        assert_eq!(cameras.count(), 1);
    }

    #[test]
    fn add_and_remove_camera() {
        let mut cameras = CameraSystem::new();
        cameras.add("minimap", Camera::new(100.0, 75.0));
        assert_eq!(cameras.count(), 2);
        assert!(cameras.get("minimap").is_some());

        cameras.remove("minimap");
        assert_eq!(cameras.count(), 1);
        assert!(cameras.get("minimap").is_none());
    }

    #[test]
    fn get_mut_camera() {
        let mut cameras = CameraSystem::new();
        let cam = cameras.get_mut("main").unwrap();
        cam.zoom = 2.0;
        assert_eq!(cameras.get("main").unwrap().zoom, 2.0);
    }

    #[test]
    fn follow_and_unfollow() {
        let mut cameras = CameraSystem::new();
        let id = ObjectId::PLACEHOLDER;

        cameras.follow("main", id, 0.1);
        // follow_targets is private, just verify unfollow doesn't panic
        cameras.unfollow("main");
    }

    #[test]
    fn iter_cameras() {
        let mut cameras = CameraSystem::new();
        cameras.add("ui", Camera::new(16.0, 9.0));

        let names: Vec<&str> = cameras.iter().map(|(name, _)| name).collect();
        assert!(names.contains(&"main"));
        assert!(names.contains(&"ui"));
    }
}
