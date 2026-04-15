//! Backend abstraction. The default implementation is `KiraBackend`
//! (added in Task 4); a `StubBackend` is provided for tests.

use unison_core::Vec2;
use crate::id::{BackendBusId, BackendPlaybackId, BackendSoundId};
use crate::params::AudioError;

/// Backend-side equivalent of [`crate::PlayParams`]. Bus is already
/// translated to a backend handle.
#[derive(Clone, Debug)]
pub struct BackendPlayParams {
    pub bus: BackendBusId,
    pub volume: f32,
    pub pitch: f32,
    pub looping: bool,
    pub fade_in: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct BackendSpatialParams {
    pub position: Vec2,
    pub max_distance: f32,
    pub rolloff_inverse_square: bool, // true = InverseSquare, false = Linear
    pub bus: BackendBusId,
    pub volume: f32,
    pub pitch: f32,
    pub looping: bool,
    pub fade_in: Option<f32>,
}

pub trait AudioBackend: Send + std::any::Any {
    fn load_sound(&mut self, bytes: &[u8]) -> Result<BackendSoundId, AudioError>;
    fn unload_sound(&mut self, sound: BackendSoundId);

    fn play(&mut self, sound: BackendSoundId, params: BackendPlayParams)
        -> Result<BackendPlaybackId, AudioError>;
    fn play_spatial(&mut self, sound: BackendSoundId, params: BackendSpatialParams)
        -> Result<BackendPlaybackId, AudioError>;

    fn stop(&mut self, playback: BackendPlaybackId, fade_out: Option<f32>);
    fn pause(&mut self, playback: BackendPlaybackId);
    fn resume(&mut self, playback: BackendPlaybackId);
    fn is_playing(&self, playback: BackendPlaybackId) -> bool;

    fn set_voice_volume(&mut self, playback: BackendPlaybackId, v: f32, tween: Option<f32>);
    fn set_voice_pitch(&mut self, playback: BackendPlaybackId, p: f32, tween: Option<f32>);
    fn set_voice_position(&mut self, playback: BackendPlaybackId, pos: Vec2);

    fn set_master_volume(&mut self, v: f32, tween: Option<f32>);
    fn set_bus_volume(&mut self, bus: BackendBusId, v: f32, tween: Option<f32>);
    fn create_bus(&mut self) -> BackendBusId;

    fn set_listener(&mut self, pos: Vec2);

    fn suspend(&mut self);
    fn resume_system(&mut self);
    fn tick(&mut self, dt: f32);

    #[doc(hidden)]
    fn as_any(&self) -> &dyn std::any::Any;
}
