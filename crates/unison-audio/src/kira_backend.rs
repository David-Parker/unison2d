//! Production `AudioBackend` implementation backed by the `kira` crate.
//!
//! All kira imports are intentionally kept inside this module so the rest of
//! the crate stays backend-agnostic. Callers go through
//! [`crate::AudioBackend`] only.
//!
//! # V1 limitations
//!
//! - Spatial audio is implemented as a simple static distance attenuation
//!   applied at `play_spatial` time (linear or inverse-square falloff). There
//!   is no continuous per-frame positional update: [`KiraBackend::set_voice_position`]
//!   is a no-op. Moving sources should re-play the sound to reset attenuation.
//! - Master/bus volumes are mapped from linear amplitude to decibels via
//!   `20 * log10(v)` (clamping very small values to [`kira::Decibels::SILENCE`]).

use std::collections::HashMap;
use std::io::Cursor;
use std::time::Duration;

use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::sound::PlaybackState;
use kira::track::{MainTrackHandle, TrackBuilder, TrackHandle};
use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend, Decibels, PlaybackRate, Tween,
};

use unison_core::Vec2;

use crate::backend::{AudioBackend, BackendPlayParams, BackendSpatialParams};
use crate::id::{BackendBusId, BackendPlaybackId, BackendSoundId};
use crate::params::AudioError;

/// Production audio backend built on top of [`kira::AudioManager`].
///
/// Construct with [`KiraBackend::new`] — this opens the default cpal output
/// device. If no audio device is available the constructor returns
/// [`AudioError::BackendFailed`].
pub struct KiraBackend {
    manager: AudioManager<DefaultBackend>,

    sounds: HashMap<u32, StaticSoundData>,
    next_sound_id: u32,

    handles: HashMap<u32, StaticSoundHandle>,
    next_pb_id: u32,

    tracks: HashMap<u32, TrackHandle>,
    next_track_id: u32,

    /// Last-known listener position (only used for spatial attenuation).
    listener: Vec2,
}

impl KiraBackend {
    /// Opens the default audio device and returns a new backend.
    ///
    /// Returns [`AudioError::BackendFailed`] if the audio manager fails to
    /// initialize (for example on headless systems with no output device).
    pub fn new() -> Result<Self, AudioError> {
        let manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .map_err(|e| AudioError::BackendFailed(format!("{e}")))?;
        Ok(Self {
            manager,
            sounds: HashMap::new(),
            next_sound_id: 1,
            handles: HashMap::new(),
            next_pb_id: 1,
            tracks: HashMap::new(),
            next_track_id: 1,
            listener: Vec2::ZERO,
        })
    }
}

/// Linear amplitude → decibels. `0.0` (and anything approaching it) clamps to
/// [`Decibels::SILENCE`] so the sound actually goes quiet.
fn amp_to_db(v: f32) -> Decibels {
    if v <= 0.0001 {
        Decibels::SILENCE
    } else {
        Decibels(20.0 * v.log10())
    }
}

/// Translate an optional fade duration (seconds) into a [`Tween`]. `None`
/// gives the kira default (a ~10 ms micro-fade that avoids clicks).
fn tween_or_default(secs: Option<f32>) -> Tween {
    match secs {
        Some(s) if s > 0.0 => Tween {
            duration: Duration::from_secs_f32(s),
            ..Tween::default()
        },
        _ => Tween::default(),
    }
}

/// V1 static distance attenuation.
///
/// - `base_vol` is the caller-requested linear volume (pre-attenuation).
/// - `pos`, `listener` are world-space 2D positions.
/// - Beyond `max_dist` the result is `0`.
/// - Otherwise `n = d / max_dist` and:
///   - inverse-square: `1 / (1 + 8·n²)`
///   - linear:         `1 - n`
fn attenuate(base_vol: f32, pos: Vec2, listener: Vec2, max_dist: f32, inverse_square: bool) -> f32 {
    if max_dist <= 0.0 {
        return base_vol;
    }
    let dx = pos.x - listener.x;
    let dy = pos.y - listener.y;
    let d = (dx * dx + dy * dy).sqrt();
    if d >= max_dist {
        return 0.0;
    }
    let n = d / max_dist;
    let falloff = if inverse_square {
        1.0 / (1.0 + 8.0 * n * n)
    } else {
        1.0 - n
    };
    base_vol * falloff
}

