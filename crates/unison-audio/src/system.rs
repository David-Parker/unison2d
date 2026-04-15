//! `AudioSystem` — the user-facing audio API. Wraps an [`AudioBackend`].
//!
//! Owns:
//! - Sound registry (external `SoundId` → backend `BackendSoundId` + raw bytes ref)
//! - Bus registry (named: master, music, sfx + user-created)
//! - Playback registry (external `PlaybackId` → backend handle)
//! - Music bookkeeping (one-track-at-a-time invariant + crossfade)
//! - Listener position (pushed by the engine each frame)
//! - Web autoplay-gesture queue (calls before `arm()` are stored and replayed)

use std::collections::HashMap;
use unison_core::Vec2;

use crate::backend::{AudioBackend, BackendPlayParams, BackendSpatialParams};
use crate::id::{BackendBusId, BackendPlaybackId, BackendSoundId, BusId, PlaybackId, SoundId};
use crate::params::{AudioError, PlayParams, Rolloff, SpatialParams};

const MASTER_BUS_NAME: &str = "master";
const MUSIC_BUS_NAME:  &str = "music";
const SFX_BUS_NAME:    &str = "sfx";
const QUEUE_CAP: usize = 64;

#[derive(Clone, Debug)]
pub struct StopOptions {
    pub fade_out: Option<f32>,
}
impl Default for StopOptions {
    fn default() -> Self { Self { fade_out: None } }
}

#[derive(Clone, Debug)]
pub struct MusicOptions {
    pub bus: Option<BusId>, // default = music bus
    pub volume: f32,
    pub fade_in: Option<f32>,
    pub crossfade: Option<f32>,
}
impl Default for MusicOptions {
    fn default() -> Self {
        Self { bus: None, volume: 1.0, fade_in: None, crossfade: None }
    }
}

struct Bus { backend_id: BackendBusId }

struct Sound { backend_id: BackendSoundId }

struct Playback {
    backend_id: BackendPlaybackId,
    /// World scope tag for `stop_all_spatial_for`. None = non-positional.
    world_tag: Option<u32>,
}

/// Calls deferred until `arm()` (web-only autoplay gesture handling).
enum QueuedCall {
    Play { sound: SoundId, params: PlayParams },
    PlayMusic { sound: SoundId, opts: MusicOptions },
    SetMasterVolume { v: f32, tween: Option<f32> },
    SetBusVolume { bus: BusId, v: f32, tween: Option<f32> },
}

pub struct AudioSystem {
    backend: Box<dyn AudioBackend>,
    armed: bool,
    queue: Vec<QueuedCall>,
    sounds:    HashMap<u32, Sound>,
    buses:     HashMap<u32, Bus>,
    bus_names: HashMap<String, BusId>,
    playbacks: HashMap<u32, Playback>,
    next_sound: u32,
    next_bus: u32,
    next_playback: u32,
    listener: Vec2,
    /// Currently-playing music (PlaybackId on the music bus, exclusive).
    current_music: Option<PlaybackId>,
}

// --- Step 2: Constructors + bus setup ---

impl AudioSystem {
    /// Build an `AudioSystem` around the given backend. Auto-creates the
    /// `master`, `music`, and `sfx` buses.
    pub fn with_backend(mut backend: Box<dyn AudioBackend>) -> Self {
        let master_bid = backend.create_bus();
        let music_bid  = backend.create_bus();
        let sfx_bid    = backend.create_bus();

        let mut sys = Self {
            backend,
            armed: true, // overridden by `unarm_for_web` below
            queue: Vec::new(),
            sounds: HashMap::new(),
            buses: HashMap::new(),
            bus_names: HashMap::new(),
            playbacks: HashMap::new(),
            next_sound: 1,
            next_bus: 1,
            next_playback: 1,
            listener: Vec2::ZERO,
            current_music: None,
        };
        sys.register_bus(MASTER_BUS_NAME, master_bid);
        sys.register_bus(MUSIC_BUS_NAME,  music_bid);
        sys.register_bus(SFX_BUS_NAME,    sfx_bid);
        sys
    }

    fn register_bus(&mut self, name: &str, backend_id: BackendBusId) -> BusId {
        let id = BusId::from_raw(self.next_bus);
        self.next_bus += 1;
        self.buses.insert(id.raw(), Bus { backend_id });
        self.bus_names.insert(name.to_string(), id);
        id
    }

