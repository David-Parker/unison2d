# unison-audio

Cross-platform audio: music, SFX, 2D-spatial playback, mix buses, and tweened parameter changes.

The crate exposes an `AudioSystem` built on top of a swappable `AudioBackend`. The default backend is `KiraBackend` (feature-gated on `backend-kira`, enabled by default) which uses [kira 0.12](https://crates.io/crates/kira) over `cpal` + `symphonia`. A `StubBackend` is provided for tests and records every call as a `StubEvent` for assertions.

## Overview

`AudioSystem` owns the public handles and routing; the backend owns actual decoding and output. Each subsystem has its own ID space:

- `SoundId` — a loaded sound asset
- `BusId` — a mix bus (master/music/sfx plus user-created)
- `PlaybackId` — a single playing instance of a sound

External IDs are stable `u32` newtypes with `from_raw` / `raw` accessors, mirroring `LightId` in `unison-lighting`. Backend-internal IDs (`BackendSoundId`, `BackendBusId`, `BackendPlaybackId`) are kept separate so the system can translate between its own external IDs and whatever the backend hands back.

Game code never talks to the backend directly — it uses `AudioSystem` through the engine, which in turn exposes the Lua/TypeScript API documented in [scripting/api-reference.md](../scripting/api-reference.md).

## Concepts

### Sounds

A sound is the decoded audio asset. Load with `AudioSystem::load(&bytes)` and free with `unload`. The default feature set supports **OGG Vorbis** and **WAV** (feature gates: `codec-ogg`, `codec-wav`; optional `codec-mp3`, `codec-flac`). Loading rejects unsupported formats with `AudioError::UnsupportedFormat`.

### Buses

Three buses are auto-created on construction: `"master"`, `"music"`, `"sfx"`. Additional buses can be created by name with `create_bus(name)` (idempotent — the same name always returns the same `BusId`). In the current kira backend all sub-tracks route directly to the main track — there is no bus-hierarchy; the master track handles global attenuation.

### Playbacks

Every call to `play` / `play_spatial` / `play_music` returns a `PlaybackId`. The ID is valid until the sound naturally ends or is `stop`-ped. You can `pause` / `resume` a playback, change its volume or pitch (optionally tweened), or reposition spatial voices.

### Listener

The listener is a single world-space `Vec2` pushed by the engine every frame via `set_listener_position`. Spatial voices attenuate based on the initial distance from the listener at play time (V1 limitation — see below). `listener_position()` reads the current value.

### Music crossfade

`play_music` enforces a one-track-at-a-time invariant: calling it while another track is playing stops the previous track (using the new track's `crossfade` as a fade-out) and fades the new track in. `current_music()` returns the active track's `PlaybackId` or `None`. `stop_music` / `pause_music` / `resume_music` act on whatever is current.

### Pre-arm queue (web)

Web browsers require a user gesture before audio may start. Call `AudioSystem::unarm_for_web()` during construction on web; non-spatial `play`, `play_music`, `set_master_volume`, and `set_bus_volume` calls made before `arm()` are queued (cap: **64** calls; overflow is dropped with a warning) and replayed when `arm()` fires. Spatial playbacks are **not** queued — they are tied to gameplay frames, so dropping them before the gesture is the correct behavior.

The web platform crate installs a first-gesture listener that calls `arm()`. Other platforms stay armed from the start.

## AudioSystem API

```rust
pub struct AudioSystem { /* … */ }

impl AudioSystem {
    pub fn with_backend(backend: Box<dyn AudioBackend>) -> Self;

    // Built-in and custom buses
    pub fn master_bus(&self) -> BusId;
    pub fn music_bus(&self)  -> BusId;
    pub fn sfx_bus(&self)    -> BusId;
    pub fn bus_by_name(&self, name: &str) -> Option<BusId>;
    pub fn create_bus(&mut self, name: &str) -> BusId;

    // Sounds
    pub fn load(&mut self, bytes: &[u8]) -> Result<SoundId, AudioError>;
    pub fn unload(&mut self, sound: SoundId);

    // Playback
    pub fn play(&mut self, sound: SoundId, params: PlayParams)
        -> Result<PlaybackId, AudioError>;
    pub fn play_spatial(&mut self, sound: SoundId, params: SpatialParams,
                        world_tag: Option<u32>) -> Result<PlaybackId, AudioError>;
    pub fn stop(&mut self, playback: PlaybackId, fade_out: Option<f32>);
    pub fn pause(&mut self, playback: PlaybackId);
    pub fn resume(&mut self, playback: PlaybackId);
    pub fn is_playing(&self, playback: PlaybackId) -> bool;

    // Voice parameters
    pub fn set_volume(&mut self, pb: PlaybackId, v: f32, tween: Option<f32>);
    pub fn set_pitch (&mut self, pb: PlaybackId, p: f32, tween: Option<f32>);
    pub fn set_position(&mut self, pb: PlaybackId, pos: Vec2);

    // Mix
    pub fn set_master_volume(&mut self, v: f32, tween: Option<f32>);
    pub fn set_bus_volume(&mut self, bus: BusId, v: f32, tween: Option<f32>);

    // Music convenience
    pub fn play_music(&mut self, sound: SoundId, opts: MusicOptions)
        -> Result<PlaybackId, AudioError>;
    pub fn stop_music(&mut self, fade_out: Option<f32>);
    pub fn pause_music(&mut self);
    pub fn resume_music(&mut self);
    pub fn current_music(&self) -> Option<PlaybackId>;

    // Listener
    pub fn set_listener_position(&mut self, pos: Vec2);
    pub fn listener_position(&self) -> Vec2;

    // Lifecycle
    pub fn suspend(&mut self);
    pub fn resume_system(&mut self);
    pub fn tick(&mut self, dt: f32);

    // Bulk stop
    pub fn stop_all(&mut self, fade_out: Option<f32>);
    pub fn stop_all_spatial_for(&mut self, world: u32, fade_out: Option<f32>);

    // Web gesture gating
    pub fn unarm_for_web(&mut self);
    pub fn arm(&mut self);
}
```

### Parameter types

```rust
pub struct PlayParams {
    pub bus: BusId,
    pub volume: f32,          // 1.0 = unity
    pub pitch: f32,           // 1.0 = original rate
    pub looping: bool,
    pub fade_in: Option<f32>, // seconds
}

pub struct SpatialParams {
    pub position: Vec2,
    pub max_distance: f32,    // silence beyond this (default 30.0)
    pub rolloff: Rolloff,     // Linear or InverseSquare (default InverseSquare)
    pub bus: BusId,
    pub volume: f32,
    pub pitch: f32,
    pub looping: bool,
    pub fade_in: Option<f32>,
}

pub struct MusicOptions {
    pub bus: Option<BusId>,   // None = music_bus()
    pub volume: f32,
    pub fade_in: Option<f32>,
    pub crossfade: Option<f32>,
}

pub struct StopOptions { pub fade_out: Option<f32>, }

pub enum Rolloff { Linear, InverseSquare }

pub enum AudioError {
    UnsupportedFormat,
    DecodeFailed(String),
    BackendFailed(String),
    NoSuchSound(SoundId),
    NoSuchPlayback(PlaybackId),
    NoSuchBus(BusId),
}
```

## AudioBackend trait

Backends are `Send + Any` plugins that implement the decoding and output layer. They receive backend-side IDs plus `BackendPlayParams` / `BackendSpatialParams` (with the bus already translated to a backend handle).

```rust
pub trait AudioBackend: Send + std::any::Any {
    fn load_sound(&mut self, bytes: &[u8]) -> Result<BackendSoundId, AudioError>;
    fn unload_sound(&mut self, sound: BackendSoundId);

    fn play(&mut self, sound: BackendSoundId, params: BackendPlayParams)
        -> Result<BackendPlaybackId, AudioError>;
    fn play_spatial(&mut self, sound: BackendSoundId, params: BackendSpatialParams)
        -> Result<BackendPlaybackId, AudioError>;

    fn stop(&mut self, pb: BackendPlaybackId, fade_out: Option<f32>);
    fn pause(&mut self, pb: BackendPlaybackId);
    fn resume(&mut self, pb: BackendPlaybackId);
    fn is_playing(&self, pb: BackendPlaybackId) -> bool;

    fn set_voice_volume(&mut self, pb: BackendPlaybackId, v: f32, tween: Option<f32>);
    fn set_voice_pitch (&mut self, pb: BackendPlaybackId, p: f32, tween: Option<f32>);
    fn set_voice_position(&mut self, pb: BackendPlaybackId, pos: Vec2);

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
```

Implement a new backend when targeting a platform that kira doesn't cover, or when swapping in an alternative decoder or output driver. For tests, use `StubBackend` (see [Testing pattern](#testing-pattern)).

## KiraBackend

Default cross-platform backend. Built with kira 0.12 using the `cpal` audio output and the `symphonia` decoder. Volumes are converted from the engine's amplitude scalar to kira's `Decibels` via `amp_to_db(v) = 20 * log10(v)`, clamping very small values to `Decibels::SILENCE` so muted sounds actually go quiet.

**Defaults:**
- Three tracks (master/music/sfx) created during `AudioSystem::with_backend`. Sub-tracks route directly to the main track (kira 0.12 behavior).
- Tweens default to `Tween::default()` (linear, immediate) when `tween: None`; when `Some(secs)` a linear tween of `secs` seconds is used.

**Supported codecs (default features):** OGG Vorbis (`codec-ogg`), WAV (`codec-wav`). Optional: `codec-mp3`, `codec-flac`.

**V1 limitations:**
- **Static spatial attenuation.** `play_spatial` computes volume from the initial listener–sound distance at play time using the chosen rolloff, then sets it on the voice. It does **not** update volume when the listener or the voice moves afterward — call `AudioSystem::set_volume` or re-trigger playback if you need dynamic attenuation.
- **`set_voice_position` is a no-op** on this backend. Spatial voices keep whatever volume they were born with. `set_sound_position` still exists in the scripting API for forward-compat, but it does not currently change audible output under the kira backend.

## Platform notes

### Web

Autoplay is gated by a user gesture. `unison-web` calls `AudioSystem::unarm_for_web()` during construction and attaches a first-gesture listener (click/touch/keydown) that calls `arm()`. Between construction and the first gesture, non-spatial `play`, `play_music`, `set_master_volume`, and `set_bus_volume` calls are queued and replayed on arm (cap: 64 calls; overflow dropped with a warning). Spatial calls are dropped silently pre-arm — they're tied to gameplay frames.

### iOS

`unison-ios` configures `AVAudioSession` with category `.ambient` (game-style: mixes with other audio, respects the silent switch, does not interrupt music apps). An `AVAudioSession.interruptionNotification` observer bridges to engine hooks via `@_silgen_name`:

- `engine_audio_suspend(state_ptr)` → `AudioSystem::suspend`
- `engine_audio_resume_system(state_ptr)` → `AudioSystem::resume_system`

The bridge uses `game_engine_ptr(GameState*) -> *mut Engine` to reach the audio system from the Swift layer.

### Android

`unison-android` requests **AudioFocus** (API 26+, `USAGE_GAME` + `CONTENT_TYPE_SONIFICATION`). The app's `onPause` / `onResume` lifecycle callbacks JNI-bridge into the engine on `com.unison2d.UnisonNative` to call `suspend` / `resume_system`. `cpal`'s aaudio output driver requires Android API 26+, so this is the platform's `minSdk`.

## Testing pattern

`unison-audio` ships a `test-helpers` feature that exposes `AudioSystem::backend_for_test()` — a downcast-to-`&StubBackend` accessor for integration tests. Enable it in your dev-dependency entry:

```toml
[dev-dependencies]
unison-audio = { path = "…", features = ["test-helpers"] }
```

Then build an `AudioSystem` around a `StubBackend` and assert on the recorded `StubEvent` stream:

```rust
use unison_audio::{AudioSystem, MusicOptions, PlayParams, StubBackend, StubEvent};

let mut sys = AudioSystem::with_backend(Box::new(StubBackend::new()));
let snd = sys.load(&[0u8; 8]).unwrap();
sys.play(snd, PlayParams::with_bus(sys.sfx_bus())).unwrap();

let backend = sys.backend_for_test();
assert!(backend.events.iter().any(|e| matches!(e, StubEvent::Play { .. })));
```

`StubBackend` records 17 event variants (`LoadSound`, `UnloadSound`, `Play`, `PlaySpatial`, `Stop`, `Pause`, `Resume`, `SetVoiceVolume`, `SetVoicePitch`, `SetVoicePosition`, `SetMasterVolume`, `SetBusVolume`, `CreateBus`, `SetListener`, `Suspend`, `ResumeSystem`, `Tick`). It also exposes `alive: HashSet<u32>` so tests can control what `is_playing` reports.

## Dependencies

- `unison-core` — `Vec2`
- `thiserror` — `AudioError` derive
- `kira` (optional, default-on) — the cross-platform backend, pulling in `cpal` + `symphonia`