impl KiraBackend {
    /// Route a `StaticSoundData` play to the appropriate track. Bus id `0` is
    /// reserved for "route to the main track" (fallback used when sub-track
    /// capacity is exhausted at bus-creation time).
    fn play_routed(
        &mut self,
        bus: BackendBusId,
        data: StaticSoundData,
    ) -> Result<StaticSoundHandle, AudioError> {
        if bus.raw() == 0 {
            self.manager
                .play(data)
                .map_err(|e| AudioError::BackendFailed(format!("{e}")))
        } else if let Some(track) = self.tracks.get_mut(&bus.raw()) {
            track
                .play(data)
                .map_err(|e| AudioError::BackendFailed(format!("{e}")))
        } else {
            Err(AudioError::BackendFailed(format!(
                "unknown bus: {}",
                bus.raw()
            )))
        }
    }
}

impl AudioBackend for KiraBackend {
    fn load_sound(&mut self, bytes: &[u8]) -> Result<BackendSoundId, AudioError> {
        // StaticSoundData::from_cursor needs owned bytes (MediaSource is 'static).
        let owned: Vec<u8> = bytes.to_vec();
        let data = StaticSoundData::from_cursor(Cursor::new(owned))
            .map_err(|e| AudioError::DecodeFailed(format!("{e}")))?;
        let id = self.next_sound_id;
        self.next_sound_id += 1;
        self.sounds.insert(id, data);
        Ok(BackendSoundId::from_raw(id))
    }

    fn unload_sound(&mut self, sound: BackendSoundId) {
        self.sounds.remove(&sound.raw());
    }

    fn play(
        &mut self,
        sound: BackendSoundId,
        params: BackendPlayParams,
    ) -> Result<BackendPlaybackId, AudioError> {
        let data = self
            .sounds
            .get(&sound.raw())
            .ok_or_else(|| AudioError::BackendFailed(format!("unknown sound: {}", sound.raw())))?
            .clone();

        let mut data = data
            .volume(amp_to_db(params.volume))
            .playback_rate(PlaybackRate(params.pitch as f64));
        if params.looping {
            data = data.loop_region(0.0..);
        }
        if let Some(fade) = params.fade_in {
            data = data.fade_in_tween(Some(Tween {
                duration: Duration::from_secs_f32(fade),
                ..Tween::default()
            }));
        }

        let handle = self.play_routed(params.bus, data)?;

        let id = self.next_pb_id;
        self.next_pb_id += 1;
        self.handles.insert(id, handle);
        Ok(BackendPlaybackId::from_raw(id))
    }

    fn play_spatial(
        &mut self,
        sound: BackendSoundId,
        params: BackendSpatialParams,
    ) -> Result<BackendPlaybackId, AudioError> {
        // V1 limitation: static attenuation at play time only; subsequent
        // set_voice_position is a no-op. See module docs.
        let attenuated = attenuate(
            params.volume,
            params.position,
            self.listener,
            params.max_distance,
            params.rolloff_inverse_square,
        );
        self.play(
            sound,
            BackendPlayParams {
                bus: params.bus,
                volume: attenuated,
                pitch: params.pitch,
                looping: params.looping,
                fade_in: params.fade_in,
            },
        )
    }

    fn stop(&mut self, playback: BackendPlaybackId, fade_out: Option<f32>) {
        if let Some(h) = self.handles.get_mut(&playback.raw()) {
            h.stop(tween_or_default(fade_out));
        }
    }

    fn pause(&mut self, playback: BackendPlaybackId) {
        if let Some(h) = self.handles.get_mut(&playback.raw()) {
            h.pause(Tween::default());
        }
    }