    /// For platforms that require a user gesture (web). Until `arm()` is
    /// called, `play` / `play_music` / volume changes are queued.
    pub fn unarm_for_web(&mut self) { self.armed = false; }

    pub fn arm(&mut self) {
        if self.armed { return; }
        self.armed = true;
        let queued = std::mem::take(&mut self.queue);
        for call in queued {
            match call {
                QueuedCall::Play { sound, params } => { let _ = self.play(sound, params); }
                QueuedCall::PlayMusic { sound, opts } => { let _ = self.play_music(sound, opts); }
                QueuedCall::SetMasterVolume { v, tween } => self.set_master_volume(v, tween),
                QueuedCall::SetBusVolume { bus, v, tween } => self.set_bus_volume(bus, v, tween),
            }
        }
    }

    /// Returns true if the call should be queued instead of executed.
    fn should_queue(&mut self, call: QueuedCall) -> bool {
        if self.armed { return false; }
        if self.queue.len() >= QUEUE_CAP {
            eprintln!("[unison-audio] pre-arm queue full ({QUEUE_CAP}); dropping call");
            return true;
        }
        self.queue.push(call);
        true
    }

    pub fn master_bus(&self) -> BusId { *self.bus_names.get(MASTER_BUS_NAME).unwrap() }
    pub fn music_bus(&self)  -> BusId { *self.bus_names.get(MUSIC_BUS_NAME).unwrap() }
    pub fn sfx_bus(&self)    -> BusId { *self.bus_names.get(SFX_BUS_NAME).unwrap() }

    pub fn bus_by_name(&self, name: &str) -> Option<BusId> {
        self.bus_names.get(name).copied()
    }

    pub fn create_bus(&mut self, name: &str) -> BusId {
        if let Some(existing) = self.bus_names.get(name) { return *existing; }
        let bid = self.backend.create_bus();
        self.register_bus(name, bid)
    }
}

// --- Step 3: Loading + playback ---

impl AudioSystem {
    pub fn load(&mut self, bytes: &[u8]) -> Result<SoundId, AudioError> {
        let backend_id = self.backend.load_sound(bytes)?;
        let id = SoundId::from_raw(self.next_sound);
        self.next_sound += 1;
        self.sounds.insert(id.raw(), Sound { backend_id });
        Ok(id)
    }

    pub fn unload(&mut self, sound: SoundId) {
        if let Some(s) = self.sounds.remove(&sound.raw()) {
            self.backend.unload_sound(s.backend_id);
        }
    }

    pub fn play(&mut self, sound: SoundId, params: PlayParams) -> Result<PlaybackId, AudioError> {
        if self.should_queue(QueuedCall::Play { sound, params: params.clone() }) {
            // Return a dummy ID; queued calls cannot return real handles.
            return Ok(PlaybackId::from_raw(0));
        }
        let backend_sound = self.sounds.get(&sound.raw())
            .ok_or(AudioError::NoSuchSound(sound))?
            .backend_id;
        let backend_bus = self.buses.get(&params.bus.raw())
            .ok_or(AudioError::NoSuchBus(params.bus))?
            .backend_id;
        let backend_pb = self.backend.play(backend_sound, BackendPlayParams {
            bus: backend_bus,
            volume: params.volume,
            pitch: params.pitch,
            looping: params.looping,
            fade_in: params.fade_in,
        })?;
        Ok(self.register_playback(backend_pb, None))
    }

    pub fn play_spatial(&mut self, sound: SoundId, params: SpatialParams,
                        world_tag: Option<u32>) -> Result<PlaybackId, AudioError> {
        // Spatial calls do NOT queue — they're tied to gameplay frames; if
        // the user hasn't gestured yet, dropping is the right behavior.
        if !self.armed { return Ok(PlaybackId::from_raw(0)); }
        let backend_sound = self.sounds.get(&sound.raw())
            .ok_or(AudioError::NoSuchSound(sound))?
            .backend_id;
        let backend_bus = self.buses.get(&params.bus.raw())
            .ok_or(AudioError::NoSuchBus(params.bus))?
            .backend_id;
        let backend_pb = self.backend.play_spatial(backend_sound, BackendSpatialParams {
            position: params.position,
            max_distance: params.max_distance,
            rolloff_inverse_square: matches!(params.rolloff, Rolloff::InverseSquare),
            bus: backend_bus,
            volume: params.volume,
            pitch: params.pitch,
            looping: params.looping,
            fade_in: params.fade_in,
        })?;
        Ok(self.register_playback(backend_pb, world_tag))
    }

