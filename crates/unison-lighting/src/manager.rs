//! Lighting manager for coordinating lights and shadow maps.

use crate::light::Light;
use crate::shadow::{
    compute_shadow_map, LightHandle, ShadowCaster, ShadowMapCache, ShadowMapId, ShadowQuality,
};
use unison_math::{Color, Rect};
use unison_profiler::profile_scope;

/// Manages all lights and their shadow maps in a scene.
pub struct LightingManager {
    /// All lights in the scene.
    lights: Vec<Option<Light>>,
    /// Global ambient light color.
    ambient: Color,
    /// Shadow map cache.
    shadow_cache: ShadowMapCache,
    /// Shadow map IDs for each light (parallel to lights vec).
    light_shadow_maps: Vec<Option<ShadowMapId>>,
    /// Current shadow quality setting.
    shadow_quality: ShadowQuality,
    /// Next light handle to assign.
    next_handle: u32,
    /// Free light slots for reuse.
    free_slots: Vec<usize>,
}

impl Default for LightingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LightingManager {
    /// Create a new lighting manager.
    pub fn new() -> Self {
        Self {
            lights: Vec::new(),
            ambient: Color::rgb(0.1, 0.1, 0.1),
            shadow_cache: ShadowMapCache::new(),
            light_shadow_maps: Vec::new(),
            shadow_quality: ShadowQuality::Medium,
            next_handle: 0,
            free_slots: Vec::new(),
        }
    }

    /// Add a light to the scene.
    pub fn add_light(&mut self, light: Light) -> LightHandle {
        let handle = LightHandle(self.next_handle);
        self.next_handle += 1;

        // Allocate shadow map if light casts shadows
        let shadow_map_id = if light.shadows && self.shadow_quality != ShadowQuality::Off {
            Some(self.shadow_cache.allocate(self.shadow_quality.resolution()))
        } else {
            None
        };

        if let Some(slot) = self.free_slots.pop() {
            self.lights[slot] = Some(light);
            self.light_shadow_maps[slot] = shadow_map_id;
        } else {
            self.lights.push(Some(light));
            self.light_shadow_maps.push(shadow_map_id);
        }

        handle
    }

    /// Remove a light from the scene.
    pub fn remove_light(&mut self, handle: LightHandle) {
        let index = handle.0 as usize;
        if index < self.lights.len() {
            self.lights[index] = None;
            if let Some(shadow_id) = self.light_shadow_maps[index].take() {
                self.shadow_cache.free(shadow_id);
            }
            self.free_slots.push(index);
        }
    }

    /// Get a light by handle.
    pub fn get_light(&self, handle: LightHandle) -> Option<&Light> {
        self.lights.get(handle.0 as usize).and_then(|l| l.as_ref())
    }

    /// Get a mutable light by handle.
    pub fn get_light_mut(&mut self, handle: LightHandle) -> Option<&mut Light> {
        self.lights
            .get_mut(handle.0 as usize)
            .and_then(|l| l.as_mut())
    }

    /// Set the global ambient light color.
    pub fn set_ambient(&mut self, color: Color) {
        self.ambient = color;
    }

    /// Get the current ambient light color.
    pub fn ambient(&self) -> Color {
        self.ambient
    }

    /// Set the shadow quality level.
    pub fn set_shadow_quality(&mut self, quality: ShadowQuality) {
        if self.shadow_quality != quality {
            self.shadow_quality = quality;
            // Reallocate all shadow maps at new resolution
            for (i, shadow_id) in self.light_shadow_maps.iter_mut().enumerate() {
                if let Some(id) = shadow_id.take() {
                    self.shadow_cache.free(id);
                }
                if let Some(light) = &self.lights[i] {
                    if light.shadows && quality != ShadowQuality::Off {
                        *shadow_id = Some(self.shadow_cache.allocate(quality.resolution()));
                    }
                }
            }
        }
    }

    /// Get the current shadow quality level.
    pub fn shadow_quality(&self) -> ShadowQuality {
        self.shadow_quality
    }

    /// Update shadow maps for lights that need it.
    pub fn update_shadows(&mut self, occluders: &[&dyn ShadowCaster]) {
        profile_scope!("lighting.update_shadows");

        if self.shadow_quality == ShadowQuality::Off {
            return;
        }

        // Collect all occluder segments
        let segments: Vec<_> = {
            profile_scope!("lighting.collect_occluders");
            occluders
                .iter()
                .flat_map(|o| o.get_occluder_segments())
                .collect()
        };

        // Update dirty shadow maps
        {
            profile_scope!("lighting.compute_shadow_maps");
            for (i, shadow_id) in self.light_shadow_maps.iter().enumerate() {
                if let Some(id) = shadow_id {
                    if let Some(shadow_map) = self.shadow_cache.get_mut(*id) {
                        if shadow_map.dirty {
                            if let Some(light) = &self.lights[i] {
                                compute_shadow_map(shadow_map, light, &segments);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Get all lights visible within the given bounds.
    pub fn get_visible_lights(&self, bounds: &Rect) -> Vec<&Light> {
        profile_scope!("lighting.get_visible_lights");

        self.lights
            .iter()
            .filter_map(|l| l.as_ref())
            .filter(|light| {
                if !light.enabled {
                    return false;
                }
                let radius = light.effective_radius();
                if radius.is_infinite() {
                    true // Directional lights are always visible
                } else {
                    bounds.intersects_circle(light.position, radius)
                }
            })
            .collect()
    }

    /// Mark a light's shadow map as dirty (call when light moves).
    pub fn mark_dirty(&mut self, handle: LightHandle) {
        let index = handle.0 as usize;
        if let Some(shadow_id) = self.light_shadow_maps.get(index).and_then(|id| *id) {
            if let Some(shadow_map) = self.shadow_cache.get_mut(shadow_id) {
                shadow_map.mark_dirty();
            }
        }
    }

    /// Mark all shadow maps as dirty (call when occluders move).
    pub fn mark_all_dirty(&mut self) {
        self.shadow_cache.mark_all_dirty();
    }

    /// Get the number of active lights.
    pub fn light_count(&self) -> usize {
        self.lights.iter().filter(|l| l.is_some()).count()
    }

    /// Get all active lights.
    pub fn all_lights(&self) -> impl Iterator<Item = &Light> {
        self.lights.iter().filter_map(|l| l.as_ref())
    }

    /// Get shadow map ID for a light.
    pub fn get_shadow_map_id(&self, handle: LightHandle) -> Option<ShadowMapId> {
        self.light_shadow_maps
            .get(handle.0 as usize)
            .and_then(|id| *id)
    }

    /// Get a shadow map by ID.
    pub fn get_shadow_map(&self, id: ShadowMapId) -> Option<&crate::shadow::ShadowMap> {
        self.shadow_cache.get(id)
    }
}
