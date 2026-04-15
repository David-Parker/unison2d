//! Parameter structs for [`crate::AudioSystem`] calls + the [`AudioError`] enum.

use unison_core::Vec2;
use crate::id::{BusId, PlaybackId, SoundId};

#[derive(Copy, Clone, Debug)]
pub enum Rolloff {
    Linear,
    InverseSquare,
}

impl Default for Rolloff {
    fn default() -> Self { Rolloff::InverseSquare }
}

#[derive(Clone, Debug)]
pub struct PlayParams {
    pub bus: BusId,
    pub volume: f32,
    pub pitch: f32,
    pub looping: bool,
    pub fade_in: Option<f32>,
}

impl PlayParams {
    /// Defaults: unity volume, pitch 1.0, non-looping, no fade.
    /// `bus` must be filled in by caller (typically the SFX bus).
    pub fn with_bus(bus: BusId) -> Self {
        Self { bus, volume: 1.0, pitch: 1.0, looping: false, fade_in: None }
    }
}

#[derive(Clone, Debug)]
pub struct SpatialParams {
    pub position: Vec2,
    pub max_distance: f32,
    pub rolloff: Rolloff,
    pub bus: BusId,
    pub volume: f32,
    pub pitch: f32,
    pub looping: bool,
    pub fade_in: Option<f32>,
}

impl SpatialParams {
    pub fn at(position: Vec2, bus: BusId) -> Self {
        Self {
            position, max_distance: 30.0, rolloff: Rolloff::default(),
            bus, volume: 1.0, pitch: 1.0, looping: false, fade_in: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("audio format unsupported")]
    UnsupportedFormat,
    #[error("decode failed: {0}")]
    DecodeFailed(String),
    #[error("backend error: {0}")]
    BackendFailed(String),
    #[error("no such sound: {0:?}")]
    NoSuchSound(SoundId),
    #[error("no such playback: {0:?}")]
    NoSuchPlayback(PlaybackId),
    #[error("no such bus: {0:?}")]
    NoSuchBus(BusId),
}
