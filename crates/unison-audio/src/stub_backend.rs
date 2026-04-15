//! In-memory recording backend used by unit + integration tests.

use std::sync::atomic::{AtomicU32, Ordering};
use unison_core::Vec2;

use crate::backend::{AudioBackend, BackendPlayParams, BackendSpatialParams};
use crate::id::{BackendBusId, BackendPlaybackId, BackendSoundId};
use crate::params::AudioError;

#[derive(Clone, Debug, PartialEq)]
pub enum StubEvent {
    LoadSound { bytes_len: usize },
    UnloadSound(BackendSoundId),
    Play { sound: BackendSoundId, volume: f32, looping: bool, bus: BackendBusId },
    PlaySpatial { sound: BackendSoundId, position: Vec2, bus: BackendBusId },
    Stop { playback: BackendPlaybackId, fade_out: Option<f32> },
    Pause(BackendPlaybackId),
    Resume(BackendPlaybackId),
    SetVoiceVolume { playback: BackendPlaybackId, v: f32, tween: Option<f32> },
    SetVoicePitch  { playback: BackendPlaybackId, p: f32, tween: Option<f32> },
    SetVoicePosition { playback: BackendPlaybackId, pos: Vec2 },
    SetMasterVolume { v: f32, tween: Option<f32> },
    SetBusVolume    { bus: BackendBusId, v: f32, tween: Option<f32> },
    CreateBus,
    SetListener(Vec2),
    Suspend,
    ResumeSystem,
    Tick(f32),
}

#[derive(Default)]
pub struct StubBackend {
    next_sound: AtomicU32,
    next_bus: AtomicU32,
    next_playback: AtomicU32,
    pub events: Vec<StubEvent>,
    /// PlaybackIds that should report as "still playing" — tests can mutate.
    pub alive: std::collections::HashSet<u32>,
}

impl StubBackend {
    pub fn new() -> Self { Self::default() }

    fn fresh_sound(&self) -> BackendSoundId {
        BackendSoundId::from_raw(self.next_sound.fetch_add(1, Ordering::Relaxed))
    }
    fn fresh_bus(&self) -> BackendBusId {
        BackendBusId::from_raw(self.next_bus.fetch_add(1, Ordering::Relaxed))
    }
    fn fresh_playback(&mut self) -> BackendPlaybackId {
        let id = self.next_playback.fetch_add(1, Ordering::Relaxed);
        self.alive.insert(id);
        BackendPlaybackId::from_raw(id)
    }
}

impl AudioBackend for StubBackend {
    fn load_sound(&mut self, bytes: &[u8]) -> Result<BackendSoundId, AudioError> {
        self.events.push(StubEvent::LoadSound { bytes_len: bytes.len() });
        Ok(self.fresh_sound())
    }
    fn unload_sound(&mut self, sound: BackendSoundId) {
        self.events.push(StubEvent::UnloadSound(sound));
    }
    fn play(&mut self, sound: BackendSoundId, params: BackendPlayParams)
        -> Result<BackendPlaybackId, AudioError>
    {
        let id = self.fresh_playback();
        self.events.push(StubEvent::Play {
            sound, volume: params.volume, looping: params.looping, bus: params.bus,
        });
        Ok(id)
    }
    fn play_spatial(&mut self, sound: BackendSoundId, params: BackendSpatialParams)
        -> Result<BackendPlaybackId, AudioError>
    {
        let id = self.fresh_playback();
        self.events.push(StubEvent::PlaySpatial {
            sound, position: params.position, bus: params.bus,
        });
        Ok(id)
    }
    fn stop(&mut self, playback: BackendPlaybackId, fade_out: Option<f32>) {
        self.alive.remove(&playback.raw());
        self.events.push(StubEvent::Stop { playback, fade_out });
    }
    fn pause(&mut self, playback: BackendPlaybackId) {
        self.events.push(StubEvent::Pause(playback));
    }
    fn resume(&mut self, playback: BackendPlaybackId) {
        self.events.push(StubEvent::Resume(playback));
    }
    fn is_playing(&self, playback: BackendPlaybackId) -> bool {
        self.alive.contains(&playback.raw())
    }
    fn set_voice_volume(&mut self, playback: BackendPlaybackId, v: f32, tween: Option<f32>) {
        self.events.push(StubEvent::SetVoiceVolume { playback, v, tween });
    }
    fn set_voice_pitch(&mut self, playback: BackendPlaybackId, p: f32, tween: Option<f32>) {
        self.events.push(StubEvent::SetVoicePitch { playback, p, tween });
    }
    fn set_voice_position(&mut self, playback: BackendPlaybackId, pos: Vec2) {
        self.events.push(StubEvent::SetVoicePosition { playback, pos });
    }
    fn set_master_volume(&mut self, v: f32, tween: Option<f32>) {
        self.events.push(StubEvent::SetMasterVolume { v, tween });
    }
    fn set_bus_volume(&mut self, bus: BackendBusId, v: f32, tween: Option<f32>) {
        self.events.push(StubEvent::SetBusVolume { bus, v, tween });
    }
    fn create_bus(&mut self) -> BackendBusId {
        self.events.push(StubEvent::CreateBus);
        self.fresh_bus()
    }
    fn set_listener(&mut self, pos: Vec2) {
        self.events.push(StubEvent::SetListener(pos));
    }
    fn suspend(&mut self) { self.events.push(StubEvent::Suspend); }
    fn resume_system(&mut self) { self.events.push(StubEvent::ResumeSystem); }
    fn tick(&mut self, dt: f32) { self.events.push(StubEvent::Tick(dt)); }
    fn as_any(&self) -> &dyn std::any::Any { self }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::AudioBackend;

    #[test]
    fn records_load_and_play() {
        let mut b = StubBackend::new();
        let sound = b.load_sound(&[0u8; 4]).unwrap();
        let bus = b.create_bus();
        let pb = b.play(sound, BackendPlayParams {
            bus, volume: 0.7, pitch: 1.0, looping: false, fade_in: None,
        }).unwrap();
        assert!(b.is_playing(pb));
        b.stop(pb, None);
        assert!(!b.is_playing(pb));
        assert_eq!(b.events.len(), 4);
    }
}