    fn register_playback(&mut self, backend_id: BackendPlaybackId, world_tag: Option<u32>)
        -> PlaybackId
    {
        let id = PlaybackId::from_raw(self.next_playback);
        self.next_playback += 1;
        self.playbacks.insert(id.raw(), Playback { backend_id, world_tag });
        id
    }

    pub fn stop(&mut self, playback: PlaybackId, fade_out: Option<f32>) {
        if let Some(pb) = self.playbacks.remove(&playback.raw()) {
            self.backend.stop(pb.backend_id, fade_out);
            if self.current_music == Some(playback) { self.current_music = None; }
        }
    }
    pub fn pause(&mut self, playback: PlaybackId) {
        if let Some(pb) = self.playbacks.get(&playback.raw()) {
            self.backend.pause(pb.backend_id);
        }
    }
    pub fn resume(&mut self, playback: PlaybackId) {
        if let Some(pb) = self.playbacks.get(&playback.raw()) {
            self.backend.resume(pb.backend_id);
        }
    }
    pub fn is_playing(&self, playback: PlaybackId) -> bool {
        self.playbacks.get(&playback.raw())
            .map(|pb| self.backend.is_playing(pb.backend_id))
            .unwrap_or(false)
    }
}

// --- Step 4: Parameter changes + master/bus volumes + listener + lifecycle ---

impl AudioSystem {
    pub fn set_volume(&mut self, playback: PlaybackId, volume: f32, tween: Option<f32>) {
        if let Some(pb) = self.playbacks.get(&playback.raw()) {
            self.backend.set_voice_volume(pb.backend_id, volume, tween);
        }
    }
    pub fn set_pitch(&mut self, playback: PlaybackId, pitch: f32, tween: Option<f32>) {
        if let Some(pb) = self.playbacks.get(&playback.raw()) {
            self.backend.set_voice_pitch(pb.backend_id, pitch, tween);
        }
    }
    pub fn set_position(&mut self, playback: PlaybackId, position: Vec2) {
        if let Some(pb) = self.playbacks.get(&playback.raw()) {
            self.backend.set_voice_position(pb.backend_id, position);
        }
    }

    pub fn set_master_volume(&mut self, v: f32, tween: Option<f32>) {
        if self.should_queue(QueuedCall::SetMasterVolume { v, tween }) { return; }
        self.backend.set_master_volume(v, tween);
    }
    pub fn set_bus_volume(&mut self, bus: BusId, v: f32, tween: Option<f32>) {
        if self.should_queue(QueuedCall::SetBusVolume { bus, v, tween }) { return; }
        if let Some(b) = self.buses.get(&bus.raw()) {
            self.backend.set_bus_volume(b.backend_id, v, tween);
        }
    }

    pub fn set_listener_position(&mut self, position: Vec2) {
        self.listener = position;
        self.backend.set_listener(position);
    }
    pub fn listener_position(&self) -> Vec2 { self.listener }

    pub fn suspend(&mut self) { self.backend.suspend(); }
    pub fn resume_system(&mut self) { self.backend.resume_system(); }

    pub fn tick(&mut self, dt: f32) { self.backend.tick(dt); }

    /// Stops every non-spatial (world_tag = None) playback.
    pub fn stop_all(&mut self, fade_out: Option<f32>) {
        let ids: Vec<PlaybackId> = self.playbacks.iter()
            .filter(|(_, pb)| pb.world_tag.is_none())
            .map(|(k, _)| PlaybackId::from_raw(*k))
            .collect();
        for id in ids { self.stop(id, fade_out); }
    }

    /// Stops every spatial playback with the given world tag.
    pub fn stop_all_spatial_for(&mut self, world: u32, fade_out: Option<f32>) {
        let ids: Vec<PlaybackId> = self.playbacks.iter()
            .filter(|(_, pb)| pb.world_tag == Some(world))
            .map(|(k, _)| PlaybackId::from_raw(*k))
            .collect();
        for id in ids { self.stop(id, fade_out); }
    }
}

// --- Step 5: Music convenience API ---