    fn resume(&mut self, playback: BackendPlaybackId) {
        if let Some(h) = self.handles.get_mut(&playback.raw()) {
            h.resume(Tween::default());
        }
    }

    fn is_playing(&self, playback: BackendPlaybackId) -> bool {
        match self.handles.get(&playback.raw()) {
            Some(h) => matches!(
                h.state(),
                PlaybackState::Playing | PlaybackState::Pausing | PlaybackState::Resuming
            ),
            None => false,
        }
    }

    fn set_voice_volume(&mut self, playback: BackendPlaybackId, v: f32, tween: Option<f32>) {
        if let Some(h) = self.handles.get_mut(&playback.raw()) {
            h.set_volume(amp_to_db(v), tween_or_default(tween));
        }
    }

    fn set_voice_pitch(&mut self, playback: BackendPlaybackId, p: f32, tween: Option<f32>) {
        if let Some(h) = self.handles.get_mut(&playback.raw()) {
            h.set_playback_rate(PlaybackRate(p as f64), tween_or_default(tween));
        }
    }

    fn set_voice_position(&mut self, _playback: BackendPlaybackId, _pos: Vec2) {
        // V1 limitation: spatial audio is a static attenuation applied at
        // play_spatial time. Per-voice position updates are a no-op for now.
    }

    fn set_master_volume(&mut self, v: f32, tween: Option<f32>) {
        let main: &mut MainTrackHandle = self.manager.main_track();
        main.set_volume(amp_to_db(v), tween_or_default(tween));
    }

    fn set_bus_volume(&mut self, bus: BackendBusId, v: f32, tween: Option<f32>) {
        if let Some(track) = self.tracks.get_mut(&bus.raw()) {
            track.set_volume(amp_to_db(v), tween_or_default(tween));
        }
    }

    fn create_bus(&mut self) -> BackendBusId {
        // Sub-tracks route to the main track by default in kira 0.12.
        let builder = TrackBuilder::new();
        match self.manager.add_sub_track(builder) {
            Ok(track) => {
                let id = self.next_track_id;
                self.next_track_id += 1;
                self.tracks.insert(id, track);
                BackendBusId::from_raw(id)
            }
            Err(_) => {
                // Resource limit hit — fall back to routing on the main
                // track (id 0) so the call never fails at the trait level.
                BackendBusId::from_raw(0)
            }
        }
    }

    fn set_listener(&mut self, pos: Vec2) {
        self.listener = pos;
    }

    fn suspend(&mut self) {
        self.manager
            .main_track()
            .set_volume(Decibels::SILENCE, Tween::default());
    }

    fn resume_system(&mut self) {
        self.manager
            .main_track()
            .set_volume(Decibels::IDENTITY, Tween::default());
    }

    fn tick(&mut self, _dt: f32) {
        // kira runs its own audio thread — nothing to do here.
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amp_to_db_silence() {
        assert_eq!(amp_to_db(0.0), Decibels::SILENCE);
    }

    #[test]
    fn amp_to_db_unity() {
        // 20 * log10(1.0) == 0.0
        let db = amp_to_db(1.0);
        assert!((db.0 - 0.0).abs() < 1e-4);
    }

    #[test]
    fn attenuate_at_listener_is_full() {
        let v = attenuate(1.0, Vec2::ZERO, Vec2::ZERO, 10.0, true);
        assert!((v - 1.0).abs() < 1e-6);
    }

    #[test]
    fn attenuate_at_max_dist_is_zero() {
        let v = attenuate(1.0, Vec2::new(10.0, 0.0), Vec2::ZERO, 10.0, false);
        assert_eq!(v, 0.0);
    }

    #[test]
    fn attenuate_linear_half() {
        let v = attenuate(1.0, Vec2::new(5.0, 0.0), Vec2::ZERO, 10.0, false);
        assert!((v - 0.5).abs() < 1e-6);
    }
}