impl AudioSystem {
    pub fn play_music(&mut self, sound: SoundId, opts: MusicOptions) -> Result<PlaybackId, AudioError> {
        if self.should_queue(QueuedCall::PlayMusic { sound, opts: opts.clone() }) {
            return Ok(PlaybackId::from_raw(0));
        }
        // Crossfade: stop current with fade_out matching crossfade.
        if let Some(prev) = self.current_music.take() {
            let fade = opts.crossfade;
            self.stop(prev, fade);
        }
        let bus = opts.bus.unwrap_or_else(|| self.music_bus());
        let pb = self.play(sound, PlayParams {
            bus,
            volume: opts.volume,
            pitch: 1.0,
            looping: true,
            fade_in: opts.fade_in.or(opts.crossfade),
        })?;
        self.current_music = Some(pb);
        Ok(pb)
    }

    pub fn stop_music(&mut self, fade_out: Option<f32>) {
        if let Some(pb) = self.current_music.take() {
            self.stop(pb, fade_out);
        }
    }
    pub fn pause_music(&mut self) {
        if let Some(pb) = self.current_music { self.pause(pb); }
    }
    pub fn resume_music(&mut self) {
        if let Some(pb) = self.current_music { self.resume(pb); }
    }
    pub fn current_music(&self) -> Option<PlaybackId> { self.current_music }
}

// --- Step 6: Unit tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stub_backend::{StubBackend, StubEvent};

    fn sys() -> AudioSystem { AudioSystem::with_backend(Box::new(StubBackend::new())) }

    #[test]
    fn builtin_buses_exist() {
        let s = sys();
        assert!(s.bus_by_name("master").is_some());
        assert!(s.bus_by_name("music").is_some());
        assert!(s.bus_by_name("sfx").is_some());
    }

    #[test]
    fn create_user_bus_is_idempotent_by_name() {
        let mut s = sys();
        let a = s.create_bus("ui");
        let b = s.create_bus("ui");
        assert_eq!(a, b);
    }

    #[test]
    fn play_routes_to_chosen_bus() {
        let mut s = sys();
        let snd = s.load(&[1u8; 8]).unwrap();
        let sfx = s.sfx_bus();
        let pb = s.play(snd, PlayParams::with_bus(sfx)).unwrap();
        assert!(pb.raw() != 0);
    }

    #[test]
    fn music_crossfade_stops_previous_and_starts_new() {
        let mut s = sys();
        let a = s.load(&[0u8; 4]).unwrap();
        let b = s.load(&[0u8; 4]).unwrap();
        s.play_music(a, MusicOptions::default()).unwrap();
        s.play_music(b, MusicOptions { crossfade: Some(1.0), ..Default::default() }).unwrap();
        assert!(s.current_music().is_some());
        // We should have at least one Stop event for the prior music.
        let backend: &StubBackend = s.backend_for_test();
        assert!(backend.events.iter().any(|e| matches!(e, StubEvent::Stop { .. })));
    }

    #[test]
    fn pre_arm_queue_replays_on_arm() {
        let mut s = sys();
        s.unarm_for_web();
        let snd = s.load(&[0u8; 4]).unwrap();
        s.play(snd, PlayParams::with_bus(s.sfx_bus())).unwrap(); // queued
        // No backend Play yet (Load happened, but Play was queued).
        assert!(!s.backend_for_test().events.iter().any(|e| matches!(e, StubEvent::Play { .. })));
        s.arm();
        assert!(s.backend_for_test().events.iter().any(|e| matches!(e, StubEvent::Play { .. })));
    }

    #[test]
    fn listener_position_pushes_to_backend() {
        let mut s = sys();
        s.set_listener_position(Vec2::new(3.0, 5.0));
        let last = s.backend_for_test().events.last().cloned().unwrap();
        assert_eq!(last, StubEvent::SetListener(Vec2::new(3.0, 5.0)));
    }

    #[test]
    fn queue_cap_drops_overflow() {
        let mut s = sys();
        s.unarm_for_web();
        for _ in 0..(QUEUE_CAP + 5) {
            s.set_master_volume(0.5, None);
        }
        assert_eq!(s.queue.len(), QUEUE_CAP);
    }
}

// Test-only accessor for inspecting the backend in unit tests.
#[cfg(test)]
impl AudioSystem {
    pub(crate) fn backend_for_test(&self) -> &crate::stub_backend::StubBackend {
        self.backend.as_any()
            .downcast_ref::<crate::stub_backend::StubBackend>()
            .expect("test backend must be StubBackend")
    }
}
